use std::path::Path;
use indicatif::{ProgressBar, ProgressStyle};
//1

pub fn create_progress_bar(total_size: u64) -> ProgressBar {
    let progress = ProgressBar::new(total_size);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .expect("Failed to set progress bar template")
            .progress_chars("=>-")
    );
    progress
}

/// Formats a file size in human-readable format
/// 
/// # Arguments
/// 
/// * `size` - The size in bytes
/// 
/// # Returns
/// 
/// * `String` - Human-readable size string
pub fn format_size(size: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    format!("{:.2} {}", size, UNITS[unit_index])
}

/// Checks if a path exists and is accessible
/// 
/// # Arguments
/// 
/// * `path` - The path to check
/// 
/// # Returns
/// 
/// * `bool` - True if the path exists and is accessible
pub fn path_exists(path: &Path) -> bool {
    path.exists()
}

/// Gets the file extension from a path
/// 
/// # Arguments
/// 
/// * `path` - The path to get the extension from
/// 
/// # Returns
/// 
/// * `Option<String>` - The extension if it exists
pub fn get_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0.00 B");
        assert_eq!(format_size(1023), "1023.00 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_size(1024 * 1024 * 1024 * 1024), "1.00 TB");
    }

    #[test]
    fn test_path_exists() {
        // 当前目录应该存在
        assert!(path_exists(Path::new(".")));
        // 不太可能存在的路径
        assert!(!path_exists(Path::new("/this/path/should/not/exist/12345")));
    }

    #[test]
    fn test_get_extension() {
        assert_eq!(get_extension(Path::new("test.txt")), Some("txt".to_string()));
        assert_eq!(get_extension(Path::new("test.ZIP")), Some("zip".to_string()));
        assert_eq!(get_extension(Path::new("test")), None);
        assert_eq!(get_extension(Path::new("")), None);
    }
} 