mod cli;
mod normalize;
mod rename;
mod scanner;

use clap::Parser;
use cli::Cli;

fn main() {
    let args = Cli::parse();

    let files = scanner::scan(&args.path, args.recursive);

    for file in files {
        let new_name = normalize::to_nfc(&file);

        if file != new_name {
            rename::apply(&file, &new_name, args.dry_run);
        }
    }
}
