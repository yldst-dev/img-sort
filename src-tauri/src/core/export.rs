use anyhow::{anyhow, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub fn copy_to_category(
    export_root: &Path,
    category: &str,
    file_name: &str,
    source: &Path,
) -> Result<PathBuf> {
    let target_dir = export_root.join(category);
    copy_to_dir(&target_dir, file_name, source)
}

pub fn copy_to_category_nested(
    export_root: &Path,
    dirs: &[&str],
    file_name: &str,
    source: &Path,
) -> Result<PathBuf> {
    let mut target_dir = export_root.to_path_buf();
    for d in dirs.iter().filter(|s| !s.trim().is_empty()) {
        target_dir = target_dir.join(d);
    }
    copy_to_dir(&target_dir, file_name, source)
}

fn copy_to_dir(target_dir: &Path, file_name: &str, source: &Path) -> Result<PathBuf> {
    fs::create_dir_all(&target_dir)?;
    let mut target = target_dir.join(file_name);
    if target.exists() {
        let stem = target
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("image");
        let ext = target.extension().and_then(|s| s.to_str()).unwrap_or("jpg");
        let mut counter = 1;
        loop {
            let candidate = target_dir.join(format!("{stem}_{counter}.{ext}"));
            if !candidate.exists() {
                target = candidate;
                break;
            }
            counter += 1;
            if counter > 9999 {
                return Err(anyhow!("too many duplicates for {}", file_name));
            }
        }
    }
    fs::copy(source, &target)?;
    Ok(target)
}
