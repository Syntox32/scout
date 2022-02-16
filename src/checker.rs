use crate::sourcefile::SourceFile;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

pub struct Checker {
    suspicious_functions: HashSet<String>,
    suspicious_imports: HashSet<String>,
}

#[derive(Debug)]
pub struct CheckResult {
    pub suspicious_functions: i32,
    pub suspicious_imports: i32,
}

impl Checker {
    pub fn new() -> Checker {
        let funcs = Checker::create_hashset("conf/functions.txt");
        let mods = Checker::create_hashset("conf/modules.txt");

        Checker {
            suspicious_functions: funcs,
            suspicious_imports: mods,
        }
    }

    pub fn check(&self, source_file: &SourceFile) -> CheckResult {
        let mut sus_funcs: i32 = 0;
        let mut sus_mods: i32 = 0;

        source_file.function_calls.iter().for_each(|s| {
            if self.suspicious_functions.contains(s.0) {
                sus_funcs += 1;
            }
        });

        source_file.imports.iter().for_each(|s| {
            if self.suspicious_imports.contains(s) {
                sus_mods += 1;
            }
        });

        CheckResult {
            suspicious_functions: sus_funcs,
            suspicious_imports: sus_mods,
        }
    }

    fn create_hashset<T>(filename: T) -> HashSet<String>
    where
        T: AsRef<Path>,
    {
        let f = File::open(filename).unwrap();
        let lines: HashSet<String> = io::BufReader::new(f)
            .lines()
            .map(|l| l.unwrap().replace("\n", "").to_lowercase())
            .collect();
        lines
    }
}
