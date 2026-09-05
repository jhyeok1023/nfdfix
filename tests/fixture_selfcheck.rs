//! Guards that must pass before a failure anywhere else can be believed.
//!
//! Two unrelated things can make the reproduction tests lie: a source file
//! whose escapes were replaced with normalized text, and a filesystem that
//! will not hold two canonically equivalent names. These tests separate them.

pub mod common;

use common::*;

/// Each declared pair is canonically equivalent and byte-distinct.
///
/// Fails if this source file was normalized, which would otherwise leave every
/// collision test passing against a fixture that has no twins in it.
#[test]
fn declared_constants_are_well_formed_twins() {
    assert_twins("GA", GA_NFC, GA_NFD);
    assert_twins("HANGEUL", HANGEUL_NFC, HANGEUL_NFD);
    assert_twins("E_MARKS", E_MARKS_NFC, E_MARKS_NFD);
    assert_twins("S_MARKS", S_MARKS_NFC, S_MARKS_NFD);

    // CAFE_NFC is NFC but not NFD, so only one side is checked.
    assert_is_nfc("CAFE_NFC", CAFE_NFC);
    assert_is_nfc("PLAIN_ASCII", PLAIN_ASCII);
    assert_is_nfd("PLAIN_ASCII", PLAIN_ASCII);
}

/// Appending a fixture extension does not change a stem's normalization form.
#[test]
fn fixture_extensions_are_normalization_neutral() {
    for (label, stem) in [
        ("GA_NFD", GA_NFD),
        ("HANGEUL_NFD", HANGEUL_NFD),
        ("E_MARKS_NFD", E_MARKS_NFD),
        ("S_MARKS_NFD", S_MARKS_NFD),
    ] {
        assert_is_nfd(&format!("txt({label})"), &txt(stem));
        assert_is_nfd(&format!("link({label})"), &link(stem));
    }
    for (label, stem) in [
        ("GA_NFC", GA_NFC),
        ("HANGEUL_NFC", HANGEUL_NFC),
        ("E_MARKS_NFC", E_MARKS_NFC),
        ("S_MARKS_NFC", S_MARKS_NFC),
    ] {
        assert_is_nfc(&format!("txt({label})"), &txt(stem));
        assert_is_nfc(&format!("link({label})"), &link(stem));
    }
}

/// No fixture name is illegal or ambiguous on any target platform.
///
/// Windows is the strict one: it reserves device names, strips trailing dots
/// and spaces, forbids some punctuation, and compares case-insensitively.
#[test]
fn fixture_names_are_safe_on_every_target_platform() {
    const RESERVED: [&str; 26] = [
        "CON", "PRN", "AUX", "NUL", "COM0", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
        "COM8", "COM9", "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
        "LPT9", "CONIN$", "CONOUT$",
    ];
    const FORBIDDEN: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

    let names = fixture_names();

    for name in &names {
        let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
        assert!(
            !RESERVED.contains(&stem.as_str()),
            "fixture name {name:?} uses the Windows reserved device name {stem:?}"
        );
        assert!(
            !name.ends_with('.') && !name.ends_with(' '),
            "fixture name {name:?} ends in a dot or a space, which Windows strips"
        );
        assert!(
            !name.contains(FORBIDDEN),
            "fixture name {name:?} contains a character Windows forbids"
        );
        assert!(!name.is_empty(), "fixture names must not be empty");
    }

    // NTFS folds case but not normalization, so the twins are safe there. Two
    // names differing only by ASCII case would not be.
    for (i, a) in names.iter().enumerate() {
        for b in names.iter().skip(i + 1) {
            assert!(
                !(a != b && a.eq_ignore_ascii_case(b)),
                "fixture names {a:?} and {b:?} differ only by ASCII case, which \
                 collides on NTFS"
            );
        }
    }
}

/// This filesystem stores a filename as exactly the bytes it was given.
///
/// Fails on macOS HFS+, which decomposes on store. A suite that cannot trust
/// its own filenames has to say so rather than run anyway.
#[test]
fn environment_preserves_exact_filename_bytes() {
    assert_ne!(
        fs_naming(),
        FsNaming::Decomposes,
        "this filesystem rewrites filenames on store, so no fixture in this \
         suite means what it says. Run the tests on Linux or Windows, or point \
         TMPDIR at a byte-preserving volume."
    );
}

/// This filesystem keeps NFC and NFD names as two distinct entries.
///
/// Fails on macOS APFS and HFS+. Names the cause once and precisely, so a run
/// there does not leave you inferring it from a dozen identical panics in the
/// reproduction tests.
#[test]
fn environment_keeps_nfc_and_nfd_twins_distinct() {
    assert_eq!(
        fs_naming(),
        FsNaming::KeepsTwins,
        "NFC and NFD twins cannot coexist here, so the data-loss reproduction \
         cannot be built. This is a property of the filesystem, not a verdict \
         on nfdfix. Run the tests on Linux or Windows, or point TMPDIR at a \
         byte-preserving volume such as an exFAT disk image."
    );
}

/// This environment can create symbolic links.
///
/// Fails on Windows without Developer Mode or an elevated shell, so the
/// symlink cases cannot silently drop out of a run.
#[test]
fn environment_can_create_symlinks() {
    assert!(
        symlinks_available(),
        "symbolic links cannot be created here, so the symlink reproduction \
         cases cannot run. On Windows, enable Developer Mode or run the tests \
         from an elevated shell."
    );
}
