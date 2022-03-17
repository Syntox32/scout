use crate::{
    evaluator::{Evaluator, EvaluatorResult, RuleManager},
    source::SourceFile,
    utils::collect_files,
};
use colored::Colorize;
use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

#[derive(PartialEq, Debug)]
pub enum PackageType {
    Wheel,
    Zip,
}

pub struct Package<'a> {
    pub path: PathBuf,
    checker: Evaluator<'a>,
    threshold: f64,
}

impl<'a> Package<'a> {
    pub fn new(path: &Path, rules: &'a RuleManager, threshold: f64) -> Self {
        Self {
            path: path.to_owned(),
            checker: Evaluator::new(rules.get_rule_sets()),
            threshold,
        }
    }

    fn get_sources(&self) -> Vec<SourceFile> {
        trace!("Ackquiring sources...");
        let files = collect_files(&self.path);
        let mut sources: Vec<SourceFile> = vec![];
        for file in files {
            trace!("Loaded file: {:?}", &file.path().file_name());
            // file.path().ends_with(".py") did not work. maybe it's a bug?
            if file
                .path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .ends_with(".py")
            {
                trace!("Found python file: {:?}", file);
                let p = file.path().to_path_buf();

                match SourceFile::load(&p) {
                    Ok(source) => sources.push(source),
                    Err(err) => error!(
                        "Parse or load error '{}' in file '{}'",
                        err,
                        p.as_path().as_os_str().to_str().unwrap()
                    ),
                };
            } else {
                trace!("file did not end with .py: {:?}", file);
            }
        }
        sources
    }

    pub fn analyse(&mut self, show_all_override: bool) -> Option<Vec<EvaluatorResult>> {
        // println!("package: analyzing");
        let sources = self.get_sources();
        let mut results: Vec<EvaluatorResult> = vec![];
        for source in sources {
            let path = &source.get_path().to_owned();
            if let Some(result) = self.evaluate_source(source, show_all_override) {
                results.push(result);
            } else {
                warn!("Could not evaluate source: {}", path);
            }
        }

        Some(results)
    }

    pub fn analyse_single(&self, path: &Path, show_all_override: bool) -> Option<EvaluatorResult> {
        match SourceFile::load(&path) {
            Ok(source) => {
                if let Some(result) = self.evaluate_source(source, show_all_override) {
                    return Some(result);
                }
                None
            }
            Err(err) => {
                error!(
                    "Parse or load error '{}' in file '{}'",
                    err,
                    path.as_os_str().to_str().unwrap()
                );
                None
            }
        }
    }

