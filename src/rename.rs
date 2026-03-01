use std::path::Path;

pub fn apply(old: &Path, new: &Path, dry_run: bool) -> bool {
    if dry_run {
        println!("{} -> {}", old.display(), new.display());
        return true;
    }

    match std::fs::rename(old, new) {
        Ok(_) => {
            println!("renamed: {} -> {}", old.display(), new.display());
            true
        }
        Err(e) => {
            eprintln!(
                "rename failed: {} -> {}: {}",
                old.display(),
                new.display(),
                e
            );
            false
        }
    }
}
