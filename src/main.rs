use clap::Parser;
use scout::{Engine, Result};

#[macro_use]
extern crate log;

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

fn main() -> Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();
    let show_all_override = args.all.unwrap_or(false);

    if show_all_override {
        warn!("Show all bulletins override is enabled.");
    }

    let engine = Engine::new()
        .set_show_all(show_all_override)
        .set_threshold(args.threshold.unwrap_or(0f64))
        .set_rule_path(
            args.rules
                .unwrap_or_else(|| Engine::get_default_rule_file())
                .as_str(),
        );

    match args.file {
        Some(path) => match engine.analyse_file(path.as_str()) {
            Ok(results) => match args.json {
                Some(_) => {
                    println!("{}", results.to_json());
                    Ok(())
                },
                None => {
                    println!("{}", results.to_string());
                    Ok(())
                }
            }
            Err(err) => Err(format!("Failed to analyse file: {}", err.to_string()).into()),
        },
        None => match args.package {
            Some(package) => match engine.analyse_package(package.as_str()) {
                Ok(results) => match args.json {
                    Some(_) => {
                        println!("{}", results.to_json());
                        Ok(())
                    },
                    None => {
                        println!("{}", results.to_string());
                        Ok(())
                    }
                },
                Err(err) => Err(format!("Failed to analyse package: {}", err.to_string()).into()),
            },
            None => {
                // TODO: Rewrite this to use the clap App functionality so we can control required arguments.
                Err("Error: Either a file or a package has to be supplied as arguments.".into())
            }
        },
    }
}
