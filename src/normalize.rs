use unicode_normalization::UnicodeNormalization;

pub fn to_nfc(name: &std::path::Path) -> std::path::PathBuf {
    let file_name = name.file_name().unwrap().to_string_lossy();
    let normalized: String = file_name.nfc().collect();

    let mut new_path = name.to_path_buf();
    new_path.set_file_name(normalized);

    new_path
}
