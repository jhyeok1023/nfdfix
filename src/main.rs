mod cli;
mod normalize;
mod rename;
mod scanner;

use clap::Parser;
use cli::Cli;
use std::{collections::HashSet, path::PathBuf};

fn main() {
    let args = Cli::parse();
    let entries = scanner::scan(&args.path, args.recursive);

    let mut errors = 0;
    let mut renamed_count = 0;
    let total = entries.len();
    let mut processed: HashSet<PathBuf> = HashSet::new();

    for entry in entries {
        let mut current = PathBuf::new();

        for component in entry.components() {
            current.push(component);

            if processed.contains(&current) {
                current = normalize::to_nfc(&current);
                continue;
            }

            let normalized = normalize::to_nfc(&current);
            if current != normalized {
                if rename::apply(&current, &normalized, args.dry_run) {
                    renamed_count += 1;
                } else {
                    errors += 1;
                }

                processed.insert(current.clone());
                current = normalized;
            }
        }
    }

    if args.dry_run {
        println!(
            "\n[dry-run] {} scanned, {} would be renamed",
            total, renamed_count
        );
    } else {
        println!(
            "\n{} scanned, {} renamed, {} errors",
            total, renamed_count, errors
        )
    }

    if renamed_count == 0 && errors == 0 {
        println!("\nNo files were renamed or errors occurred.");
    }

    std::process::exit(if errors > 0 { 1 } else { 0 });
}
