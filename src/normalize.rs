use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

/// `path` with only its final component normalized to NFC.
///
/// The walk in `main.rs` rebuilds an entry one component at a time, so by the
/// time a component is examined every ancestor above it already carries the
/// name it has on disk. That name may still be NFD on purpose, because the
/// rename was skipped over a conflict or failed outright. Normalizing the
/// whole path would rewrite those ancestors back to a form that was never
/// created, aiming the rename at a directory the entry does not live in, so
/// only the leaf is touched here.
///
/// A leaf that is not valid UTF-8 is returned unchanged, as is a path whose
/// last component is not a normal name (`/`, `.`, `..`, a Windows prefix).
pub fn leaf_to_nfc(path: &Path) -> PathBuf {
    match path.file_name().and_then(|leaf| leaf.to_str()) {
        Some(leaf) => path.with_file_name(leaf.nfc().collect::<String>()),
        None => path.to_path_buf(),
    }
}
