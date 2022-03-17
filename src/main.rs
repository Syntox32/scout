use std::path::{Path, PathBuf};
use std::str::FromStr;

use clap::Parser;
use scout::{JsonResult, Package, RuleManager};

#[macro_use]
extern crate log;

fn analyse_package(
    path: &Path,
    rules: &RuleManager,
    threshold: f64,
    json_output: bool,
    show_all_override: bool,
) {
    let mut package = Package::new(path, rules, threshold);
    if let Some(result) = package.analyse(show_all_override) {
        if json_output {
            let mut out = JsonResult::new();
            for mut res in result {
                out.add(&mut res);
            }
            println!("{}", out.get_json());
        } else {
            for res in result {
                println!("{}", res.message);
            }
        }
    } else {
        warn!("something happend analysing the package");
    }
}

fn analyse_single(
    path: &str,
    rm: &RuleManager,
    threshold: f64,
    json_output: bool,
    show_all_override: bool,
) {
    let path = PathBuf::from_str(path).unwrap();
    let package = Package::new(&path, &rm, threshold);
    if let Some(mut result) = package.analyse_single(&path, show_all_override) {
        // result.density_evaluator._plot();
        if json_output {
            let mut out = JsonResult::new();
            out.add_with_fields(&mut result);
            println!("{}", out.get_json());
        } else {
            println!("{}", result.message);
        }
    }
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

    #[clap(short, long)]
    threshold: Option<f64>,

    #[clap(short, long)]
    json: Option<bool>,

    #[clap(short, long)]
    rules: Option<String>,

    #[clap(short, long)]
    all: Option<bool>,
}

fn main() {
    pretty_env_logger::init();
    let args = Args::parse();
    let rule_path = args
        .rules
        .unwrap_or(RuleManager::DEFAULT_RULE_FILE.to_string());
    let rm = RuleManager::new(&rule_path);

    let json = args.json.unwrap_or(false);
    let show_all_override = args.all.unwrap_or(false);

    if show_all_override {
        warn!("Show all bulletins override is enabled.");
    }

    match args.file {
        Some(path) => {
            trace!("Analysing single file: '{}'", &path.as_str());
            analyse_single(
                path.as_str(),
                &rm,
                args.threshold.unwrap_or(0f64),
                json,
                show_all_override,
            );
        }
        None => match args.package {
            Some(package) => {
                trace!("Analysing package: '{}'", &package.as_str());
                let pkg = Package::locate_package(package.as_str());
                // println!("{:?}", &pkg);
                if let Some(path) = pkg {
                    debug!("Detected package: '{:?}'", &path);
                    analyse_package(
                        &path,
                        &rm,
                        args.threshold.unwrap_or(0f64),
                        json,
                        show_all_override,
                    );
                } else {
                    debug!("could not find path")
                }
            }
            None => {
                // TODO: Rewrite this to use the clap App functionality so we can control required arguments.
                eprintln!("Error: Either a file or a package has to be supplied as arguments.");
            }
        },
    }
}
