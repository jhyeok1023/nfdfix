# nfdfix
`nfdfix` is a simple CLI tool to normalize **NFD-formatted filenames** to **NFC**.  

## Installation
### Build from source
```bash
git clone https://github.com/zenru1023/nfdfix.git
cd nfdfix
cargo build --release
```
Then binary will be available at:
```bash
./target/release/nfdfix
```

#### Optional: Install to Cargo bin path
```bash
cargo install --path .
```
Then you can run:
```bash
nfdfix <path>
```

## Usage
```bash
# Normalize a single file
nfdfix myfile.txt

# Normalize all files in a directory (top-level only)
nfdfix mydir

# Recursive scan for all subdirectories
nfdfix -r mydir

# Preview changes without renaming
nfdfix --dry-run mydir
```

## Exit codes
`nfdfix` is meant to be driven from scripts, so it distinguishes a run that
failed from a run that refused to act.

| Code | Meaning |
|---|---|
| `0` | Nothing went wrong. Every NFD name that needed normalizing was renamed. |
| `1` | At least one rename was attempted and failed. |
| `2` | Nothing failed, but at least one rename was skipped because the NFC name was already taken by another entry. |

A name conflict is never resolved by overwriting: the existing entry is left
alone, the NFD name stays as it is, and a `skipped:` line naming both paths is
written to standard error.

`--dry-run` reports the same codes, but it is not a complete pre-flight check.
It inspects the filesystem as it stands, and it performs no renames, so it
cannot see a conflict that would only appear underneath a directory the real
run renames first. A `--dry-run` exit of `0` is not a promise that the real run
will not report `2`.

Errors outrank skips. A run with both reports `1`.

## Dependencies and Licenses

- [clap](https://crates.io/crates/clap) - MIT or Apache-2.0
- [walkdir](https://crates.io/crates/walkdir) - Unlicense or MIT
- [unicode-normalization](https://crates.io/crates/unicode-normalization) - MIT or Apache-2.0

## License
This project is licensed under [BSD-3-Clause](LICENSE).
