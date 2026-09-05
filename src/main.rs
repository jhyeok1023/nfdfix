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
    // Components already renamed, so a descendant has to be rebuilt against
    // the NFC name they now carry.
    let mut processed: HashSet<PathBuf> = HashSet::new();
    // Components that kept the name they had, because the rename was skipped
    // over a conflict or failed. A descendant has to be rebuilt against that
    // name: the NFC form was never created, so normalizing it into the path
    // would aim the descendant's rename at whatever entry does hold the NFC
    // name. Holding them here also keeps one undone directory from being
    // re-reported once per descendant.
    let mut held: HashSet<PathBuf> = HashSet::new();

    for entry in entries {
        // Rebuilt one component at a time, so `current` always names the path
        // as it stands on disk. Only the component just pushed is a rename
        // candidate; every ancestor above it has been decided already, which
        // is why the normalization below is leaf-only.
        let mut current = PathBuf::new();

        for component in entry.components() {
            current.push(component);

            if held.contains(&current) {
                continue;
            }

            if processed.contains(&current) {
                current = normalize::leaf_to_nfc(&current);
                continue;
            }

            let normalized = normalize::leaf_to_nfc(&current);
            if current != normalized {
                match rename::apply(&current, &normalized, args.dry_run) {
                    Outcome::Renamed => {
                        renamed_count += 1;
                        processed.insert(current.clone());
                        current = normalized;
                    }
                    // A failed rename left the name exactly as it was, so it
                    // is held for the same reason a skipped one is. Advancing
                    // past it would rebuild every descendant against a
                    // directory that was never created.
                    Outcome::Failed => {
                        errors += 1;
                        held.insert(current.clone());
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

    // 3, not 2: clap exits 2 on a usage error, before any of this runs. A
    // script reading 2 as "a rename was skipped" would read a mistyped flag as
    // a name conflict on a tree the tool never touched. Overriding clap's code
    // instead would put the contract in a dependency's hands, where a later
    // clap release can move it.
    std::process::exit(if errors > 0 {
        1
    } else if skipped_count > 0 {
        3
    } else {
        0
    });
}
