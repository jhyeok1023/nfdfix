use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn scan(path: &Path, recursive: bool) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if path.is_file() {
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        if recursive {
            for entry in WalkDir::new(path) {
                let entry = entry.unwrap();
                if entry.file_type().is_file() {
                    files.push(entry.path().to_path_buf());
                }
            }
        } else {
            for entry in std::fs::read_dir(path).unwrap() {
                let entry = entry.unwrap();
                if entry.path().is_file() {
                    files.push(entry.path());
                }
            }
        }
    }

    files
}
