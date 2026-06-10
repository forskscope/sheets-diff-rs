use std::path::Path;
use std::{env, process};

use sheets_diff::core::diff::Diff;
use sheets_diff::core::unified_format::unified_diff;

fn main() {
    let args: Vec<String> = env::args().collect();
    let (old_filepath, new_filepath) = parse_args(&args);

    match Diff::try_new(old_filepath, new_filepath) {
        Ok(diff) => print!("{}", unified_diff(&diff).format()),
        Err(err) => {
            eprintln!("sheets-diff: {err}");
            process::exit(2);
        }
    }
}

fn parse_args(args: &[String]) -> (&str, &str) {
    if args.len() != 3 {
        eprintln!("Usage: {} <old-file> <new-file>", args[0]);
        process::exit(1);
    }

    let old_filepath = args[1].as_str();
    let new_filepath = args[2].as_str();

    if !Path::new(old_filepath).exists() || !Path::new(new_filepath).exists() {
        eprintln!("sheets-diff: one or both file paths do not exist");
        process::exit(1);
    }

    (old_filepath, new_filepath)
}
