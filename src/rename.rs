use std::fs;
use std::io;
use std::path::Path;

/// What one rename attempt did.
pub enum Outcome {
    /// The entry was renamed, or would be under `--dry-run`.
    Renamed,
    /// The destination name is taken by another entry; nothing was touched.
    Skipped,
    /// The rename did not happen and the reason was not a name conflict:
    /// either the filesystem refused the call, or the paths involved could not
    /// be inspected well enough to make the call safely.
    Failed,
}

pub fn apply(old: &Path, new: &Path, dry_run: bool) -> Outcome {
    match guard(old, new) {
        Guard::Clear => {}
        Guard::Conflict => {
            eprintln!(
                "skipped: {} -> {}: destination already exists",
                old.display(),
                new.display()
            );
            return Outcome::Skipped;
        }
        // Not a name conflict. Exit code 2 is defined as "nothing failed", and
        // a path that cannot be read is a failure, so this counts as one. It
        // keeps the `rename failed:` prefix so the failures printed still add
        // up to the error count in the summary line.
        Guard::Unreadable(reason) => {
            eprintln!(
                "rename failed: {} -> {}: {}",
                old.display(),
                new.display(),
                reason
            );
            return Outcome::Failed;
        }
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

/// What inspecting the destination found.
enum Guard {
    /// Nothing is in the way; the rename may go ahead.
    Clear,
    /// The destination is a different entry that already exists.
    Conflict,
    /// Whether the destination is in the way could not be determined. The
    /// string says which inspection failed and why.
    Unreadable(String),
}

/// Whether `new` may be taken.
///
/// `fs::rename` replaces an existing destination on every target platform and
/// reports success, so without this check normalizing an NFD name destroys its
/// NFC twin silently.
///
/// The check is not atomic. `fs::rename` is a second syscall, so an entry
/// created at `new` in between is still replaced. Closing that window needs
/// `renameat2` with `RENAME_NOREPLACE` on Linux, or `MoveFileExW` without
/// `MOVEFILE_REPLACE_EXISTING` on Windows; `std` exposes neither, and reaching
/// for them means a per-platform dependency. The race needs another process
/// writing into the tree mid-run, which is out of scope here, but the window
/// is real rather than closed.
fn guard(old: &Path, new: &Path) -> Guard {
    // `symlink_metadata`, not `exists()`: the latter follows symbolic links and
    // so reports `false` for a dangling one, which would let the rename clobber
    // the link.
    let dest = match fs::symlink_metadata(new) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Guard::Clear,
        // The destination could not be inspected. Refuse rather than risk
        // overwriting whatever is behind the error.
        Err(e) => return Guard::Unreadable(format!("cannot inspect the destination: {e}")),
        Ok(m) => m,
    };

    // On a normalization-insensitive filesystem such as macOS APFS, the NFD
    // name resolves to the entry already stored under the NFC name, so the
    // destination is the source. That is the rename this tool exists to
    // perform, not a collision.
    match fs::symlink_metadata(old) {
        Ok(source) if is_same_file(&source, &dest) => Guard::Clear,
        Ok(_) => Guard::Conflict,
        // `new` is known to exist by this point, and whether it is the same
        // entry as `old` is exactly what could not be determined. Letting
        // `fs::rename` run would replace an existing entry on the strength of
        // a check that never completed, so refuse instead.
        Err(e) => Guard::Unreadable(format!("cannot inspect the source: {e}")),
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
