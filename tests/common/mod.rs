#![allow(dead_code)]
#![warn(missing_docs)]

//! Fixture builders and process helpers shared by the reproduction tests.

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::{env, fs, io};

use unicode_normalization::UnicodeNormalization;

// Names are built from escapes, never pasted as text. Decomposed text in a
// source file is one editor save away from being normalized, which would
// collapse every pair below into a single name and leave the suite passing
// while proving nothing.

/// Precomposed Hangul syllable GA: U+AC00.
pub const GA_NFC: &str = "\u{AC00}";

/// Decomposed Hangul syllable GA: U+1100 CHOSEONG KIYEOK + U+1161 JUNGSEONG A.
pub const GA_NFD: &str = "\u{1100}\u{1161}";

/// Two precomposed Hangul syllables: U+D55C HAN + U+AE00 GEUL.
pub const HANGEUL_NFC: &str = "\u{D55C}\u{AE00}";

/// The same two syllables as six jamo, both with a trailing jongseong.
///
/// HAN is U+1112 HIEUH + U+1161 A + U+11AB JONGSEONG NIEUN; GEUL is U+1100
/// KIYEOK + U+1173 EU + U+11AF JONGSEONG RIEUL. A fix that only handles
/// consonant-vowel syllables passes [`GA_NFD`] and still loses this.
pub const HANGEUL_NFD: &str = "\u{1112}\u{1161}\u{11AB}\u{1100}\u{1173}\u{11AF}";

/// U+1EC7 LATIN SMALL LETTER E WITH CIRCUMFLEX AND DOT BELOW.
pub const E_MARKS_NFC: &str = "\u{1EC7}";

/// The same letter as a base plus two combining marks, in canonical order.
///
/// U+0065 e, then U+0323 DOT BELOW (ccc 220), then U+0302 CIRCUMFLEX (ccc
/// 230). Canonical order is ascending combining class, which is why the dot
/// comes first. The two marks the other way round still compose to U+1EC7,
/// because NFC reorders before composing, but that sequence is not NFD.
pub const E_MARKS_NFD: &str = "\u{0065}\u{0323}\u{0302}";

/// U+1E69 LATIN SMALL LETTER S WITH DOT BELOW AND DOT ABOVE.
pub const S_MARKS_NFC: &str = "\u{1E69}";

/// U+0073 s, then U+0323 DOT BELOW (ccc 220), then U+0307 DOT ABOVE (ccc 230).
///
/// Same shape as [`E_MARKS_NFD`] but a different base and different marks, so
/// a hard-coded special case cannot fix one and leave the other broken.
pub const S_MARKS_NFD: &str = "\u{0073}\u{0323}\u{0307}";

/// "caf" + U+00E9 E WITH ACUTE: NFC, and deliberately given no NFD twin.
pub const CAFE_NFC: &str = "caf\u{00E9}";

/// A control name whose NFC and NFD forms are identical.
pub const PLAIN_ASCII: &str = "plain";

/// Name of the ASCII directory the ancestor-scope fixtures pass on the command
/// line.
pub const SCAN_ROOT: &str = "scanroot";

/// Appends the fixture file extension to a stem.
pub fn txt(stem: &str) -> String {
    format!("{stem}.txt")
}

/// Appends the fixture symlink extension to a stem.
pub fn link(stem: &str) -> String {
    format!("{stem}.link")
}

/// Every name this suite creates on disk.
///
/// Drives the platform-safety check, so a name that is only illegal on Windows
/// cannot be added without a Linux run noticing.
pub fn fixture_names() -> Vec<String> {
    let stems = [
        GA_NFC,
        GA_NFD,
        HANGEUL_NFC,
        HANGEUL_NFD,
        E_MARKS_NFC,
        E_MARKS_NFD,
        S_MARKS_NFC,
        S_MARKS_NFD,
        CAFE_NFC,
        PLAIN_ASCII,
    ];
    let mut names: Vec<String> = Vec::new();
    for stem in stems {
        names.push(stem.to_string());
        names.push(txt(stem));
        names.push(link(stem));
    }
    names.push(format!("sub_{HANGEUL_NFD}"));
    names.push(SCAN_ROOT.to_string());
    names.push("target.txt".to_string());
    names.push("payload.txt".to_string());
    names.push("keep.txt".to_string());
    names
}

