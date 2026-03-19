//! File utilities

use std::path::Path;

pub fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

pub fn should_skip(path: &Path) -> bool {
    let skip_dirs = ["node_modules", "target", "venv", "__pycache__", ".git", "dist", "build"];
    
    path.components().any(|c| {
        c.as_os_str().to_string_lossy().starts_with('.') ||
        skip_dirs.contains(&c.as_os_str().to_string_lossy().as_ref())
    })
}
