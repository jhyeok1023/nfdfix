use std::path::Path;

pub fn apply(old: &Path, new: &Path, dry_run: bool) {
    if dry_run {
        println!("{} -> {}", old.display(), new.display());
        return;
    }

    if let Err(e) = std::fs::rename(old, new) {
        eprintln!("rename failed: {}", e);
    } else {
        println!("renamed: {} -> {}", old.display(), new.display());
    }
}
