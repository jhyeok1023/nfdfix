//! `rename::apply` calls `fs::rename` with no collision check. That call
//! replaces an existing destination on every target platform and returns Ok,
//! so the tool logs a clobber as a success, counts no errors, and exits 0.

pub mod common;

use common::*;
use std::fs;

/// Body of the four twin-file cases: both payloads must outlive a run that
/// normalizes one of the two names.
fn assert_twin_files_both_survive(test: &str, nfc_stem: &str, nfd_stem: &str) {
    require_twins(test);

    let dir = fixture();
    let root = dir.path();

    let nfd_name = txt(nfd_stem);
    let nfc_name = txt(nfc_stem);

    write_file(root, &nfd_name, "CONTENT-NFD");
    write_file(root, &nfc_name, "CONTENT-NFC");
    assert_distinct_entries(root, &[&nfd_name, &nfc_name]);

    // Top level only. Directory order is arbitrary on btrfs and NTFS, but only
    // the NFD entry differs from its NFC form, so it is the sole rename
    // attempted whichever way round read_dir hands them over.
    let run = run_flat(root);

    // The exit code is evidence, not the verdict: a clobbering rename returns
    // Ok, so the run ends at 0 with the file already gone. Asserting on it here
    // would make this test pass the moment the tool merely started reporting
    // the conflict, whether or not it stopped destroying the file.
    let survivors = surviving_contents(root);
    assert!(
        survivors.contains("CONTENT-NFC"),
        "the NFC twin [{}] was silently overwritten by its NFD twin [{}].\n\
         The tool exited with {:?} and reported no error.\n\
         surviving file contents: {survivors:?}\nremaining entries:\n{}\n{}",
        codepoints(&nfc_name),
        codepoints(&nfd_name),
        run.code,
        describe_dir(root),
        run.debug()
    );
    assert!(
        survivors.contains("CONTENT-NFD"),
        "the NFD twin's contents were lost.\n\
         surviving file contents: {survivors:?}\n{}",
        run.debug()
    );
}

/// A precomposed syllable and its jamo decomposition both survive a run.
///
/// Fails today: `fs::rename` replaces the destination, so normalizing the NFD
/// name unlinks the NFC file. The run reports one rename, no errors, exit 0.
#[test]
fn hangul_syllable_twin_files_must_both_survive() {
    assert_twin_files_both_survive(
        "hangul_syllable_twin_files_must_both_survive",
        GA_NFC,
        GA_NFD,
    );
}

/// The same holds for a word whose syllables carry trailing jongseong.
///
/// Fails today for the same reason as the single-syllable case. Kept separate
/// because a fix handling only consonant-vowel syllables would pass that one.
#[test]
fn hangul_word_twin_files_must_both_survive() {
    assert_twin_files_both_survive(
        "hangul_word_twin_files_must_both_survive",
        HANGEUL_NFC,
        HANGEUL_NFD,
    );
}

/// The same holds for a Latin base carrying two combining marks.
///
/// Fails today for the same reason, but through canonical ordering rather
/// than Hangul's algorithmic composition, which is a different code path.
#[test]
fn combining_marks_twin_files_must_both_survive() {
    assert_twin_files_both_survive(
        "combining_marks_twin_files_must_both_survive",
        E_MARKS_NFC,
        E_MARKS_NFD,
    );
}

/// The same holds for a second base with different marks.
///
/// Fails today for the same reason. Its purpose is to deny a fix that special
/// cases one letter and leaves the general problem in place.
#[test]
fn second_combining_marks_twin_files_must_both_survive() {
    assert_twin_files_both_survive(
        "second_combining_marks_twin_files_must_both_survive",
        S_MARKS_NFC,
        S_MARKS_NFD,
    );
}

/// Names that are already NFC and have no twin are left exactly as they are.
///
/// Passes today, and is here so that a red result elsewhere is attributable to
/// the bug rather than to the harness. Gated on exact names rather than twins,
/// so it still runs on macOS APFS.
#[test]
fn already_nfc_names_without_twins_are_left_untouched() {
    require_exact_names("already_nfc_names_without_twins_are_left_untouched");

    let dir = fixture();
    let root = dir.path();

    write_file(root, &txt(CAFE_NFC), "CAFE");
    write_file(root, &txt(PLAIN_ASCII), "PLAIN");

    let before = snapshot(root);
    let run = run_flat(root);
    let after = snapshot(root);

    assert_eq!(
        before,
        after,
        "the tool modified files that were already NFC and had no twin\n{}",
        run.debug()
    );
    assert_eq!(
        run.renames(),
        0,
        "the tool renamed something it should have left alone\n{}",
        run.debug()
    );
    assert_eq!(
        run.rename_failures(),
        0,
        "the tool reported a rename failure with nothing to rename\n{}",
        run.debug()
    );
    assert_eq!(run.code, Some(0), "expected a clean exit\n{}", run.debug());
}

