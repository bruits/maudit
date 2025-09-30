use std::path::Path;

/// Simple file path filter for the dev server
pub fn should_watch_path(path: &Path) -> bool {
    // Skip .DS_Store files
    if let Some(file_name) = path.file_name()
        && file_name == ".DS_Store"
    {
        return false;
    }

    // Skip dist and target directories
    if path
        .ancestors()
        .any(|p| p.ends_with("dist") || p.ends_with("target"))
    {
        return false;
    }

    true
}
