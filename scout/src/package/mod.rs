use crate::{
    evaluator::{Evaluator, EvaluatorResult, RuleManager},
    source::SourceFile,
    utils::{collect_files, stack_size},
};
use colored::Colorize;
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

// https://stackoverflow.com/questions/25436356/how-to-loop-over-boxed-iterator
// trait BoxedIter<A>: Iterator {}
// impl<A, I> BoxedIter<A> for I
//     where I: Iterator {}

// impl<A> Iterator<A> for Box<dyn BoxedIter<A>> {
//     type Item = A;
//     fn next(&mut self) -> Option<A> {
//         self.next()
//     }
// }

pub struct Package<'e> {
    pub path: PathBuf,
    checker: Evaluator<'e>,
    threshold: f64,
    sources: Box<Vec<Box<SourceFile>>>,
}

impl<'e> Package<'e> {
    pub fn new(path: &Path, rules: &'e RuleManager, threshold: f64) -> Self {
        Self {
            path: path.to_owned(),
            checker: Evaluator::new(rules.get_rule_sets()),
            threshold,
            sources: Box::new(Vec::new()),
        }
    }

    // TODO Add proper error handling on this one
    fn add_sourcefile(&mut self, path: &PathBuf) -> Option<()> {
        println!("size of all_files vec: {}", stack_size(&self.sources));

        match SourceFile::load(path) {
            Ok(source) => {
                self.sources.push(Box::new(source));
                Some(())
            }
            Err(err) => {
                error!(
                    "Parse or load error '{}' in file '{}'",
                    err,
                    path.as_path().as_os_str().to_str().unwrap()
                );
                None
            }
        }
    }

    fn load_sources(&mut self) {
        trace!("Ackquiring sources...");
        let files = collect_files(&self.path);
        for (idx, file) in files.iter().enumerate() {
            trace!("Loaded file: {:?}", &file.path().file_name());
            // println!("file index: {}", idx);
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

                self.add_sourcefile(&p);
            } else {
                trace!("file did not end with .py: {:?}", file);
            }
        }
    }

    pub fn analyse(&'e mut self, show_all_override: bool) -> Option<Vec<EvaluatorResult>> {
        self.load_sources();

        let mut results: Vec<EvaluatorResult> = vec![];
        for source in &(*self.sources) {
            let path = &source.get_path().to_owned();
            if let Some(result) = self.evaluate_source(&source, show_all_override) {
                results.push(result);
            } else {
                warn!("Could not evaluate source: {}", path);
            }
        }

        Some(results)
    }

    pub fn analyse_single(
        &'e mut self,
        path: &PathBuf,
        show_all_override: bool,
    ) -> Option<EvaluatorResult> {
        self.add_sourcefile(path)?;
        let source = self.sources.last()?;
        let result = self.evaluate_source(source, show_all_override)?;

        Some(result)
        // Some(self.evaluate_source(source, show_all_override)?)
    }

    fn check_source(&self, source: &'e SourceFile, show_all_override: bool) -> EvaluatorResult {
        self.checker.check(source, show_all_override)
    }

    fn create_evaluation_report(&self, eval_result: &EvaluatorResult) -> Option<String> {
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

            let hotspot_code: Vec<(usize, String)> = hotspot.get_code(&eval_result.source);

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
        Some(message)
    }

    fn evaluate_source(
        &'e self,
        source: &'e SourceFile,
        show_all_override: bool,
    ) -> Option<EvaluatorResult> {
        let mut eval_result = self.check_source(source, show_all_override);

        match self.create_evaluation_report(&eval_result) {
            Some(message) => {
                eval_result.message = Some(message);
                Some(eval_result)
            }
            None => None,
        }
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

    pub fn locate_package(path: &str) -> Option<Box<Path>> {
        let p = PathBuf::from_str(path).unwrap();
        debug!("Locating package: {:?}", &p);

        let pkg_path = Package::get_package_dir(&p)?;
        // println!("pkg path: {}", &pkg_path.as_os_str().to_str().unwrap());
        Some(Box::from(pkg_path))
    }
}