/// Renders a string as space-separated `U+XXXX` code points.
///
/// Failure messages print this instead of the characters, which are
/// indistinguishable in a terminal whichever form they are in.
pub fn codepoints(s: &str) -> String {
    s.chars()
        .map(|c| format!("U+{:04X}", c as u32))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Asserts that `s` is in NFC form.
pub fn assert_is_nfc(label: &str, s: &str) {
    assert_eq!(
        s.nfc().collect::<String>(),
        s,
        "constant {label} is not NFC; this source file was probably normalized \
         by an editor. code points = {}",
        codepoints(s)
    );
}

/// Asserts that `s` is in NFD form.
pub fn assert_is_nfd(label: &str, s: &str) {
    assert_eq!(
        s.nfd().collect::<String>(),
        s,
        "constant {label} is not NFD; this source file was probably normalized \
         by an editor. code points = {}",
        codepoints(s)
    );
}

/// Asserts that two constants are canonically equivalent but byte-distinct.
pub fn assert_twins(label: &str, nfc: &str, nfd: &str) {
    assert_is_nfc(label, nfc);
    assert_is_nfd(label, nfd);
    assert_ne!(
        nfc, nfd,
        "{label}: the two forms are byte-identical, so there is no twin"
    );
    assert_eq!(
        nfd.nfc().collect::<String>(),
        nfc,
        "{label}: the NFD form does not compose to the declared NFC form"
    );
    assert_eq!(
        nfc.nfd().collect::<String>(),
        nfd,
        "{label}: the NFC form does not decompose to the declared NFD form"
    );
}

/// What the filesystem under `env::temp_dir()` does to Unicode filenames.
///
/// Every variant is a positive identification. An I/O failure is not one of
/// them: [`fs_naming`] panics instead, so a broken environment can never be
/// mistaken for a filesystem that merely cannot host the fixture.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FsNaming {
    /// Twins coexist with the exact bytes written. Linux ext4/btrfs/xfs/tmpfs,
    /// Windows NTFS.
    KeepsTwins,
    /// One name round-trips, but canonically equivalent names share an entry.
    /// macOS APFS.
    NormalizationInsensitive,
    /// The stored name differs from the bytes written. macOS HFS+.
    Decomposes,
}

/// Classifies the filesystem the fixtures are built on.
///
/// Cached for the life of the process, so do not move TMPDIR mid-run.
pub fn fs_naming() -> FsNaming {
    static CACHE: OnceLock<FsNaming> = OnceLock::new();
    *CACHE.get_or_init(probe_fs_naming)
}

fn probe_fs_naming() -> FsNaming {
    let dir = tempfile::Builder::new()
        .prefix("nfdfix-probe-")
        .tempdir()
        .expect("probe: failed to create a temporary directory");
    let root = dir.path();
    let nfc = txt(GA_NFC);
    let nfd = txt(GA_NFD);

    fs::write(root.join(&nfc), "PROBE-NFC")
        .unwrap_or_else(|e| panic!("probe: failed to write the NFC name: {e}"));

    let after_first = entry_names(root);
    assert_eq!(
        after_first.len(),
        1,
        "probe: expected exactly one entry after writing one file, found {after_first:?}"
    );
    if after_first[0] != OsStr::new(nfc.as_str()) {
        return FsNaming::Decomposes;
    }

    fs::write(root.join(&nfd), "PROBE-NFD")
        .unwrap_or_else(|e| panic!("probe: failed to write the NFD name: {e}"));

    let after_second = entry_names(root);
    let nfc_body = fs::read_to_string(root.join(&nfc))
        .unwrap_or_else(|e| panic!("probe: failed to read back the NFC name: {e}"));

    match after_second.len() {
        // Comparing contents, not just the entry count, is what separates a
        // normalizing filesystem from a write that failed for its own reasons.
        2 => {
            let nfd_body = fs::read_to_string(root.join(&nfd))
                .unwrap_or_else(|e| panic!("probe: failed to read back the NFD name: {e}"));
            let both_present = after_second.iter().any(|n| n == OsStr::new(nfc.as_str()))
                && after_second.iter().any(|n| n == OsStr::new(nfd.as_str()));
            assert!(
                both_present && nfc_body == "PROBE-NFC" && nfd_body == "PROBE-NFD",
                "probe: two entries exist but they do not hold independent contents; \
                 entries = {after_second:?}, nfc body = {nfc_body:?}, nfd body = {nfd_body:?}"
            );
            FsNaming::KeepsTwins
        }
        1 => {
            assert_eq!(
                nfc_body, "PROBE-NFD",
                "probe: one entry remains but it does not hold the second write, \
                 so the collapse cannot be attributed to normalization insensitivity"
            );
            FsNaming::NormalizationInsensitive
        }
        n => panic!("probe: unclassifiable directory state, {n} entries: {after_second:?}"),
    }
}

