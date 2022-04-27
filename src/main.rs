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

    #[clap(long)]
    config_json: Option<String>,

    #[clap(long)]
    config: Option<String>,

    #[clap(short, long)]
    all: Option<bool>,

    #[clap(long)]
    fields: Option<bool>,
}

fn main() -> Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();
    let show_all_override = args.all.unwrap_or(false);
    let include_fields = args.fields.unwrap_or(false);

    if show_all_override {
        warn!("Show all bulletins override is enabled.");
    }

    let mut engine = Engine::new()
        .set_show_all(show_all_override)
        .set_threshold(args.threshold.unwrap_or(0f64))
        .set_rule_path(args.rules);

    if let Some(config_json) = args.config_json {
        engine.set_config(config_json);
    }

    match args.file {
        Some(path) => match engine.analyse_file(path.as_str()) {
            Ok(results) => match args.json {
                Some(_) => {
                    let result = if include_fields {
                        results.to_json_with_fields()
                    } else {
                        results.to_json()
                    };

                    println!("{}", result);
                    Ok(())
                }
                None => {
                    println!("{}", results.to_string());
                    Ok(())
                }
            },
            Err(err) => Err(format!("Failed to analyse file: {}", err.to_string()).into()),
        },
        None => match args.package {
            Some(package) => match engine.analyse_package(package.as_str()) {
                Ok(results) => match args.json {
                    Some(_) => {
                        if include_fields {
                            warn!("with_fields is only supported for single files only.");
                        }
                        println!("{}", results.to_json());
                        Ok(())
                    }
                    None => {
                        println!("{}", results.to_string());
                        Ok(())
                    }
                },
                Err(err) => {
                    info!("error debugging re {:?}", err);
                    Err(format!("Failed to analyse package: {}", err.to_string()).into())
                }
            },
            None => {
                // TODO: Rewrite this to use the clap App functionality so we can control required arguments.
                Err("Error: Either a file or a package has to be supplied as arguments.".into())
            }
        },
    }
}
