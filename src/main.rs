use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

mod ast_visitor;
mod checker;
mod package;
mod sourcefile;
mod utils;

fn analyse(path: &PathBuf) {
    match sourcefile::SourceFile::load(path) {
        Ok(sf) => {
            println!("Path: {}", &sf.source_path.as_os_str().to_str().unwrap());
            //println!("Source: {}", &sf.source);

            println!("Imports:\n{}", sf.display_list(&sf.imports));
            println!("Functions:\n{}", sf.display_functions());

            let checker = checker::Checker::new();
            let check_result = checker.check(&sf);
            println!("{:?}", check_result);
        }
        Err(err) => {
            println!("Error: {}", err);
        }
    }
}

#[allow(unused)]
fn analyse_all() {
    let paths = fs::read_dir("../ast_experiment/tests").unwrap();

    for path in paths {
        let p = path.expect("could not get path from direntry").path();
        if p.is_file() {
            let abs = p
                .canonicalize()
                .expect("could not convert path to absolute path");
            analyse(&abs);
        }
    }
}

fn analyse_single(path: &str) {
    let p = PathBuf::from_str(path).unwrap();
    analyse(&p);
}

fn main() {
    //analyse_all()
    analyse_single("../ast_experiment/tests/test-7.py");
}
