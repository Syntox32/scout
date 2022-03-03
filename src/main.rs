use std::path::{Path, PathBuf};
use std::str::FromStr;

mod ast;
mod evaluator;
mod package;
mod source;
mod utils;
mod visitors;

use package::Package;

use crate::evaluator::RuleManager;
use clap::Parser;

#[macro_use]
extern crate log;

fn analyse_package(path: &Path, rules: &RuleManager) {
    let mut package = Package::new(path, rules);
    package.analyse();
}

fn analyse_single(path: &str, rm: &RuleManager) {
    let path = PathBuf::from_str(path).unwrap();
    let package = Package::new(&path, &rm);
    package.analyse_single(&path);
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to a single Python file we want to analyse
    #[clap(short, long)]
    file: Option<String>,

    /// The path to the package to analyse
    #[clap(short, long)]
    package: Option<String>,
}

fn main() {
    pretty_env_logger::init();
    let args = Args::parse();
    let rm = RuleManager::new();

    match args.file {
        Some(path) => {
            trace!("Analysing single file: '{}'", &path.as_str());
            analyse_single(path.as_str(), &rm);
        }
        None => match args.package {
            Some(package) => {
                trace!("Analysing package: '{}'", &package.as_str());
                let pkg = Package::locate_package(package.as_str());
                if let Some(path) = pkg {
                    debug!("Detected package: '{:?}'", &path);
                    analyse_package(&path, &rm);
                }
            }
            None => {
                // TODO: Rewrite this to use the clap App functionality so we can control required arguments.
                eprintln!("Error: Either a file or a package has to be supplied as arguments.");
            }
        },
    }
}
