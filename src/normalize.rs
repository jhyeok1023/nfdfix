use std::path::{Component, Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

pub fn to_nfc(path: &Path) -> std::path::PathBuf {
    let mut result = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Normal(name) => match name.to_str() {
                Some(s) => result.push(s.nfc().collect::<String>()),
                None => result.push(name),
            },

            other => result.push(other),
        }
    }

    result
}
