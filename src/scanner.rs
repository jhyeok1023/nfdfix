use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn scan(path: &Path, recursive: bool) -> Vec<PathBuf> {
    let mut entries = Vec::new();

    if path.is_file() {
        entries.push(path.to_path_buf());
        return entries;
    }

    if !path.is_dir() {
        return entries;
    }

    if recursive {
        for entry in WalkDir::new(path).sort_by_file_name() {
            match entry {
                Ok(e) => entries.push(e.path().to_path_buf()),
                Err(e) => eprintln!("scan error: {}", e),
            }
        }
    } else {
        match std::fs::read_dir(path) {
            Ok(dir) => {
                for entry in dir {
                    match entry {
                        Ok(e) => entries.push(e.path().to_path_buf()),
                        Err(e) => eprintln!("scan error: {}", e),
                    }
                }
            }
            Err(e) => eprintln!("scan error: {}", e),
        }
    }

    entries
}
