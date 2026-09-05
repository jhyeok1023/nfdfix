mod cli;
mod normalize;
mod rename;
mod scanner;

use clap::Parser;
use cli::Cli;
use rename::Outcome;
use std::{collections::HashSet, path::PathBuf};

fn main() {
    let args = Cli::parse();
    let entries = scanner::scan(&args.path, args.recursive);

    let mut errors = 0;
    let mut renamed_count = 0;
    let mut skipped_count = 0;
    let total = entries.len();
    // Components already handled that now carry their NFC name, so descendants
    // must be rebuilt against it.
    let mut processed: HashSet<PathBuf> = HashSet::new();
    // Components already handled that kept the name they had. Rebuilding a
    // descendant against the NFC form would name a path that was never
    // created; holding them here also keeps a skipped directory from being
    // re-reported once per descendant.
    let mut held: HashSet<PathBuf> = HashSet::new();

    for entry in entries {
        let mut current = PathBuf::new();

        for component in entry.components() {
            current.push(component);

            if held.contains(&current) {
                continue;
            }

            if processed.contains(&current) {
                current = normalize::to_nfc(&current);
                continue;
            }

            let normalized = normalize::to_nfc(&current);
            if current != normalized {
                match rename::apply(&current, &normalized, args.dry_run) {
                    Outcome::Renamed => {
                        renamed_count += 1;
                        processed.insert(current.clone());
                        current = normalized;
                    }
                    Outcome::Failed => {
                        errors += 1;
                        processed.insert(current.clone());
                        current = normalized;
                    }
                    Outcome::Skipped => {
                        skipped_count += 1;
                        held.insert(current.clone());
                    }
                }
            }
        }
    }

    if args.dry_run {
        println!(
            "\n[dry-run] {} scanned, {} would be renamed, {} would be skipped",
            total, renamed_count, skipped_count
        );
    } else {
        // The summary line's shape is a contract: the tests read the error
        // count out of it by position. The skipped count goes on its own line.
        println!(
            "\n{} scanned, {} renamed, {} errors",
            total, renamed_count, errors
        );
        println!("{} skipped (name conflict)", skipped_count);
    }

    if renamed_count == 0 && errors == 0 && skipped_count == 0 {
        println!("\nNo files were renamed or errors occurred.");
    }

    std::process::exit(if errors > 0 {
        1
    } else if skipped_count > 0 {
        2
    } else {
        0
    });
}
