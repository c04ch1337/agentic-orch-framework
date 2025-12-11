//! Path validation
//!
//! This module provides validators for file paths, helping prevent
//! path traversal attacks and ensuring paths are safe and valid.

use crate::errors::{ValidationError, ValidationResult};
use path_absolutize::Absolutize;
use pathdiff::diff_paths;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Validate path exists (wrapper around Path::exists)
pub fn path_exists(path: &str) -> ValidationResult<()> {
    let p = Path::new(path);
    if p.exists() {
        Ok(())
    } else {
        Err(ValidationError::InvalidPath(format!(
            "Path '{}' does not exist",
            path
        )))
    }
}

/// Validate path is absolute
pub fn is_absolute_path(path: &str) -> ValidationResult<()> {
    let p = Path::new(path);
    if p.is_absolute() {
        Ok(())
    } else {
        Err(ValidationError::InvalidPath(format!(
            "Path '{}' is not absolute",
            path
        )))
    }
}

/// Validate path is relative
pub fn is_relative_path(path: &str) -> ValidationResult<()> {
    let p = Path::new(path);
    if p.is_relative() {
        Ok(())
    } else {
        Err(ValidationError::InvalidPath(format!(
            "Path '{}' is not relative",
            path
        )))
    }
}

/// Validate path is a file
pub fn is_file(path: &str) -> ValidationResult<()> {
    let p = Path::new(path);
    if p.exists() && p.is_file() {
        Ok(())
    } else {
        Err(ValidationError::InvalidPath(format!(
            "Path '{}' is not a file or does not exist",
            path
        )))
    }
}

/// Validate path is a directory
pub fn is_directory(path: &str) -> ValidationResult<()> {
    let p = Path::new(path);
    if p.exists() && p.is_dir() {
        Ok(())
    } else {
        Err(ValidationError::InvalidPath(format!(
            "Path '{}' is not a directory or does not exist",
            path
        )))
    }
}

/// Validate path has an allowed extension
pub fn allowed_extension(path: &str, allowed_extensions: &[&str]) -> ValidationResult<()> {
    let p = Path::new(path);
    if let Some(ext) = p.extension() {
        if let Some(ext_str) = ext.to_str() {
            if allowed_extensions.contains(&ext_str) {
                Ok(())
            } else {
                Err(ValidationError::InvalidPath(format!(
                    "Path has disallowed extension '{}'. Allowed: {:?}",
                    ext_str, allowed_extensions
                )))
            }
        } else {
            Err(ValidationError::InvalidPath(
                "Path extension contains invalid UTF-8".to_string(),
            ))
        }
    } else {
        Err(ValidationError::InvalidPath(
            "Path has no extension".to_string(),
        ))
    }
}

/// Validate path does not have a denied extension
pub fn denied_extension(path: &str, denied_extensions: &[&str]) -> ValidationResult<()> {
    let p = Path::new(path);
    if let Some(ext) = p.extension() {
        if let Some(ext_str) = ext.to_str() {
            if denied_extensions.contains(&ext_str) {
                Err(ValidationError::InvalidPath(format!(
                    "Path has denied extension '{}'",
                    ext_str
                )))
            } else {
                Ok(())
            }
        } else {
            Err(ValidationError::InvalidPath(
                "Path extension contains invalid UTF-8".to_string(),
            ))
        }
    } else {
        // No extension is considered safe
        Ok(())
    }
}

