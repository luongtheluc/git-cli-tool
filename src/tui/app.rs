use std::collections::HashMap;

use ratatui::style::Color;

use crate::git_runner;
use crate::repo_scanner::RepoInfo;

/// Which panel currently has keyboard focus
#[derive(Debug, Clone, PartialEq)]
pub enum Panel {
    Sidebar,
    Main,
}

/// Normal navigation vs text input mode
#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Input,
}

/// What the current text input will be used for
#[derive(Debug, Clone, PartialEq)]
pub enum InputPurpose {
    CommitMessage,
    BranchName,
    None,
}

/// A single file entry from `git status --porcelain` (XY path)
#[derive(Debug, Clone)]
pub struct StatusEntry {
    /// Two-char XY status code, e.g. " M", "??", "A "
    pub code: String,
    pub path: String,
}

impl StatusEntry {
    /// Map XY code to a display color (lazygit conventions)
    pub fn status_color(&self) -> Color {
        let x = self.code.as_bytes().first().copied().unwrap_or(b' ');
        let y = self.code.as_bytes().get(1).copied().unwrap_or(b' ');
        if x == b'?' && y == b'?' {
            Color::Red // untracked
        } else if x != b' ' {
            Color::Green // staged (index changed)
        } else {
            Color::Yellow // unstaged (working tree changed)
        }
    }

    /// Human-readable label for the file status.
    pub fn label(&self) -> &'static str {
        match self.code.as_str() {
            "??" => "untracked",
            "A " | "AM" => "added    ",
            "M " | "MM" => "staged   ",
            " M" => "modified ",
            "D " => "deleted  ",
            " D" => "deleted  ",
            "R " | "RM" => "renamed  ",
            _ => "changed  ",
        }
    }
}

/// Live progress state for a running batch operation
pub struct BatchProgress {
    pub total: usize,
    pub completed: usize,
    /// Display name like "Pulling", "Pushing", etc.
    pub op_name: String,
    #[allow(dead_code)]
    pub results: Vec<BatchResult>,
}

/// Result of a single repo operation within a batch
pub struct BatchResult {
    pub repo_name: String,
    pub success: bool,
    pub message: String,
}

/// Core TUI application state — owns all mutable data
pub struct App {
    pub repos: Vec<RepoInfo>,
    /// Index into `repos` of the currently highlighted row
    pub selected: usize,
    /// Cached git status per repo index (populated on demand)
    pub status_cache: HashMap<usize, Vec<StatusEntry>>,
    /// Which panel has focus
    pub focused_panel: Panel,
    /// Set to true to exit the event loop
    pub should_quit: bool,
    /// Transient status/hint shown in the footer
    pub message: Option<String>,
    /// Per-repo checkbox state for multi-select
    pub checked: Vec<bool>,
    /// Current input mode (Normal or Input)
    pub input_mode: InputMode,
    /// Text buffer for input mode
    pub input_buffer: String,
    /// What the input text will be used for
    pub input_purpose: InputPurpose,
    /// Live progress for a running batch operation (None = idle)
    pub batch_progress: Option<BatchProgress>,
}

impl App {
    pub fn new(repos: Vec<RepoInfo>) -> Self {
        let checked = vec![false; repos.len()];
        App {
            repos,
            selected: 0,
            status_cache: HashMap::new(),
            focused_panel: Panel::Sidebar,
            should_quit: false,
            message: None,
            checked,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            input_purpose: InputPurpose::None,
            batch_progress: None,
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.repos.len() {
            self.selected += 1;
        }
    }

    /// Toggle checkbox on the currently highlighted repo
    pub fn toggle_selected(&mut self) {
        if let Some(v) = self.checked.get_mut(self.selected) {
            *v = !*v;
        }
    }

    /// Toggle all checkboxes: if all checked → uncheck all, else check all
    pub fn toggle_all(&mut self) {
        let all_checked = self.checked.iter().all(|&c| c);
        self.checked.iter_mut().for_each(|c| *c = !all_checked);
    }

    /// Returns indices of checked repos, or just the current repo if none checked
    pub fn selected_indices(&self) -> Vec<usize> {
        let indices: Vec<usize> = self
            .checked
            .iter()
            .enumerate()
            .filter(|(_, &c)| c)
            .map(|(i, _)| i)
            .collect();
        if indices.is_empty() {
            vec![self.selected]
        } else {
            indices
        }
    }

    /// Enter text input mode for a specific purpose
    pub fn enter_input(&mut self, purpose: InputPurpose) {
        self.input_mode = InputMode::Input;
        self.input_purpose = purpose;
        self.input_buffer.clear();
    }

    /// Load status only if not already cached (lazy). Use `refresh_status()` to force.
    pub fn ensure_status_loaded(&mut self) {
        if !self.status_cache.contains_key(&self.selected) {
            self.refresh_status();
        } else if let Some(repo) = self.repos.get(self.selected) {
            let count = self.status_cache.get(&self.selected).map(|v| v.len()).unwrap_or(0);
            self.message = Some(if count == 0 {
                format!("{} is clean", repo.name)
            } else {
                format!("{} — {} changed file(s)", repo.name, count)
            });
        }
    }

    /// Force-reload git status for the selected repo (even if cached).
    /// Reuses git_runner::status_files() to avoid duplicating git status parsing.
    pub fn refresh_status(&mut self) {
        if let Some(repo) = self.repos.get(self.selected) {
            match git_runner::status_files(&repo.path) {
                Ok(files) => {
                    let entries: Vec<StatusEntry> = files
                        .into_iter()
                        .map(|(code, path)| StatusEntry { code, path })
                        .collect();
                    let count = entries.len();
                    self.status_cache.insert(self.selected, entries);
                    self.message = Some(if count == 0 {
                        format!("{} is clean", repo.name)
                    } else {
                        format!("{} — {} changed file(s)", repo.name, count)
                    });
                }
                Err(e) => {
                    self.status_cache.insert(self.selected, vec![]);
                    self.message = Some(format!("git error: {}", e));
                }
            }
        }
    }

    /// Invalidate status cache for given repo indices (forces re-fetch next time)
    pub fn invalidate_status(&mut self, indices: &[usize]) {
        for i in indices {
            self.status_cache.remove(i);
        }
    }

    /// Status entries for the currently selected repo (empty slice if uncached)
    pub fn current_status(&self) -> &[StatusEntry] {
        self.status_cache
            .get(&self.selected)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}
