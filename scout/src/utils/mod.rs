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

pub fn collect_files(path: &Path) -> Box<Vec<DirEntry>> {
    let mut all_files: Box<Vec<DirEntry>> = Box::new(vec![]);

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
    {
        all_files.push(entry);
    }

    all_files
}

pub fn stack_size<T>(v: &Vec<T>) -> usize {
    let mut s: usize = 0;
    for entry in v {
        s += std::mem::size_of_val(entry);
    }
    s
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