/// Whether this environment can create symbolic links at all.
pub fn symlinks_available() -> bool {
    static CACHE: OnceLock<bool> = OnceLock::new();
    *CACHE.get_or_init(|| {
        let dir = tempfile::Builder::new()
            .prefix("nfdfix-symprobe-")
            .tempdir()
            .expect("probe: failed to create a temporary directory");
        fs::write(dir.path().join("probe-target.txt"), "PROBE")
            .expect("probe: failed to write the symlink target");
        match make_file_symlink(dir.path(), "probe.link", "probe-target.txt") {
            Ok(()) => fs::symlink_metadata(dir.path().join("probe.link"))
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false),
            Err(_) => false,
        }
    })
}

/// Creates a symbolic link to a regular file, relative to `dir`.
#[cfg(unix)]
pub fn make_file_symlink(dir: &Path, link_name: &str, target: &str) -> io::Result<()> {
    std::os::unix::fs::symlink(target, dir.join(link_name))
}

/// Creates a symbolic link to a regular file, relative to `dir`.
///
/// `symlink_file` rather than `symlink_dir` because Windows needs the link
/// kind up front and every fixture link here points at a regular file. Without
/// Developer Mode this fails with os error 1314.
#[cfg(windows)]
pub fn make_file_symlink(dir: &Path, link_name: &str, target: &str) -> io::Result<()> {
    std::os::windows::fs::symlink_file(target, dir.join(link_name))
}

/// Creates a symbolic link to a regular file, relative to `dir`.
#[cfg(not(any(unix, windows)))]
pub fn make_file_symlink(_dir: &Path, _link_name: &str, _target: &str) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "symbolic links are not supported on this platform",
    ))
}

// Fails loudly rather than skipping. A suite whose whole point is to be red
// must never come back all green on a filesystem that cannot host the fixture.
// A skip path belongs here when a macOS runner is actually wired up; until
// then it would be a branch nothing ever executes.
fn require(test: &str, ok: bool, why: &str, detail: String) {
    assert!(
        ok,
        "{test} cannot run here: {why}\n\
         {detail}\n\
         This suite reproduces data loss that requires NFC and NFD filenames to \
         coexist as two directory entries. That holds on Linux \
         (ext4/btrfs/xfs/tmpfs) and on Windows NTFS, but not on macOS APFS \
         (normalization-insensitive) or HFS+ (decomposes on store). Run the \
         suite on Linux or Windows, or point TMPDIR at a byte-preserving \
         volume such as an exFAT disk image."
    );
}

fn fs_detail() -> String {
    format!(
        "Detected filesystem naming behavior: {:?} (probed under {}).",
        fs_naming(),
        env::temp_dir().display()
    )
}

/// Requires a filesystem on which NFC and NFD twins are two distinct entries.
pub fn require_twins(test: &str) {
    require(
        test,
        fs_naming() == FsNaming::KeepsTwins,
        "the filesystem does not keep NFC and NFD twins as distinct entries",
        fs_detail(),
    );
}

/// Requires a filesystem that stores a filename as the bytes it was given.
pub fn require_exact_names(test: &str) {
    require(
        test,
        fs_naming() != FsNaming::Decomposes,
        "the filesystem rewrites filenames on store",
        fs_detail(),
    );
}

