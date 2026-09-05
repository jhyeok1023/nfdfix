//! Two defects in the path walk in `src/main.rs`, both independent of the
//! twin-clobbering in twin_collision.rs.
//!
//! main.rs:38-39 advances `current` and inserts into `processed` even when the
//! rename failed, so descendants are rebuilt against a directory that was
//! never created. main.rs:22 walks every component of each entry including the
//! CLI prefix, so the tool renames ancestors of the path it was given.

pub mod common;

use common::*;
use std::ffi::OsStr;
use std::path::Path;

/// The destination twin must be non-empty. Renaming a directory onto an empty
/// one succeeds on Linux and on Windows 10 1607+ NTFS, which would silently
/// change what these tests measure.
fn build_cascade_fixture(root: &Path) {
    let source = make_dir(root, GA_NFD);
    write_file(&source, &txt(E_MARKS_NFD), "CHILD-1");
    write_file(&source, &txt(S_MARKS_NFD), "CHILD-2");
    write_file(&source, &txt(HANGEUL_NFD), "CHILD-3");
    let nested = make_dir(&source, &format!("sub_{HANGEUL_NFD}"));
    write_file(&nested, &txt(GA_NFD), "CHILD-4");

    let dest = make_dir(root, GA_NFC);
    write_file(&dest, "keep.txt", "KEEP");

    assert_distinct_entries(root, &[GA_NFD, GA_NFC]);
}

/// A failed directory rename produces no failures for paths that do not exist.
///
/// Fails today: the NFD directory cannot rename onto its non-empty NFC twin,
/// but main.rs advances past the failure anyway, so each of the five NFD-named
/// descendants is retried under a directory name that was never created. One
/// real conflict becomes six reported failures, and the descendants are never
/// normalized at all.
#[test]
fn failed_directory_rename_must_not_produce_phantom_child_failures() {
    require_twins("failed_directory_rename_must_not_produce_phantom_child_failures");

    let dir = fixture();
    let root = dir.path();
    build_cascade_fixture(root);

    let run = run_recursive(root);

    // The genuine conflict is the only failure whose source still exists.
    // Anything else names a path the tool invented.
    let phantom: Vec<&str> = run
        .failure_source_paths()
        .into_iter()
        .filter(|old| !Path::new(old).exists())
        .collect();

    assert!(
        phantom.is_empty(),
        "after a single real directory conflict, the tool reported {} rename \
         failure(s) for source paths that do not exist:\n  {}\n\n{}",
        phantom.len(),
        phantom.join("\n  "),
        run.debug()
    );
    // How many failures a real conflict produces is not this test's claim. A
    // fix may merge the directories, or rename one aside, and report a
    // different count without being wrong. What has to hold is that the
    // summary agrees with the failures actually printed.
    assert_eq!(
        run.reported_errors(),
        run.rename_failures(),
        "the summary line disagrees with the {} failure(s) actually printed\n{}",
        run.rename_failures(),
        run.debug()
    );
}

/// A failed directory rename loses none of the data involved.
///
/// Passes today only because the destination happened to be non-empty. Pinned
/// so a fix that resolves the conflict by merging or moving cannot quietly
/// drop a child.
#[test]
fn failed_directory_rename_must_not_lose_data() {
    require_twins("failed_directory_rename_must_not_lose_data");

    let dir = fixture();
    let root = dir.path();
    build_cascade_fixture(root);

    let run = run_recursive(root);

    let survivors = surviving_contents(root);
    for payload in ["CHILD-1", "CHILD-2", "CHILD-3", "CHILD-4", "KEEP"] {
        assert!(
            survivors.contains(payload),
            "{payload} was lost while resolving a directory name conflict.\n\
             surviving file contents: {survivors:?}\n{}",
            run.debug()
        );
    }
}

