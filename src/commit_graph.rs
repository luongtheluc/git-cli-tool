#![allow(dead_code)]

use anyhow::Result;

use crate::tui::app::CommitNode;

pub const DEFAULT_PAGE_SIZE: usize = 50;

#[derive(Debug, Clone)]
pub struct CommitGraph {
    pub nodes: Vec<CommitNode>,
    pub graph_lines: Vec<String>,
}

pub fn parse_git_log_graph(output: &str) -> Result<CommitGraph> {
    let mut nodes = Vec::new();
    let mut graph_lines = Vec::new();

    for raw_line in output.lines() {
        if raw_line.trim().is_empty() {
            continue;
        }

        let graph_line = raw_line.to_string();
        graph_lines.push(graph_line.clone());

        let hash = extract_hash(raw_line).unwrap_or_default();
        let branch_decorators = extract_decorators(raw_line);
        let subject = extract_subject(raw_line, &hash);

        nodes.push(CommitNode {
            hash,
            subject,
            author: String::new(),
            graph_line,
            branch_decorators,
            timestamp: String::new(),
        });
    }

    Ok(CommitGraph { nodes, graph_lines })
}

pub fn get_page(graph: &CommitGraph, page: usize, page_size: usize) -> Vec<(String, CommitNode)> {
    let size = if page_size == 0 {
        DEFAULT_PAGE_SIZE
    } else {
        page_size
    };

    let start = page.saturating_mul(size);
    let end = (start + size).min(graph.nodes.len());

    if start >= graph.nodes.len() {
        return Vec::new();
    }

    graph.nodes[start..end]
        .iter()
        .cloned()
        .map(|node| (node.graph_line.clone(), node))
        .collect()
}

fn extract_hash(line: &str) -> Option<String> {
    for token in line.split_whitespace() {
        if is_short_hash(token) {
            return Some(token.to_string());
        }
    }
    None
}

fn is_short_hash(token: &str) -> bool {
    let len = token.len();
    (7..=40).contains(&len) && token.chars().all(|c| c.is_ascii_hexdigit())
}

fn extract_decorators(line: &str) -> Vec<String> {
    let Some(hash) = extract_hash(line) else {
        return Vec::new();
    };

    let Some(after_hash) = line.split_once(&hash).map(|(_, tail)| tail.trim_start()) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    if after_hash.starts_with('(') {
        if let Some(close_idx) = after_hash.find(')') {
            let inner = &after_hash[1..close_idx];
            out.extend(
                inner
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(ToString::to_string),
            );
        }
    }
    out
}

fn extract_subject(line: &str, hash: &str) -> String {
    if hash.is_empty() {
        return line.trim().to_string();
    }

    if let Some(hash_pos) = line.find(hash) {
        let after_hash = &line[hash_pos + hash.len()..];
        let trimmed = after_hash.trim_start();

        if trimmed.starts_with('(') {
            if let Some(close_idx) = trimmed.find(')') {
                return trimmed[close_idx + 1..].trim().to_string();
            }
        }

        return trimmed.to_string();
    }

    line.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_log_graph_basic() {
        let output = "* abc1234 (HEAD -> main) Initial commit\n* def5678 Second commit";
        let graph = parse_git_log_graph(output).unwrap();
        
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.graph_lines.len(), 2);
        assert_eq!(graph.nodes[0].hash, "abc1234");
        assert_eq!(graph.nodes[0].subject, "Initial commit");
        assert!(graph.nodes[0].branch_decorators.contains(&"HEAD -> main".to_string()));
    }

    #[test]
    fn test_parse_git_log_graph_with_merge() {
        let output = r#"*   123abcd Merge branch 'feature'
|\  
| * 456efgh Feature commit
* | 789ijkl Main commit
|/  
* abc1234 Initial commit"#;
        
        let graph = parse_git_log_graph(output).unwrap();
        assert!(graph.nodes.len() >= 4, "Should parse all commits including merge");
        assert!(graph.nodes[0].subject.contains("Merge"), "First commit should be merge");
    }

    #[test]
    fn test_extract_hash_finds_short_hash() {
        let line = "* abc1234 commit message";
        let hash = extract_hash(line);
        assert_eq!(hash, Some("abc1234".to_string()));
    }

    #[test]
    fn test_extract_decorators_parses_branch_refs() {
        let line = "* abc1234 (HEAD -> main, origin/main, tag: v1.0) commit";
        let decorators = extract_decorators(line);
        assert_eq!(decorators.len(), 3);
        assert!(decorators.contains(&"HEAD -> main".to_string()));
        assert!(decorators.contains(&"origin/main".to_string()));
        assert!(decorators.contains(&"tag: v1.0".to_string()));
    }

    #[test]
    fn test_extract_subject_handles_parentheses_in_message() {
        let line = "* abc1234 fix parser (retry logic)";
        let subject = extract_subject(line, "abc1234");
        assert_eq!(subject, "fix parser (retry logic)");
    }

    #[test]
    fn test_extract_subject_strips_decorators() {
        let line = "* abc1234 (HEAD -> main) Initial commit";
        let subject = extract_subject(line, "abc1234");
        assert_eq!(subject, "Initial commit");
    }

    #[test]
    fn test_get_page_returns_correct_slice() {
        let nodes = vec![
            CommitNode {
                hash: "a".into(),
                subject: "1".into(),
                author: "".into(),
                graph_line: "* a 1".into(),
                branch_decorators: vec![],
                timestamp: "".into(),
            },
            CommitNode {
                hash: "b".into(),
                subject: "2".into(),
                author: "".into(),
                graph_line: "* b 2".into(),
                branch_decorators: vec![],
                timestamp: "".into(),
            },
            CommitNode {
                hash: "c".into(),
                subject: "3".into(),
                author: "".into(),
                graph_line: "* c 3".into(),
                branch_decorators: vec![],
                timestamp: "".into(),
            },
        ];
        
        let graph = CommitGraph {
            nodes,
            graph_lines: vec!["* a 1".into(), "* b 2".into(), "* c 3".into()],
        };
        
        let page = get_page(&graph, 0, 2);
        assert_eq!(page.len(), 2);
        assert_eq!(page[0].1.hash, "a");
        assert_eq!(page[1].1.hash, "b");
        
        let page2 = get_page(&graph, 1, 2);
        assert_eq!(page2.len(), 1);
        assert_eq!(page2[0].1.hash, "c");
    }

    #[test]
    fn test_get_page_handles_empty_graph() {
        let graph = CommitGraph {
            nodes: vec![],
            graph_lines: vec![],
        };
        
        let page = get_page(&graph, 0, 10);
        assert_eq!(page.len(), 0);
    }
}
