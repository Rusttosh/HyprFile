use anyhow::Result;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

/// Search criteria.
#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub root: PathBuf,
    pub pattern: String,
    pub case_sensitive: bool,
    pub fuzzy: bool,
    pub content_search: bool,
}

/// Single search result.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: PathBuf,
    pub score: Option<f64>,
}

/// Trait for search functionality.
pub trait SearchEngine: Send + Sync {
    fn search(&self, query: SearchQuery) -> impl Future<Output = Result<Vec<SearchResult>>> + Send;
    fn cancel(&self);
}

/// Simple fuzzy matcher.
fn fuzzy_match(pattern: &str, text: &str) -> Option<f64> {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();
    let mut p_idx = 0;
    let mut t_idx = 0;
    let mut consecutive = 0;
    let mut matched_positions = Vec::new();

    while p_idx < pattern_chars.len() && t_idx < text_chars.len() {
        if pattern_chars[p_idx].to_lowercase().next() == text_chars[t_idx].to_lowercase().next() {
            matched_positions.push(t_idx);
            p_idx += 1;
            consecutive += 1;
        } else {
            consecutive = 0;
        }
        t_idx += 1;
    }

    if p_idx < pattern_chars.len() {
        return None;
    }

    // Score: higher is better
    // Base score from coverage
    let coverage = pattern.len() as f64 / text.len().max(1) as f64;
    // Bonus for consecutive matches
    let consec_bonus = consecutive as f64 / pattern.len().max(1) as f64;
    // Bonus for matching at start
    let start_bonus = if matched_positions.first() == Some(&0) { 0.2 } else { 0.0 };
    let score = coverage + consec_bonus + start_bonus;
    Some(score)
}

/// Local async search engine.
pub struct LocalSearchEngine {
    token: Arc<Mutex<CancellationToken>>,
}

impl LocalSearchEngine {
    pub fn new() -> Self {
        Self {
            token: Arc::new(Mutex::new(CancellationToken::new())),
        }
    }

    async fn search_inner(&self, query: SearchQuery) -> Result<Vec<SearchResult>> {
        let pattern = if query.case_sensitive {
            query.pattern.clone()
        } else {
            query.pattern.to_lowercase()
        };
        let root = &query.root;
        let mut results = Vec::new();

        let mut dirs = vec![root.clone()];
        while let Some(current) = dirs.pop() {
            let token = self.token.lock().await;
            if token.is_cancelled() {
                debug!("search cancelled");
                return Ok(results);
            }
            drop(token);

            let mut read_dir = match tokio::fs::read_dir(&current).await {
                Ok(rd) => rd,
                Err(e) => {
                    warn!("cannot read {}: {}", current.display(), e);
                    continue;
                }
            };

            while let Some(entry) = read_dir.next_entry().await? {
                let name = entry.file_name().to_string_lossy().into_owned();
                let path = entry.path();

                let check_name = if query.case_sensitive {
                    name.clone()
                } else {
                    name.to_lowercase()
                };

                let matched = if query.fuzzy {
                    fuzzy_match(&pattern, &check_name).is_some()
                } else if pattern.starts_with('*') && pattern.ends_with('*') {
                    check_name.contains(&pattern.trim_matches('*'))
                } else if pattern.starts_with('*') {
                    check_name.ends_with(&pattern[1..])
                } else if pattern.ends_with('*') {
                    check_name.starts_with(&pattern[..pattern.len() - 1])
                } else if query.case_sensitive {
                    check_name == pattern
                } else {
                    check_name.contains(&pattern)
                };

                if matched {
                    let score = if query.fuzzy {
                        fuzzy_match(&pattern, &check_name)
                    } else {
                        None
                    };
                    results.push(SearchResult { path: path.clone(), score });
                }

                if path.is_dir() {
                    dirs.push(path);
                }
            }
        }

        // Sort fuzzy results by score descending
        if query.fuzzy {
            results.sort_by(|a, b| {
                let sa = b.score.unwrap_or(0.0).partial_cmp(&a.score.unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal);
                sa
            });
        }

        Ok(results)
    }
}

impl Default for LocalSearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchEngine for LocalSearchEngine {
    async fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>> {
        // Create a fresh cancellation token for this search
        let token = CancellationToken::new();
        {
            let mut guard = self.token.lock().await;
            guard.cancel();
            *guard = token.clone();
        }

        tokio::select! {
            _ = token.cancelled() => {
                Ok(Vec::new())
            }
            res = self.search_inner(query) => res,
        }
    }

    fn cancel(&self) {
        let token = self.token.clone();
        tokio::spawn(async move {
            let guard = token.lock().await;
            guard.cancel();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn engine() -> LocalSearchEngine {
        LocalSearchEngine::new()
    }

    #[tokio::test]
    async fn test_search_contains() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        tokio::fs::write(root.join("hello.txt"), "").await.unwrap();
        tokio::fs::write(root.join("world.rs"), "").await.unwrap();
        tokio::fs::write(root.join("foo.md"), "").await.unwrap();

        let query = SearchQuery {
            root: root.to_path_buf(),
            pattern: "hello".to_string(),
            case_sensitive: false,
            fuzzy: false,
            content_search: false,
        };
        let res = engine().search(query).await.unwrap();
        assert_eq!(res.len(), 1);
        assert!(res[0].path.ends_with("hello.txt"));
    }

    #[tokio::test]
    async fn test_search_exact() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        tokio::fs::write(root.join("exact"), "").await.unwrap();
        tokio::fs::write(root.join("exact_extra"), "").await.unwrap();

        let query = SearchQuery {
            root: root.to_path_buf(),
            pattern: "exact".to_string(),
            case_sensitive: false,
            fuzzy: false,
            content_search: false,
        };
        let res = engine().search(query).await.unwrap();
        assert_eq!(res.len(), 2);
    }

    #[tokio::test]
    async fn test_fuzzy_search() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        tokio::fs::write(root.join("hello_world.rs"), "").await.unwrap();
        tokio::fs::write(root.join("hello.md"), "").await.unwrap();

        let query = SearchQuery {
            root: root.to_path_buf(),
            pattern: "hw".to_string(),
            case_sensitive: false,
            fuzzy: true,
            content_search: false,
        };
        let res = engine().search(query).await.unwrap();
        assert!(!res.is_empty());
        assert!(res.iter().any(|r| r.path.ends_with("hello_world.rs")));
    }

    #[tokio::test]
    async fn test_cancel_search() {
        let engine = engine();
        let query = SearchQuery {
            root: PathBuf::from("/tmp"),
            pattern: "test".to_string(),
            case_sensitive: false,
            fuzzy: false,
            content_search: false,
        };
        engine.cancel();
        let res = engine.search(query).await.unwrap();
        // Cancelled before or during search; empty result is acceptable
        assert!(res.is_empty());
    }
}