/// `fixture()` asserts the tempdir path is ASCII, so the directory planted
/// here is the only NFD-named ancestor and any ancestor rename is ours.
/// Returns the ASCII scan root to pass on the command line.
fn build_ancestor_fixture(root: &Path) -> std::path::PathBuf {
    let ancestor = make_dir(root, GA_NFD);
    let scan_root = make_dir(&ancestor, SCAN_ROOT);
    write_file(&scan_root, "payload.txt", "PAYLOAD");
    write_file(&scan_root, &txt(E_MARKS_NFD), "INSIDE");
    assert_distinct_entries(root, &[GA_NFD]);
    scan_root
}

/// The output checks are primary: the tool announces every rename it attempts,
/// `renamed:` on stdout when the call returned Ok and `rename failed:` on
/// stderr when it did not, so they hold on Linux, on Windows, and on macOS
/// APFS, where renaming to an equivalent name is implementation defined.
///
/// Both are needed. What is wrong here is that the ancestor is walked at all,
/// and a scope fix that leaves the component walk in place can end up
/// attempting the rename and failing it: no `renamed:` line is printed and the
/// directory is still there, so the entry check below waves it through and the
/// defect looks fixed. That is exactly the shape a careless narrowing takes.
fn assert_ancestor_untouched(root: &Path, run: &Run) {
    let ancestor = OsStr::new(GA_NFD);
    assert!(
        !run.renamed_source_names().iter().any(|n| n == ancestor),
        "the tool renamed [{}], a directory above the path it was asked to \
         scan.\n{}",
        codepoints(GA_NFD),
        run.debug()
    );
    assert!(
        !run.failure_source_paths()
            .into_iter()
            .any(|p| Path::new(p).file_name() == Some(ancestor)),
        "the tool tried to rename [{}], a directory above the path it was \
         asked to scan, and merely failed to. Out-of-scope paths must not be \
         attempted at all.\n{}",
        codepoints(GA_NFD),
        run.debug()
    );
    assert!(
        entry_names(root).iter().any(|n| n == ancestor),
        "the NFD-named ancestor [{}] is gone from the fixture root.\n\
         remaining entries:\n{}\n{}",
        codepoints(GA_NFD),
        describe_dir(root),
        run.debug()
    );
}

/// A recursive run renames nothing above the directory it was given.
///
/// Fails today: main.rs:22 walks every component of each entry, including the
/// prefix that came from the command line, so the NFD-named parent of the scan
/// root is renamed even though it was never in scope.
#[test]
fn nfd_named_ancestor_above_the_scan_root_must_not_be_renamed() {
    require_exact_names("nfd_named_ancestor_above_the_scan_root_must_not_be_renamed");

    let dir = fixture();
    let root = dir.path();
    let scan_root = build_ancestor_fixture(root);

    let run = run_recursive(&scan_root);

    assert_ancestor_untouched(root, &run);

    // The point is scope, not refusal: in-scope work must still happen.
    let survivors = surviving_contents(root);
    assert!(
        survivors.contains("INSIDE") && survivors.contains("PAYLOAD"),
        "the in-scope files were damaged\nsurviving file contents: {survivors:?}\n{}",
        run.debug()
    );
}

/// The same holds without `-r`.
///
/// Fails today for the same reason, which locates the defect in main.rs's
/// component walk rather than in WalkDir.
#[test]
fn nfd_named_ancestor_must_not_be_renamed_without_recursive_either() {
    require_exact_names("nfd_named_ancestor_must_not_be_renamed_without_recursive_either");

    let dir = fixture();
    let root = dir.path();
    let scan_root = build_ancestor_fixture(root);

    let run = run_flat(&scan_root);

    assert_ancestor_untouched(root, &run);
}

/// Naming a single ASCII file renames none of its ancestors.
///
/// Fails today, and states the scope defect at its sharpest: `scanner::scan`
/// takes its `is_file()` early return and yields exactly one path, yet the
/// component walk still reaches two directories up and moves the grandparent.
#[test]
fn single_file_argument_must_not_rename_its_ancestors() {
    require_exact_names("single_file_argument_must_not_rename_its_ancestors");

    let dir = fixture();
    let root = dir.path();
    let scan_root = build_ancestor_fixture(root);

    let run = run_flat(&scan_root.join("payload.txt"));

    assert_ancestor_untouched(root, &run);
}