/// Requires an environment in which symbolic links can be created.
pub fn require_symlinks(test: &str) {
    require(
        test,
        symlinks_available(),
        "symbolic links cannot be created here",
        "On Windows, enable Developer Mode or run the tests from an elevated shell.".to_string(),
    );
}

/// Captured result of one `nfdfix` invocation.
pub struct Run {
    /// Exit status, or `None` if the process was killed by a signal.
    pub code: Option<i32>,
    /// Everything the run wrote to standard output.
    pub stdout: String,
    /// Everything the run wrote to standard error.
    pub stderr: String,
}

impl Run {
    /// Renders the whole run for embedding in an assertion message.
    pub fn debug(&self) -> String {
        format!(
            "--- exit code: {:?}\n--- stdout ---\n{}--- stderr ---\n{}",
            self.code, self.stdout, self.stderr
        )
    }

    /// How many renames the tool claimed to perform.
    pub fn renames(&self) -> usize {
        self.stdout
            .lines()
            .filter(|l| l.starts_with("renamed: "))
            .count()
    }

    /// How many renames the tool reported failing.
    pub fn rename_failures(&self) -> usize {
        self.failure_lines().len()
    }

    /// The reported failure lines.
    ///
    /// Assertions count the tool's own prefix rather than matching OS error
    /// text: this suite runs under a Korean locale, where `io::Error`'s
    /// `Display` goes through a localized `strerror`, and the underlying codes
    /// differ between Unix and Windows anyway.
    pub fn failure_lines(&self) -> Vec<&str> {
        self.stderr
            .lines()
            .filter(|l| l.starts_with("rename failed: "))
            .collect()
    }

    /// Source paths out of the reported failure lines.
    pub fn failure_source_paths(&self) -> Vec<&str> {
        self.failure_lines()
            .into_iter()
            .filter_map(|l| l.strip_prefix("rename failed: "))
            .filter_map(|rest| rest.split_once(" -> ").map(|(old, _)| old))
            .collect()
    }

    /// Final components of the sources the tool reported renaming.
    ///
    /// Taking `file_name()` keeps the comparison free of platform separators.
    /// No fixture name contains " -> ".
    pub fn renamed_source_names(&self) -> Vec<OsString> {
        self.stdout
            .lines()
            .filter_map(|l| l.strip_prefix("renamed: "))
            .filter_map(|rest| rest.split_once(" -> ").map(|(old, _)| old))
            .filter_map(|old| Path::new(old).file_name().map(|n| n.to_os_string()))
            .collect()
    }

    /// The error count out of the `N scanned, M renamed, K errors` summary.
    pub fn reported_errors(&self) -> Option<usize> {
        self.stdout
            .lines()
            .find(|l| l.trim_end().ends_with(" errors"))
            .and_then(|l| l.split_whitespace().nth(4))
            .and_then(|t| t.parse().ok())
    }
}

