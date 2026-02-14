//! `.changesignore` file support for filtering shadow git checkpoint diffs.
//!
//! Reads a `.changesignore` file from the project root (CWD) and returns
//! the patterns as a list of strings suitable for git pathspec exclusions.
//!
//! File format:
//! - One pattern per line
//! - Lines starting with `#` are comments
//! - Blank lines are ignored
//! - Patterns are used as git `:(exclude)<pattern>` pathspecs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Default patterns to use when no `.changesignore` file exists.
/// These are git pathspec patterns — must match the paths as stored in the
/// shadow git checkpoint repo (which mirrors the project workspace structure).
const DEFAULT_PATTERNS: &[&str] = &[
    "src-tauri/target",
    "src-tauri/gen/schemas",
    "node_modules",
    "dist",
    "build",
    ".output",
    ".vscode",
    ".idea",
    ".DS_Store",
    "Thumbs.db",
    "*.lock",
    "package-lock.json",
    "Cargo.lock",
];

/// Response type for the GET /changes/ignore endpoint
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangesIgnoreResponse {
    /// The parsed exclude patterns (comments and blanks stripped)
    pub patterns: Vec<String>,
    /// The raw file content (for editing in the UI)
    pub raw_content: String,
    /// Where the patterns were loaded from
    pub source: String,
    /// Absolute path to the .changesignore file
    pub file_path: String,
}

/// Request type for the PUT /changes/ignore endpoint
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangesIgnoreUpdateRequest {
    /// The raw file content to write (preserving comments, formatting)
    pub raw_content: String,
}

/// Get the path to the `.changesignore` file.
///
/// Searches in order:
/// 1. Current working directory (project root when running from root)
/// 2. Parent of CWD (when Tauri dev runs from `src-tauri/`)
///
/// Returns the first path found, or falls back to CWD-based path for creation.
pub fn changesignore_path() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Check CWD first (project root)
    let cwd_path = cwd.join(".changesignore");
    if cwd_path.exists() {
        return cwd_path;
    }

    // Check parent dir (when running from src-tauri/)
    let parent_path = cwd.join("..").join(".changesignore");
    if parent_path.exists() {
        // Canonicalize to get a clean absolute path
        return parent_path.canonicalize().unwrap_or(parent_path);
    }

    // Not found — return the parent dir path for creation
    // (Tauri typically runs from src-tauri/, so parent is project root)
    if cwd.ends_with("src-tauri") || cwd.to_string_lossy().contains("src-tauri") {
        let parent = cwd.join("..").join(".changesignore");
        parent.canonicalize().unwrap_or(parent)
    } else {
        cwd_path
    }
}

/// Parse patterns from raw `.changesignore` file content.
/// Strips comments (lines starting with `#`) and blank lines.
pub fn parse_patterns(content: &str) -> Vec<String> {
    content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect()
}

/// Load exclude patterns from `.changesignore` file.
///
/// If the file exists, parses it and returns the patterns.
/// If the file doesn't exist, returns the built-in defaults.
///
/// This is the primary function used by diff handlers to auto-load
/// exclude patterns before computing diffs.
pub fn load_patterns() -> Vec<String> {
    let path = changesignore_path();

    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let patterns = parse_patterns(&content);
                log::info!(
                    ".changesignore: loaded {} patterns from {:?}: {:?}",
                    patterns.len(),
                    path,
                    patterns
                );
                patterns
            }
            Err(e) => {
                log::warn!(
                    "Failed to read .changesignore at {:?}: {}. Using defaults.",
                    path, e
                );
                default_patterns()
            }
        }
    } else {
        log::debug!(
            ".changesignore not found at {:?}. Using {} built-in defaults.",
            path,
            DEFAULT_PATTERNS.len()
        );
        default_patterns()
    }
}

