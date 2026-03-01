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

## Dependencies and Licenses

- [clap](https://crates.io/crates/clap) - MIT or Apache-2.0
- [walkdir](https://crates.io/crates/walkdir) - Unlicense or MIT
- [unicode-normalization](https://crates.io/crates/unicode-normalization) - MIT or Apache-2.0

## License
This project is licensed under [BSD-3-Clause](LICENSE).
