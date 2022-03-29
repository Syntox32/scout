use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};
use std::ops::Deref;
use std::path::PathBuf;
use std::{fs, path::Path};

use walkdir::WalkDir;

use crate::Result;

pub fn load_from_file<P>(path: P) -> Result<String>
where
    P: AsRef<Path>,
{
    Ok(fs::read_to_string(path)?)
}

/// Example copied from the Rust By Example book
/// https://doc.rust-lang.org/rust-by-example/std_misc/file/read_lines.html
///
pub fn read_lines<P>(filename: P) -> Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
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

pub fn collect_files_matching(
    path: &Path,
    filename_match: Vec<&'static str>,
) -> HashMap<String, PathBuf> {
    let mut matches: HashMap<String, PathBuf> = HashMap::new();

    let mut _count = 0;
    for entry in WalkDir::new(path).follow_links(false) {
        if let Ok(e) = entry {
            let p = e.path().to_path_buf();
            if p.is_file() {
                if let Some(filename) = p.file_name() {
                    let target = filename.to_str().unwrap();
                    for f_match in filename_match.iter() {
                        if target == *f_match {
                            if let None = matches.get(*f_match) {
                                matches.insert(f_match.to_string(), p.clone());
                            } else {
                                warn!("Another entry was made for the file: {} - in collect_files_matching", &f_match);
                            }
                        }
                    }
                }
            }
        }
    }

    matches
}

pub fn collect_files(path: &Path, ending: &'static str) -> Box<Vec<PathBuf>> {
    let mut all_files: Box<Vec<PathBuf>> = Box::new(vec![]);

    let mut _count = 0;
    for entry in WalkDir::new(path).follow_links(false) {
        // if count > 200 {
        //     return all_files;
        // }
        // count += 1;
        if let Ok(e) = entry {
            let p = e.path().to_path_buf();
            if p.is_file() {
                if let Some(filename) = p.file_name() {
                    if filename.to_str().unwrap().ends_with(ending) {
                        all_files.push(p);
                        // println!("{} size of all_files vec: {}", count, all_files.len());
                    }
                }
            }
        }
    }

    // let mut count = 0;
    // for entry in WalkDir::new(path)
    //     .into_iter()
    //     .filter_map(|e| e.ok())
    //     .filter(|e| e.path().is_file())
    //     .filter(|f| f.path().file_name().unwrap().to_str().unwrap().ends_with(ending))
    // {

    //     all_files.push(Box::new(entry));
    // }

    all_files
}

pub fn _stack_size<T>(v: &Vec<T>) -> usize {
    let mut s: usize = 0;
    for entry in v {
        s += std::mem::size_of_val(entry);
    }
    s
}

pub fn get_last_attr(full_identifier: &str) -> &str {
    match full_identifier
        .split('.')
        .collect::<Vec<&str>>()
        .iter()
        .last()
    {
        Some(last) => last,
        None => full_identifier,
    }
}

#[allow(unused)]
#[derive(PartialEq, Debug)]
pub enum PackageType {
    Wheel,
    Zip,
}

#[allow(unused)]
fn detect_package_type<P>(path: P) -> Option<PackageType>
where
    P: AsRef<Path>,
{
    let mut any_is_dist_info: Option<bool> = None;

    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap();

        if name.ends_with(".dist-info") {
            any_is_dist_info = Some(true);
        } else if name.ends_with("setup.py") {
            any_is_dist_info = Some(false);
        }
    }

    match any_is_dist_info {
        Some(true) => Some(PackageType::Wheel),
        Some(false) => Some(PackageType::Zip),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::{detect_package_type, PackageType};

    #[test]
    fn test_detect_package() {
        let test_wheel = "../../dataset/top/unpacked/Flask-2.0.2-py3-none-any.whl";
        let test_zip = "../../dataset/top/unpacked/termcolor-1.1.0.tar.gz/termcolor-1.1.0";
        let test_none = "../../dataset/top/unpacked/termcolor-1.1.0.tar.gz";

        assert_eq!(detect_package_type(test_wheel), Some(PackageType::Wheel));
        assert_eq!(detect_package_type(test_zip), Some(PackageType::Zip));
        assert_eq!(detect_package_type(test_none), None);
    }
}

pub mod ast {
    use ron::value;
    use rustpython_parser::ast::{Expression, ExpressionType, Keyword, Operator, StringGroup};

    use crate::visitors::VariableType;

    pub fn try_to_string(expr: &Expression) -> Option<String> {
        match &expr.node {
            // ExpressionType::Call { .. } => self.resolve_call(arg),
            ExpressionType::Binop { a, op, b } => resolve_binop(a, b, op),
            ExpressionType::String { value } => resolve_string_group(&value),
            ExpressionType::Identifier { name } => Some(name.to_owned()),
            _ => None,
        }
    }

    pub fn is_identifier(expr: &Expression) -> bool {
        match &expr.node {
            ExpressionType::Identifier { .. } => true,
            _ => false,
        }
    }

    pub fn try_identifier(expr: &Expression) -> Option<String> {
        match &expr.node {
            ExpressionType::Identifier { name } => Some(name.to_owned()),
            _ => None,
        }
    }

    pub fn resolve_args(args: &[Expression]) -> Vec<Option<VariableType>> {
        // trace!("{:#?}", args);
        let results: Vec<Option<VariableType>> = args
            .iter()
            .map(|arg| {
                // try_to_string(arg)
                if let Some(str_val) = try_to_string(&arg) {
                    let val = if is_identifier(&arg) {
                        VariableType::Identifier(str_val)
                    } else {
                        VariableType::Str(str_val)
                    };
                    Some(val)
                } else {
                    None
                }
            })
            .collect();
        results
    }

    pub fn resolve_kwargs(args: &[Keyword]) -> Vec<(Option<String>, Option<VariableType>)> {
        // trace!("{:#?}", args);
        let results: Vec<(Option<String>, Option<VariableType>)> = args
            .iter()
            .map(|arg| {
                if let Some(str_val) = try_to_string(&arg.value) {
                    let val = if is_identifier(&arg.value) {
                        VariableType::Identifier(str_val)
                    } else {
                        VariableType::Str(str_val)
                    };
                    (arg.name.clone(), Some(val))
                } else {
                    (arg.name.clone(), None)
                }
            })
            .collect();
        results
    }

    pub fn resolve_string_group(value: &StringGroup) -> Option<String> {
        match value {
            StringGroup::Constant { value } => Some(value.to_owned()),
            _ => None, //String::from("unsupported by resolve_string_group") }
        }
    }

    pub fn resolve_binop(
        a: &Box<Expression>,
        b: &Box<Expression>,
        op: &Operator,
    ) -> Option<String> {
        let aa = try_to_string(a)?;
        let bb = try_to_string(b)?;
        do_binop(aa, bb, op)
    }

    pub fn do_binop(a: String, b: String, op: &Operator) -> Option<String> {
        // trace!("doing bin op: {} {:?} {}", a, op, b);
        match op {
            Operator::Add => Some(format!("{}{}", a.to_owned(), b.to_owned())),
            _ => None, //format!("{} binop {}", a, b)
        }
    }
}
