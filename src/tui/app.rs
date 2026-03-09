use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Instant;

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
    GitCommand,
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

/// Live progress state for a running batch or single operation
#[derive(Clone)]
pub enum OperationProgress {
    /// Batch operation with total count and completion tracking
    Batch {
        total: usize,
        completed: usize,
        op_name: String,
        #[allow(dead_code)]
        results: Vec<BatchResult>,
    },
    /// Single operation (e.g., refresh status, commit) with name only
    Single {
        op_name: String,
    },
}

impl OperationProgress {
    /// Get the operation display name
    pub fn op_name(&self) -> &str {
        match self {
            OperationProgress::Batch { op_name, .. } => op_name,
            OperationProgress::Single { op_name } => op_name,
        }
    }

    /// Get completion percentage (0.0-1.0) for batch ops, 0.0 for single
    pub fn progress_pct(&self) -> f64 {
        match self {
            OperationProgress::Batch {
                completed, total, ..
            } => {
                if *total > 0 {
                    *completed as f64 / *total as f64
                } else {
                    0.0
                }
            }
            OperationProgress::Single { .. } => 0.0, // Indeterminate for single ops
        }
    }

    /// Get current progress (x/total) for batch ops, returns None for single
    pub fn batch_progress(&self) -> Option<(usize, usize)> {
        match self {
            OperationProgress::Batch {
                completed, total, ..
            } => Some((*completed, *total)),
            OperationProgress::Single { .. } => None,
        }
    }
}

/// Result of a single repo operation within a batch
#[derive(Clone)]
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
    /// Live progress for a running batch or single operation (None = idle)
    pub operation_progress: Option<OperationProgress>,
    /// Channel receiver for incremental batch operation results
    pub batch_receiver: Option<mpsc::Receiver<BatchResult>>,
    /// Animation start time for smooth spinner rotation
    pub animation_start: Option<Instant>,
    /// Scroll offset for batch result lines shown in modal
    pub batch_result_scroll: usize,
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
            operation_progress: None,
            batch_receiver: None,
            animation_start: None,
            batch_result_scroll: 0,
        }
    }

    /// Reset scroll position for a new/closed batch modal
    pub fn reset_batch_result_scroll(&mut self) {
        self.batch_result_scroll = 0;
    }

    /// Scroll result list down by `delta` lines
    pub fn scroll_batch_results_down(&mut self, delta: usize) {
        self.batch_result_scroll = self.batch_result_scroll.saturating_add(delta);
    }

    /// Scroll result list up by `delta` lines
    pub fn scroll_batch_results_up(&mut self, delta: usize) {
        self.batch_result_scroll = self.batch_result_scroll.saturating_sub(delta);
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
            // Show progress modal during git operation
            self.operation_progress = Some(OperationProgress::Single {
                op_name: format!("Refreshing {}", repo.name),
            });
            self.animation_start = Some(Instant::now());

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

            // Clear progress modal after operation completes
            self.operation_progress = None;
            self.animation_start = None;
        }
    }

    /// Invalidate status cache for given repo indices (forces re-fetch next time)
    pub fn invalidate_status(&mut self, indices: &[usize]) {
        for i in indices {
            self.status_cache.remove(i);
        }
    }

    /// Poll batch operation receiver for new results (non-blocking)
    /// Returns true if batch is complete, false if still in progress
    pub fn poll_batch_results(&mut self) -> bool {
        let Some(rx) = self.batch_receiver.as_ref() else {
            return true; // No operation in progress
        };

        // Collect all available results without blocking
        let mut results_this_tick = Vec::new();
        let mut disconnected = false;
        loop {
            match rx.try_recv() {
                Ok(result) => results_this_tick.push(result),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }

        // Update progress with new results
        if let Some(OperationProgress::Batch {
            ref mut completed,
            ref mut results,
            total,
            ..
        }) = self.operation_progress
        {
            *completed += results_this_tick.len();
            results.extend(results_this_tick);

            if disconnected && *completed < total {
                let missing = total.saturating_sub(*completed);
                results.push(BatchResult {
                    repo_name: "batch".to_string(),
                    success: false,
                    message: format!("Channel closed early: {} result(s) missing", missing),
                });
                *completed = total;
            }

            // Check if batch is complete
            if *completed >= total {
                // Batch complete - cleanup receiver and return true
                self.batch_receiver = None;
                return true;
            }
        }

        false // Still in progress
    }

    /// Status entries for the currently selected repo (empty slice if uncached)
    pub fn current_status(&self) -> &[StatusEntry] {
        self.status_cache
            .get(&self.selected)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}