/// `--dry-run` leaves the filesystem untouched even with a twin pair present.
///
/// Passes today, because `rename::apply` returns before reaching `fs::rename`.
/// Pinned so that stays true when a fix rewrites that function.
#[test]
fn dry_run_must_not_touch_the_filesystem() {
    require_twins("dry_run_must_not_touch_the_filesystem");

    let dir = fixture();
    let root = dir.path();

    write_file(root, &txt(GA_NFD), "CONTENT-NFD");
    write_file(root, &txt(GA_NFC), "CONTENT-NFC");
    assert_distinct_entries(root, &[&txt(GA_NFD), &txt(GA_NFC)]);

    let before = snapshot(root);
    let run = run_dry(root);
    let after = snapshot(root);

    assert_eq!(
        before,
        after,
        "--dry-run modified the filesystem\n{}",
        run.debug()
    );
    assert!(
        run.stdout.contains("[dry-run]"),
        "--dry-run did not print its summary\n{}",
        run.debug()
    );
}

/// Renaming a directory does not let the tool clobber a twin inside it.
///
/// Fails today, and this is the sharpest case in the suite: the parent has no
/// twin of its own, and the file that dies sits at a path the scanner never
/// enumerated in that form. The NFD directory renames cleanly and goes into
/// `processed`; walking its NFD-named child, the lookup at main.rs:25-28
/// rewrites `current` to the renamed parent, so the tool renames
/// `<nfc dir>/<nfd child>` onto `<nfc dir>/<nfc child>` and destroys it. Two
/// renames, no errors, exit 0.
#[test]
fn nfd_file_inside_renamed_nfd_directory_must_not_clobber_twin() {
    require_twins("nfd_file_inside_renamed_nfd_directory_must_not_clobber_twin");

    let dir = fixture();
    let root = dir.path();

    let dir_nfd = HANGEUL_NFD.to_string();
    let child_nfd = txt(GA_NFD);
    let child_nfc = txt(GA_NFC);

    let nested = make_dir(root, &dir_nfd);
    write_file(&nested, &child_nfd, "NESTED-NFD");
    write_file(&nested, &child_nfc, "NESTED-NFC");

    assert_distinct_entries(root, &[&dir_nfd]);
    assert_eq!(
        entry_names(root).len(),
        1,
        "this case requires the NFD directory to have no twin of its own:\n{}",
        describe_dir(root)
    );
    assert_distinct_entries(&nested, &[&child_nfd, &child_nfc]);

    let run = run_recursive(root);

    let survivors = surviving_contents(root);
    assert!(
        survivors.contains("NESTED-NFC") && survivors.contains("NESTED-NFD"),
        "a nested twin pair was collapsed while the parent directory was being \
         normalized. The tool exited with {:?}, reporting {} rename(s) and {} \
         failure(s), yet only {survivors:?} survived.\n{}",
        run.code,
        run.renames(),
        run.rename_failures(),
        run.debug()
    );
}

/// A conflict between a file and a directory loses nothing and is reported.
///
/// Covers both directions: an NFD file against an NFC directory, and an NFD
/// directory against an NFC file. Unix splits these into EISDIR and ENOTDIR,
/// and Windows differs again.
///
/// Passes today, since both renames fail and the run exits 1. It is here to
/// make the fix decide rather than to reproduce a loss: an existence check
/// written as a bare `new.exists()` treats every kind alike, and if it skips
/// silently the tool would report success while leaving both NFD names in
/// place, which turns the last two assertions red.
#[test]
fn cross_type_twin_conflicts_must_be_reported_and_lose_nothing() {
    require_twins("cross_type_twin_conflicts_must_be_reported_and_lose_nothing");

    let dir = fixture();
    let root = dir.path();

    let file_nfd = txt(GA_NFD);
    let dir_nfc = txt(GA_NFC);
    write_file(root, &file_nfd, "FILE-OVER-DIR");
    let held = make_dir(root, &dir_nfc);
    write_file(&held, "child.txt", "DIR-HELD-CHILD");

    let dir_nfd = HANGEUL_NFD.to_string();
    let file_nfc = HANGEUL_NFC.to_string();
    let moving = make_dir(root, &dir_nfd);
    write_file(&moving, "child.txt", "DIR-MOVING-CHILD");
    write_file(root, &file_nfc, "FILE-HELD");

    assert_distinct_entries(root, &[&file_nfd, &dir_nfc, &dir_nfd, &file_nfc]);

    // Flat, so the directories' children are never scanned. That keeps this
    // case about the cross-type conflict rather than the phantom cascade in
    // walk_semantics.rs.
    let run = run_flat(root);

    let survivors = surviving_contents(root);
    for payload in [
        "FILE-OVER-DIR",
        "DIR-HELD-CHILD",
        "DIR-MOVING-CHILD",
        "FILE-HELD",
    ] {
        assert!(
            survivors.contains(payload),
            "{payload} was lost to a conflict between a file and a directory.\n\
             surviving file contents: {survivors:?}\nremaining entries:\n{}\n{}",
            describe_dir(root),
            run.debug()
        );
    }

    assert!(
        fs::metadata(root.join(&dir_nfc))
            .expect("the NFC directory is missing")
            .is_dir(),
        "the NFC directory was replaced by a file\n{}",
        run.debug()
    );
    assert!(
        fs::metadata(root.join(&file_nfc))
            .expect("the NFC file is missing")
            .is_file(),
        "the NFC file was replaced by a directory\n{}",
        run.debug()
    );

    // Neither rename can succeed, so the tool has to say so. A fix that skips
    // on `exists()` without reporting would turn this into a silent no-op with
    // both NFD names left behind.
    assert_ne!(
        run.code,
        Some(0),
        "the tool exited cleanly despite two unresolvable name conflicts\n{}",
        run.debug()
    );
    assert_eq!(
        run.renames(),
        0,
        "the tool claimed to rename something across a type conflict\n{}",
        run.debug()
    );
}

