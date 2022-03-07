use std::io::Result;
use std::ops::Deref;
use std::{fs, path::Path};

use walkdir::{DirEntry, WalkDir};

pub fn load_from_file<P>(path: &P) -> Result<String>
where
    P: AsRef<Path>,
{
    fs::read_to_string(path)
}

pub fn _indent(strings: &[String], indent: String) -> Vec<String> {
    let mut res: Vec<String> = Vec::new();
    for s in strings {
        let mut ident = indent.clone();
        ident.push_str(&s.clone());
        let what = ident.deref().to_owned();
        res.push(what);
    }
    res
}

pub fn format_empty_arg(opt: &Option<String>) -> String {
    opt.to_owned().unwrap_or_else(|| String::from("*"))
}

pub fn collect_files(path: &Path) -> Vec<DirEntry> {
    let mut all_files: Vec<DirEntry> = vec![];

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
    {
        // println!("{}", entry.path().display());
        all_files.push(entry);
    }

    all_files
}
