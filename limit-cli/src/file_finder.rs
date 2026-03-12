use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use walkdir::WalkDir;

/// Represents a matched file with fuzzy score
#[derive(Debug, Clone)]
pub struct FileMatch {
    /// Relative path from working directory
    pub path: PathBuf,
    /// Whether it's a directory
    pub is_dir: bool,
    /// Fuzzy match score (higher is better)
    pub score: i64,
}

/// File finder with fuzzy matching and caching
pub struct FileFinder {
    /// Working directory for file scanning
    working_dir: PathBuf,
    /// Ignore patterns (from .gitignore, etc.)
    ignore_patterns: Vec<glob::Pattern>,
    /// Cached file list
    cached_files: Vec<PathBuf>,
    /// Last scan timestamp
    last_scan: Option<Instant>,
    /// Cache time-to-live
    cache_ttl: Duration,
    /// Maximum scan depth
    max_depth: usize,
}

impl FileFinder {
    /// Create a new FileFinder for the given working directory
    pub fn new(working_dir: PathBuf) -> Self {
        let ignore_patterns = Self::load_ignore_patterns(&working_dir);
        
        Self {
            working_dir,
            ignore_patterns,
            cached_files: Vec::new(),
            last_scan: None,
            cache_ttl: Duration::from_secs(5),
            max_depth: 10,
        }
    }

    /// Load ignore patterns from .gitignore, .ignore, etc.
    fn load_ignore_patterns(dir: &Path) -> Vec<glob::Pattern> {
        let mut patterns = Vec::new();
        
        // Default ignore patterns
        let default_ignores = vec![
            "target",
            ".git",
            "node_modules",
            "*.lock",
            ".DS_Store",
            "dist",
            "build",
        ];
        
        for pattern in default_ignores {
            if let Ok(p) = glob::Pattern::new(pattern) {
                patterns.push(p);
            }
        }
        
        // Load .gitignore
        if let Ok(content) = std::fs::read_to_string(dir.join(".gitignore")) {
            for line in content.lines() {
                let line = line.trim();
                if !line.is_empty() && !line.starts_with('#') {
                    if let Ok(pattern) = glob::Pattern::new(line) {
                        patterns.push(pattern);
                    }
                }
            }
        }
        
        patterns
    }

    /// Check if a path should be ignored
    fn should_ignore(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        
        for pattern in &self.ignore_patterns {
            if pattern.matches(&path_str) {
                return true;
            }
        }
        
        false
    }

    /// Scan directory and return all files (relative paths)
    pub fn scan_files(&mut self) -> &Vec<PathBuf> {
        // Check cache
        if let Some(last_scan) = self.last_scan {
            if last_scan.elapsed() < self.cache_ttl {
                return &self.cached_files;
            }
        }
        
        // Rescan
        self.cached_files.clear();
        
        for entry in WalkDir::new(&self.working_dir)
            .max_depth(self.max_depth)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            // Get relative path
            if let Ok(rel_path) = path.strip_prefix(&self.working_dir) {
                // Skip ignored patterns
                if self.should_ignore(rel_path) {
                    continue;
                }
                
                // Skip hidden files/dirs (except .gitignore is already handled)
                let path_str = rel_path.to_string_lossy();
                if path_str.starts_with('.') && !path_str.contains('/') {
                    continue;
                }
                
                self.cached_files.push(rel_path.to_path_buf());
            }
        }
        
        // Sort for consistent ordering
        self.cached_files.sort();
        
        self.last_scan = Some(Instant::now());
        &self.cached_files
    }

    /// Filter files by query using fuzzy matching
    pub fn filter_files(&self, files: &[PathBuf], query: &str) -> Vec<FileMatch> {
        if query.is_empty() {
            // Return first 20 files if no query
            return files
                .iter()
                .take(20)
                .map(|p| FileMatch {
                    path: p.clone(),
                    is_dir: p.to_string_lossy().ends_with('/'),
                    score: 0,
                })
                .collect();
        }

        let query_lower = query.to_lowercase();
        let mut matches: Vec<FileMatch> = Vec::new();

        for path in files {
            let path_str = path.to_string_lossy().to_string();
            let path_lower = path_str.to_lowercase();
            
            // Use simple substring matching for now (frizbee will be added later)
            if path_lower.contains(&query_lower) {
                // Calculate simple score based on position
                let score = if path_lower.starts_with(&query_lower) {
                    1000 - path_str.len() as i64  // Prefer shorter paths at start
                } else {
                    500 - (path_lower.find(&query_lower).unwrap_or(999) as i64)
                };
                
                matches.push(FileMatch {
                    path: path.clone(),
                    is_dir: path_str.ends_with('/'),
                    score,
                });
            }
        }

        // Sort by score (descending)
        matches.sort_by(|a, b| b.score.cmp(&a.score));
        
        // Limit to 20 results
        matches.truncate(20);
        matches
    }

    /// Get the working directory
    pub fn working_dir(&self) -> &Path {
        &self.working_dir
    }

    /// Force refresh the cache
    pub fn refresh_cache(&mut self) {
        self.last_scan = None;
        self.scan_files();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_finder_basic() {
        let dir = std::env::current_dir().unwrap();
        let mut finder = FileFinder::new(dir);
        let files = finder.scan_files();
        
        // Should find Cargo.toml in current directory
        assert!(files.iter().any(|p| p.to_string_lossy() == "Cargo.toml"));
    }

    #[test]
    fn test_filter_files() {
        let dir = std::env::current_dir().unwrap();
        let mut finder = FileFinder::new(dir);
        let files = finder.scan_files().clone();
        
        let matches = finder.filter_files(&files, "Cargo");
        assert!(!matches.is_empty());
        assert!(matches.iter().any(|m| m.path.to_string_lossy() == "Cargo.toml"));
    }

    #[test]
    fn test_cache_ttl() {
        let dir = std::env::current_dir().unwrap();
        let mut finder = FileFinder::new(dir);
        finder.cache_ttl = std::time::Duration::from_millis(50);
        
        // First scan
        let files1 = finder.scan_files().clone();
        
        // Should use cache immediately
        let files2 = finder.scan_files().clone();
        assert_eq!(files1.len(), files2.len());
        
        // Wait for cache to expire
        std::thread::sleep(std::time::Duration::from_millis(60));
        
        // Should have rescanned
        let files3 = finder.scan_files().clone();
        assert!(!files3.is_empty());
    }
}