/// Load the full `.changesignore` response (patterns + raw content + metadata).
/// Used by the GET /changes/ignore endpoint.
pub fn load_full() -> ChangesIgnoreResponse {
    let path = changesignore_path();
    let file_path = path.to_string_lossy().to_string();

    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let patterns = parse_patterns(&content);
                ChangesIgnoreResponse {
                    patterns,
                    raw_content: content,
                    source: "file".to_string(),
                    file_path,
                }
            }
            Err(e) => {
                log::warn!("Failed to read .changesignore: {}", e);
                ChangesIgnoreResponse {
                    patterns: default_patterns(),
                    raw_content: default_file_content(),
                    source: "defaults (file read error)".to_string(),
                    file_path,
                }
            }
        }
    } else {
        ChangesIgnoreResponse {
            patterns: default_patterns(),
            raw_content: default_file_content(),
            source: "defaults (no file)".to_string(),
            file_path,
        }
    }
}

/// Save raw content to the `.changesignore` file.
/// Used by the PUT /changes/ignore endpoint.
pub fn save_content(raw_content: &str) -> Result<ChangesIgnoreResponse, String> {
    let path = changesignore_path();
    let file_path = path.to_string_lossy().to_string();

    std::fs::write(&path, raw_content)
        .map_err(|e| format!("Failed to write .changesignore: {}", e))?;

    let patterns = parse_patterns(raw_content);
    log::info!(
        "Saved .changesignore with {} patterns to {:?}",
        patterns.len(),
        path
    );

    Ok(ChangesIgnoreResponse {
        patterns,
        raw_content: raw_content.to_string(),
        source: "file".to_string(),
        file_path,
    })
}

/// Merge `.changesignore` patterns with explicit exclude query params.
///
/// If explicit excludes are provided, they are used AS-IS (no merging).
/// If no explicit excludes are provided, auto-loads from `.changesignore`.
///
/// This allows REST API users to override with `?exclude=...` while
/// the UI gets automatic filtering by default.
pub fn merge_excludes(explicit_excludes: &[String]) -> Vec<String> {
    if explicit_excludes.is_empty() {
        // No explicit excludes — auto-load from .changesignore
        load_patterns()
    } else {
        // Explicit excludes provided — use them as-is
        // (Caller chose specific patterns, don't second-guess)
        explicit_excludes.to_vec()
    }
}

/// Return the built-in default patterns.
fn default_patterns() -> Vec<String> {
    DEFAULT_PATTERNS.iter().map(|s| s.to_string()).collect()
}

/// Generate the default file content with comments.
fn default_file_content() -> String {
    r#"# .changesignore — Patterns to exclude from Changes/Latest diff views
# One pattern per line. These are passed as git pathspec exclusions.
# Lines starting with # are comments. Blank lines are ignored.
#
# Patterns must match the FULL path as stored in the checkpoint repo.
# e.g. use "src-tauri/target" not just "target"

# Rust build output (the big one — 1500+ files)
src-tauri/target

# Generated Tauri schemas (large, auto-generated)
src-tauri/gen/schemas

# Node dependencies
node_modules

# Build output
dist
build
.output

# Lock files (large, auto-generated)
package-lock.json
Cargo.lock

# IDE / OS
.vscode
.idea
.DS_Store
Thumbs.db
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_patterns() {
        let content = r#"
# Comment line
target
  node_modules  

# Another comment
dist

.DS_Store
"#;
        let patterns = parse_patterns(content);
        assert_eq!(patterns, vec!["target", "node_modules", "dist", ".DS_Store"]);
    }

    #[test]
    fn test_parse_empty() {
        assert!(parse_patterns("").is_empty());
        assert!(parse_patterns("# only comments\n# here").is_empty());
    }

    #[test]
    fn test_merge_with_explicit() {
        let explicit = vec!["foo".to_string(), "bar".to_string()];
        let merged = merge_excludes(&explicit);
        assert_eq!(merged, vec!["foo", "bar"]);
    }

    #[test]
    fn test_merge_empty_loads_defaults() {
        let merged = merge_excludes(&[]);
        // Should return at least the built-in defaults (or file-loaded patterns)
        assert!(!merged.is_empty());
    }
}