    fn evaluate_source(&self, source: SourceFile, show_all_override: bool) -> Option<EvaluatorResult> {
        let mut eval_result = self.checker.check(source, show_all_override);
        let mut message: String = String::from("");

        trace!("Bulletins before purge: {:?}", eval_result.bulletins());

        if !(eval_result.found_anything()
            && eval_result.any_bulletins_over_threshold(self.threshold))
        {
            trace!(
                "File was skipped because no bulletins showed: {}",
                eval_result.source.get_path()
            );
            return None;
        }

        eval_result.display_functionality();
        debug!(
            "Functions found in source file: [{}]",
            eval_result.source._display_functions()
        );
        debug!(
            "Imports found in source file: [{}]",
            eval_result.source._display_imports()
        );
        trace!(
            "Bulletins by hotspots: {:?}",
            eval_result.bulletins_by_hotspot()
        );

        message += format!("Location: {}\n", eval_result.source.get_path()).as_str();
        let mut first = true;
        for (group, hotspot) in eval_result.bulletins_by_hotspot() {
            if first {
                first = false;
            }
            debug!("Current hotspot: {:?}", hotspot);
            let f = eval_result.get_uniq_functionality(&group);
            debug!("Functionality for group: {:?}", f);

            let hotspot_code: Vec<(usize, &str)> = hotspot.get_code(&eval_result.source);

            let mut display = false;
            let mut output: Vec<String> = vec![];

            #[allow(unused_assignments)]
            let mut turned_yellow: bool = false;
            for (line, line_string) in hotspot_code {
                turned_yellow = false;

                let line = line + 1;
                output.push(format!("{:>3}| {}", line, line_string.to_owned().dimmed()));

                for bulletin in group.iter() {
                    // add one cause its a 0 based index because of enumerate
                    if (bulletin.line() == line && hotspot.peak() >= bulletin.threshold)
                        && hotspot.peak() > self.threshold
                    {
                        if !display {
                            display = true;
                        }

                        if !turned_yellow {
                            output.pop();
                            output.push(format!(
                                "{:>3}| {}",
                                line,
                                line_string.to_owned().bright_yellow()
                            ));
                            turned_yellow = true;
                        }

                        let mut lw = line.to_string().len();
                        if lw < 3 {
                            lw = 3;
                        }
                        lw += 1;
                        let mut pad: String = String::from("");
                        for _ in 0..(lw + bulletin.col() + 1) {
                            pad += " ";
                        }
                        let b: String = format!("^{}", bulletin.reason());
                        output.push(format!("{}{}", pad, b.bright_red()));
                    }
                }
            }

            if display {
                if !first {
                    message += "...\n";
                }
                message += format!("{}", output.join("\n")).as_str();
            }
        }
        eval_result.message = message;
        Some(eval_result)
    }

    fn get_package_dir(path: &Path) -> Option<PathBuf> {
        Some(path.to_path_buf())
        // match Package::detect_package_type(&path)? {
        //     PackageType::Zip => Some(path.to_path_buf()),
        //     PackageType::Wheel => {
        //         let mut pb = path.to_path_buf();
        //         pb.push(Package::get_wheel_pkg_name(path)?);
        //         Some(pb)
        //     }
        // }
    }

    fn get_wheel_pkg_name(path: &Path) -> Option<String> {
        path.file_name()?
            .to_str()?
            .split_once('-')
            .map(|(str_a, _)| str_a.to_lowercase())
        // for dir in fs::read_dir(path).unwrap() {
        //     let dir_name = dir.unwrap().path().file_name()?.to_str()?.to_string();
        //     if !&dir_name.ends_with("dist-info") {
        //         return Some(dir_name);
        //     }
        // }
        // None
        
    }

    pub fn locate_package(path: &str) -> Option<Box<Path>> {
        let p = PathBuf::from_str(path).unwrap();
        debug!("Locating package: {:?}", &p);

        let pkg_path = Package::get_package_dir(&p)?;
        // println!("pkg path: {}", &pkg_path.as_os_str().to_str().unwrap());
        Some(Box::from(pkg_path))
    }

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
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::str::FromStr;

    use crate::package::Package;
    use crate::package::PackageType;

    #[test]
    fn test_detect_package() {
        let test_wheel = "../../dataset/top/unpacked/Flask-2.0.2-py3-none-any.whl";
        let test_zip = "../../dataset/top/unpacked/termcolor-1.1.0.tar.gz/termcolor-1.1.0";
        let test_none = "../../dataset/top/unpacked/termcolor-1.1.0.tar.gz";

        assert_eq!(
            Package::detect_package_type(test_wheel),
            Some(PackageType::Wheel)
        );
        assert_eq!(
            Package::detect_package_type(test_zip),
            Some(PackageType::Zip)
        );
        assert_eq!(Package::detect_package_type(test_none), None);
    }

    #[test]
    fn test_get_wheel_package_name() {
        let test_wheel =
            PathBuf::from_str("../../dataset/top/unpacked/Flask-2.0.2-py3-none-any.whl").unwrap();
        assert_eq!(
            Package::get_wheel_pkg_name(&test_wheel),
            Some(String::from("flask"))
        );
    }
}