/// Validate path is within allowed directory
pub fn in_directory(path: &str, allowed_dir: &str) -> ValidationResult<()> {
    let path_buf = PathBuf::from(path);
    let base_dir = PathBuf::from(allowed_dir);

    // Absolutize both paths to handle relative path components
    let abs_path = match path_buf.absolutize() {
        Ok(p) => p,
        Err(_) => {
            return Err(ValidationError::InvalidPath(format!(
                "Failed to resolve absolute path for '{}'",
                path
            )))
        }
    };

    let abs_dir = match base_dir.absolutize() {
        Ok(p) => p,
        Err(_) => {
            return Err(ValidationError::InvalidPath(format!(
                "Failed to resolve absolute path for base directory '{}'",
                allowed_dir
            )))
        }
    };

    let abs_path_str = match abs_path.to_str() {
        Some(s) => s,
        None => {
            return Err(ValidationError::InvalidPath(
                "Path contains invalid UTF-8".to_string(),
            ))
        }
    };

    let abs_dir_str = match abs_dir.to_str() {
        Some(s) => s,
        None => {
            return Err(ValidationError::InvalidPath(
                "Base directory contains invalid UTF-8".to_string(),
            ))
        }
    };

    if abs_path_str.starts_with(abs_dir_str) {
        Ok(())
    } else {
        Err(ValidationError::InvalidPath(format!(
            "Path '{}' is not within allowed directory '{}'",
            path, allowed_dir
        )))
    }
}

/// Validate path does not contain directory traversal patterns (../)
pub fn no_traversal(path: &str) -> ValidationResult<()> {
    // Check for suspicious patterns
    if path.contains("../")
        || path.contains("..\\")
        || path.ends_with("/..")
        || path.ends_with("\\..")
        || path == ".."
    {
        Err(ValidationError::SecurityThreat(format!(
            "Path '{}' contains directory traversal patterns",
            path
        )))
    } else {
        Ok(())
    }
}

/// Validate path is canonical (normalized, absolute, no symbolic links)
pub fn is_canonical(path: &str) -> ValidationResult<()> {
    let p = Path::new(path);
    match p.canonicalize() {
        Ok(canonical) => {
            let canonical_str = match canonical.to_str() {
                Some(s) => s,
                None => {
                    return Err(ValidationError::InvalidPath(
                        "Canonical path contains invalid UTF-8".to_string(),
                    ))
                }
            };

            if canonical_str == path {
                Ok(())
            } else {
                Err(ValidationError::InvalidPath(format!(
                    "Path '{}' is not canonical (resolves to '{}')",
                    path, canonical_str
                )))
            }
        }
        Err(_) => Err(ValidationError::InvalidPath(format!(
            "Failed to canonicalize path '{}'",
            path
        ))),
    }
}

/// Normalize a path by resolving relative components (like .. and .)
pub fn normalize_path(path: &str) -> Result<String, ValidationError> {
    let p = Path::new(path);

    // For relative paths, we'll resolve against the current directory
    let path_to_resolve = if p.is_relative() {
        match std::env::current_dir() {
            Ok(mut cur_dir) => {
                cur_dir.push(p);
                cur_dir
            }
            Err(_) => {
                return Err(ValidationError::InvalidPath(
                    "Failed to get current directory".to_string(),
                ))
            }
        }
    } else {
        PathBuf::from(p)
    };

    // Try to normalize the path
    match path_to_resolve.absolutize() {
        Ok(normalized) => match normalized.to_str() {
            Some(s) => Ok(s.to_string()),
            None => Err(ValidationError::InvalidPath(
                "Normalized path contains invalid UTF-8".to_string(),
            )),
        },
        Err(_) => Err(ValidationError::InvalidPath(format!(
            "Failed to normalize path '{}'",
            path
        ))),
    }
}

/// Validate a path does not contain any denied segments (directory or file names)
pub fn denied_segments(path: &str, denied: &[&str]) -> ValidationResult<()> {
    let p = Path::new(path);

    // Check if any component matches the denied segments
    for component in p.components() {
        if let Some(segment) = component.as_os_str().to_str() {
            if denied.contains(&segment) {
                return Err(ValidationError::InvalidPath(format!(
                    "Path contains denied segment '{}'",
                    segment
                )));
            }
        } else {
            return Err(ValidationError::InvalidPath(
                "Path contains invalid UTF-8".to_string(),
            ));
        }
    }

    Ok(())
}

/// Validate a path is allowed based on a whitelist pattern
pub fn matches_pattern(path: &str, allowed_patterns: &[&str]) -> ValidationResult<()> {
    let p = Path::new(path);

    // Try to match the path against allowed patterns
    for pattern in allowed_patterns {
        if glob_match::glob_match(pattern, path) {
            return Ok(());
        }
    }

    Err(ValidationError::InvalidPath(format!(
        "Path '{}' does not match any allowed pattern",
        path
    )))
}

