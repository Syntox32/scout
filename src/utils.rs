use std::fs;
use std::io::Result;
use std::ops::Deref;
use std::path::PathBuf;

pub fn load_from_file(path: &PathBuf) -> Result<String> {
    fs::read_to_string(path)
}

pub fn indent(strings: &Vec<String>, indent: String) -> Vec<String> {
    let mut res: Vec<String> = Vec::new();
    for s in strings {
        let mut ident = indent.clone();
        ident.push_str(&s.clone());
        let what = ident.deref().to_owned();
        res.push(what);
    }
    res
}