/// An NFD-named symlink does not replace a real file holding the NFC name.
///
/// Fails today: `rename` does not follow a symlink in the final component, and
/// the Windows fallback opens the source with FILE_FLAG_OPEN_REPARSE_POINT, so
/// the link itself is moved onto the regular file and unlinks it. Exit 0.
#[test]
#[cfg(any(unix, windows))]
fn nfd_symlink_must_not_clobber_its_nfc_regular_file_twin() {
    const TEST: &str = "nfd_symlink_must_not_clobber_its_nfc_regular_file_twin";
    require_twins(TEST);
    require_symlinks(TEST);

    let dir = fixture();
    let root = dir.path();

    let link_nfd = txt(E_MARKS_NFD);
    let file_nfc = txt(E_MARKS_NFC);

    write_file(root, "target.txt", "TARGET-PAYLOAD");
    write_file(root, &file_nfc, "REGULAR-VICTIM");
    make_file_symlink(root, &link_nfd, "target.txt").expect("failed to create the fixture symlink");

    assert_distinct_entries(root, &["target.txt", &link_nfd, &file_nfc]);
    assert!(
        fs::symlink_metadata(root.join(&link_nfd))
            .expect("the fixture symlink is missing")
            .file_type()
            .is_symlink(),
        "precondition failed: the NFD fixture entry is not a symlink"
    );

    let run = run_flat(root);

    let victim = root.join(&file_nfc);
    let meta = fs::symlink_metadata(&victim).unwrap_or_else(|e| {
        panic!(
            "the NFC regular file disappeared entirely: {e}\nremaining entries:\n{}\n{}",
            describe_dir(root),
            run.debug()
        )
    });
    assert!(
        meta.file_type().is_file(),
        "the NFC regular file was replaced by the NFD symlink; it is now {:?}. \
         The tool exited with {:?} and reported no error.\n{}",
        meta.file_type(),
        run.code,
        run.debug()
    );
    assert_eq!(
        fs::read_to_string(&victim).expect("failed to read the NFC regular file"),
        "REGULAR-VICTIM",
        "the NFC regular file's contents were replaced\n{}",
        run.debug()
    );
    assert_eq!(
        fs::read_to_string(root.join("target.txt")).expect("failed to read the symlink target"),
        "TARGET-PAYLOAD",
        "the symlink target was modified; a rename must never follow the link\n{}",
        run.debug()
    );
}

/// A lone NFD-named symlink is renamed as a link, with its target intact.
///
/// Passes today, and is the one thing the tool gets right here. Pinned so a
/// fix for the case above cannot "solve" it by dereferencing links or by
/// refusing to touch them at all.
#[test]
#[cfg(any(unix, windows))]
fn lone_nfd_symlink_is_renamed_as_a_link_with_target_intact() {
    const TEST: &str = "lone_nfd_symlink_is_renamed_as_a_link_with_target_intact";
    require_exact_names(TEST);
    require_symlinks(TEST);

    let dir = fixture();
    let root = dir.path();

    let link_nfd = link(S_MARKS_NFD);
    let link_nfc = link(S_MARKS_NFC);

    write_file(root, "payload.txt", "PAYLOAD");
    make_file_symlink(root, &link_nfd, "payload.txt")
        .expect("failed to create the fixture symlink");

    let run = run_flat(root);

    assert!(
        root.join(&link_nfd).symlink_metadata().is_err(),
        "the NFD-named link was not renamed\nremaining entries:\n{}\n{}",
        describe_dir(root),
        run.debug()
    );
    let renamed = fs::symlink_metadata(root.join(&link_nfc)).unwrap_or_else(|e| {
        panic!(
            "the renamed link is missing: {e}\nremaining entries:\n{}\n{}",
            describe_dir(root),
            run.debug()
        )
    });
    assert!(
        renamed.file_type().is_symlink(),
        "the link was replaced by something that is not a link: {:?}\n{}",
        renamed.file_type(),
        run.debug()
    );
    assert_eq!(
        fs::read_link(root.join(&link_nfc)).expect("failed to read the renamed link"),
        std::path::Path::new("payload.txt"),
        "the link now points somewhere else\n{}",
        run.debug()
    );
    assert_eq!(
        fs::read_to_string(root.join("payload.txt")).expect("failed to read the target"),
        "PAYLOAD",
        "the link target was modified\n{}",
        run.debug()
    );
}