/// Combine multiple common path safety checks
pub fn is_safe_path(path: &str, base_dir: Option<&str>) -> ValidationResult<()> {
    // Check for traversal patterns
    no_traversal(path)?;

    // If base directory is specified, ensure the path is within it
    if let Some(dir) = base_dir {
        in_directory(path, dir)?;
    }

    // Deny dangerous extensions for extra security
    denied_extension(
        path,
        &[
            "exe", "dll", "bat", "cmd", "sh", "com", "scr", "vbs", "ps1", "py", "rb", "pl", "js",
        ],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    // Helper to create test files/dirs for testing
    fn setup_test_dirs() -> (PathBuf, PathBuf) {
        let tmp_dir = env::temp_dir().join("path_validator_tests");
        let test_file = tmp_dir.join("test_file.txt");

        if !tmp_dir.exists() {
            fs::create_dir(&tmp_dir).unwrap();
        }

        if !test_file.exists() {
            fs::write(&test_file, "test content").unwrap();
        }

        (tmp_dir, test_file)
    }

    #[test]
    fn test_path_type_validation() {
        let (tmp_dir, test_file) = setup_test_dirs();

        let tmp_dir_str = tmp_dir.to_str().unwrap();
        let test_file_str = test_file.to_str().unwrap();

        assert!(is_directory(tmp_dir_str).is_ok());
        assert!(is_file(test_file_str).is_ok());

        assert!(is_directory(test_file_str).is_err());
        assert!(is_file(tmp_dir_str).is_err());
    }

    #[test]
    fn test_extensions() {
        assert!(allowed_extension("file.txt", &["txt", "pdf"]).is_ok());
        assert!(allowed_extension("file.pdf", &["txt", "pdf"]).is_ok());
        assert!(allowed_extension("file.exe", &["txt", "pdf"]).is_err());

        assert!(denied_extension("file.txt", &["exe", "bat"]).is_ok());
        assert!(denied_extension("file.exe", &["exe", "bat"]).is_err());
    }

    #[test]
    fn test_no_traversal() {
        assert!(no_traversal("path/to/file.txt").is_ok());
        assert!(no_traversal("../path/to/file.txt").is_err());
        assert!(no_traversal("path/../file.txt").is_err());
        assert!(no_traversal("path/to/..").is_err());
    }

    #[test]
    fn test_in_directory() {
        let (tmp_dir, test_file) = setup_test_dirs();
        let tmp_dir_str = tmp_dir.to_str().unwrap();
        let test_file_str = test_file.to_str().unwrap();

        assert!(in_directory(test_file_str, tmp_dir_str).is_ok());
        assert!(in_directory(tmp_dir_str, test_file_str).is_err());
    }

    #[test]
    fn test_matches_pattern() {
        assert!(matches_pattern("file.txt", &["*.txt", "*.pdf"]).is_ok());
        assert!(matches_pattern("docs/file.pdf", &["docs/*.pdf"]).is_ok());
        assert!(matches_pattern("file.exe", &["*.txt", "*.pdf"]).is_err());
    }

    #[test]
    fn test_is_safe_path() {
        let (tmp_dir, test_file) = setup_test_dirs();
        let tmp_dir_str = tmp_dir.to_str().unwrap();
        let test_file_str = test_file.to_str().unwrap();

        // Valid path inside base dir with safe extension
        assert!(is_safe_path(test_file_str, Some(tmp_dir_str)).is_ok());

        // Path with traversal
        let traversal_path = format!("{}/../file.txt", tmp_dir_str);
        assert!(is_safe_path(&traversal_path, Some(tmp_dir_str)).is_err());

        // Path with unsafe extension
        let unsafe_ext = tmp_dir.join("malware.exe").to_str().unwrap().to_string();
        assert!(is_safe_path(&unsafe_ext, Some(tmp_dir_str)).is_err());
    }
}