/// Runs the binary cargo built for this test run.
///
/// The crate has no library target and `main` calls `process::exit`, so a
/// subprocess is the only way in. It is also the only way to observe the exit
/// code, which is itself evidence: a clobbering rename returns `Ok`.
pub fn run_nfdfix(args: &[&OsStr]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_nfdfix"))
        .args(args)
        .output()
        .expect("failed to spawn the nfdfix binary");
    Run {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// Runs `nfdfix <target>`.
pub fn run_flat(target: &Path) -> Run {
    run_nfdfix(&[target.as_os_str()])
}

/// Runs `nfdfix -r <target>`.
pub fn run_recursive(target: &Path) -> Run {
    run_nfdfix(&[OsStr::new("-r"), target.as_os_str()])
}

/// Runs `nfdfix --dry-run <target>`.
pub fn run_dry(target: &Path) -> Run {
    run_nfdfix(&[OsStr::new("--dry-run"), target.as_os_str()])
}

/// Creates an isolated temporary directory to build a fixture in.
///
/// The ASCII assertion is load-bearing. A decomposed character in TMPDIR would
/// make the tool rename an ancestor of the fixture, which destroys the premise
/// of the ancestor-scope tests and perturbs everything else.
pub fn fixture() -> tempfile::TempDir {
    let dir = tempfile::Builder::new()
        .prefix("nfdfix-fixture-")
        .tempdir()
        .expect("failed to create a temporary fixture directory");
    let shown = dir
        .path()
        .to_str()
        .expect("the fixture path is not valid UTF-8");
    assert!(
        shown.is_ascii(),
        "the fixture path {shown} is not ASCII; point TMPDIR at an ASCII path \
         before running this suite"
    );
    dir
}

/// Creates a regular file with exactly the given name bytes.
pub fn write_file(dir: &Path, name: &str, contents: &str) {
    fs::write(dir.join(name), contents)
        .unwrap_or_else(|e| panic!("failed to create fixture file [{}]: {e}", codepoints(name)));
}

/// Creates a subdirectory with exactly the given name bytes.
pub fn make_dir(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    fs::create_dir(&p).unwrap_or_else(|e| {
        panic!(
            "failed to create fixture directory [{}]: {e}",
            codepoints(name)
        )
    });
    p
}

/// Sorted directory entry names, exactly as the filesystem stored them.
///
/// Deliberately not normalized. One NFC conversion here would make a twin pair
/// look like a single entry and every assertion in the suite would be vacuous.
pub fn entry_names(dir: &Path) -> Vec<OsString> {
    let mut names: Vec<OsString> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", dir.display()))
        .map(|e| e.expect("failed to read a directory entry").file_name())
        .collect();
    names.sort();
    names
}

/// Lists a directory with per-name code points, for failure messages.
pub fn describe_dir(dir: &Path) -> String {
    entry_names(dir)
        .iter()
        .map(|n| match n.to_str() {
            Some(s) => format!("  {s:?}  [{}]", codepoints(s)),
            None => format!("  {n:?}  [not valid UTF-8]"),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Asserts the filesystem kept every listed name as its own entry.
///
/// Precondition for every collision test. On a normalizing filesystem the
/// twins would collapse at creation time and the test would pass having
/// exercised nothing.
pub fn assert_distinct_entries(dir: &Path, expected: &[&str]) {
    let names = entry_names(dir);
    for want in expected {
        assert!(
            names.iter().any(|n| n == OsStr::new(*want)),
            "precondition failed: the filesystem did not preserve the name [{}].\n\
             actual entries:\n{}",
            codepoints(want),
            describe_dir(dir)
        );
    }
    let unique: BTreeSet<&&str> = expected.iter().collect();
    assert_eq!(
        unique.len(),
        expected.len(),
        "the expected-name list itself contains duplicates"
    );
    assert!(
        names.len() >= expected.len(),
        "precondition failed: expected at least {} distinct entries, found {}:\n{}",
        expected.len(),
        names.len(),
        describe_dir(dir)
    );
}

/// Every regular-file body found anywhere under `root`.
///
/// Collision tests assert on surviving data, never on the names the tool
/// chose. A fix has to pick some collision-avoidance scheme and that choice is
/// not this branch's to make; "both payloads still exist" stays true whatever
/// it settles on.
pub fn surviving_contents(root: &Path) -> BTreeSet<String> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| fs::read_to_string(e.path()).ok())
        .collect()
}

/// Everything under `root` as `kind relative/path body`.
///
/// Backs the guards that assert nothing changed. Components join with '/' so
/// the value does not depend on the platform separator.
pub fn snapshot(root: &Path) -> BTreeSet<String> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path() != root)
        .map(|e| {
            let rel = e
                .path()
                .strip_prefix(root)
                .expect("walkdir yielded a path outside the root");
            let name = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            let ft = e.file_type();
            let (kind, body) = if ft.is_symlink() {
                (
                    "symlink",
                    fs::read_link(e.path())
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                )
            } else if ft.is_dir() {
                ("dir", String::new())
            } else {
                (
                    "file",
                    fs::read_to_string(e.path()).unwrap_or_else(|_| "<unreadable>".to_string()),
                )
            };
            format!("{kind} {name} {body}")
        })
        .collect()
}
