use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const ALLOWED_EXT: &[&str] = &["png", "jpg", "jpeg", "heic", "dng"];

pub fn scan_sources(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension().and_then(|s| s.to_str()) {
                if ALLOWED_EXT.contains(&ext.to_lowercase().as_str()) {
                    files.push(entry.path().to_path_buf());
                }
            }
        }
    }
    Ok(files)
}
