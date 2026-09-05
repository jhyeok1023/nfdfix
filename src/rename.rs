use std::fs;
use std::io;
use std::path::Path;

/// What one rename attempt did.
pub enum Outcome {
    /// The entry was renamed, or would be under `--dry-run`.
    Renamed,
    /// The destination name is taken by another entry; nothing was touched.
    Skipped,
    /// The rename was attempted and the filesystem refused it.
    Failed,
}

pub fn apply(old: &Path, new: &Path, dry_run: bool) -> Outcome {
    if let Some(reason) = conflict(old, new) {
        eprintln!(
            "skipped: {} -> {}: {}",
            old.display(),
            new.display(),
            reason
        );
        return Outcome::Skipped;
    }

    if dry_run {
        println!("{} -> {}", old.display(), new.display());
        return Outcome::Renamed;
    }

    match fs::rename(old, new) {
        Ok(_) => {
            println!("renamed: {} -> {}", old.display(), new.display());
            Outcome::Renamed
        }
        Err(e) => {
            eprintln!(
                "rename failed: {} -> {}: {}",
                old.display(),
                new.display(),
                e
            );
            Outcome::Failed
        }
    }
}

/// Why `new` cannot be taken, or `None` if the rename may go ahead.
///
/// `fs::rename` replaces an existing destination on every target platform and
/// reports success, so without this check normalizing an NFD name destroys its
/// NFC twin silently.
fn conflict(old: &Path, new: &Path) -> Option<String> {
    // `symlink_metadata`, not `exists()`: the latter follows symbolic links and
    // so reports `false` for a dangling one, which would let the rename clobber
    // the link.
    let dest = match fs::symlink_metadata(new) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return None,
        // The destination could not be inspected. Refuse rather than risk
        // overwriting whatever is behind the error.
        Err(e) => return Some(format!("cannot inspect the destination: {e}")),
        Ok(m) => m,
    };

    // On a normalization-insensitive filesystem such as macOS APFS, the NFD
    // name resolves to the entry already stored under the NFC name, so the
    // destination is the source. That is the rename this tool exists to
    // perform, not a collision.
    match fs::symlink_metadata(old) {
        Ok(source) if is_same_file(&source, &dest) => None,
        // The source could not be stated either. Let `fs::rename` run and
        // report the real error rather than misfiling it as a name conflict.
        Err(_) => None,
        Ok(_) => Some("destination already exists".to_string()),
    }
}

#[cfg(unix)]
fn is_same_file(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    a.dev() == b.dev() && a.ino() == b.ino()
}

/// Windows has no stable file-identity API: `MetadataExt::file_index` and
/// `volume_serial_number` are still behind the unstable `windows_by_handle`
/// feature, and a dependency on `windows-sys` is not worth it here. Windows
/// does not need one. NTFS, exFAT and FAT fold case but not normalization, so
/// an NFC name and its NFD twin are always two distinct entries and the plain
/// existence check above is exact.
///
/// Note the arm is wider than that justification: `not(unix)` also covers wasm
/// and any other future target. With the three supported platforms that costs
/// nothing today, but the reasoning above is about Windows filesystems alone.
#[cfg(not(unix))]
fn is_same_file(_: &fs::Metadata, _: &fs::Metadata) -> bool {
    false
}
