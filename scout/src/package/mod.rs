use crate::{
    evaluator::{Evaluator, EvaluatorResult, RuleManager},
    source::SourceFile,
};
use colored::Colorize;
use std::{
    collections::HashMap,
    fs::{self, ReadDir},
    io,
    path::{Path, PathBuf},
    str::FromStr,
};

pub struct Package<'e> {
    pub path: PathBuf,
    checker: Evaluator<'e>,
    threshold: f64,
    sources: Vec<SourceFile>,
}

impl<'e> Package<'e> {
    pub fn new(path: &Path, rules: &'e RuleManager, threshold: f64) -> Self {
        Self {
            path: path.to_owned(),
            checker: Evaluator::new(rules.get_rule_sets()),
            threshold,
            sources: Vec::new(),
        }
    }

    // TODO Add proper error handling on this one
    fn add_sourcefile(&mut self, path: PathBuf) -> Option<()> {
        // for debugging stack sizes
        // println!("size of sources vec (len {}): {}", self.sources.len(), stack_size(&self.sources));

        match SourceFile::load(path) {
            Ok(source) => {
                self.sources.push(source);
                Some(())
            }
            Err(err) => {
                error!("Add source error: {}", err);
                None
            }
        }
    }

    fn load_sources(&mut self) {
        trace!("Ackquiring sources...");

        let mut queue: Box<Vec<io::Result<ReadDir>>> = Box::new(vec![fs::read_dir(&self.path)]);
        while !queue.is_empty() {
            if let Some(item) = queue.pop() {
                if let Ok(entries) = item {
                    for entry in entries {
                        if let Ok(dir_entry) = entry {
                            if let Ok(ftype) = dir_entry.file_type() {
                                if ftype.is_file() {
                                    if dir_entry
                                        .path()
                                        .file_name()
                                        .unwrap()
                                        .to_str()
                                        .unwrap()
                                        .ends_with(".py")
                                    {
                                        // println!("{}", dir_entry.path().file_name().unwrap().to_str().unwrap());
                                        self.add_sourcefile(dir_entry.path());
                                    }
                                } else if ftype.is_dir() {
                                    queue.push(fs::read_dir(dir_entry.path()));
                                }
                            }
                        }
                    }
                } else {
                    error!(
                        "Path is not a directory: {}",
                        &self
                            .path
                            .as_path()
                            .to_path_buf()
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                    );
                }
            }
        }
        // let files = collect_files(&self.path, ".py");
        // for file in files.iter() {
        //     trace!("Loading file: {:?}",file.file_name());
        //     self.add_sourcefile(file.to_owned());
        // }
        //     // // println!("file index: {}", idx);
        //     // // file.path().ends_with(".py") did not work. maybe it's a bug?
        //     // if file
        //     //     .file_name()
        //     //     .unwrap()
        //     //     .to_str()
        //     //     .unwrap()
        //     //     .ends_with(".py")
        //     // {
        //     //     trace!("Found python file: {:?}", file);
        //     //
        //     // } else {
        //     //     trace!("file did not end with .py: {:?}", file);
        //     // }
        // }
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

        self.calculate_tfidf(&mut results);

        Some(results)
    }

    // fn sources_with_import(&self, import: &str, results: &Vec<EvaluatorResult>) -> usize {
    //     results
    //         .iter()
    //         .filter(|&er| er.source.has_import(import))
    //         .count()
    // }

    pub fn calculate_tfidf(&self, results: &mut Vec<EvaluatorResult>) {
        let mut lookup: HashMap<&str, HashMap<&String, bool>> = HashMap::new();
        for result in results.iter() {
            let mut im_lookup: HashMap<&String, bool> = HashMap::new();
            for (im, count) in result.source.get_counts() {
                let exists = match count {
                    0 => false,
                    _ => true,
                };
                im_lookup.insert(im, exists);
            }
            lookup.insert(result.source.get_path(), im_lookup);
        }

        let count_sources = results.len() as f64;
        debug!("count_sources: {}", count_sources);

        for result in results.iter_mut() {
            let term_freq: HashMap<String, f64> = result.source.calc_term_frequency_table();
            debug!("TFIDF table for result: {:?}", term_freq);

            for (im, freq) in term_freq {
                let sources_with_im = lookup
                    .iter()
                    .filter(|&(_, im_lookup)| im_lookup.contains_key(&im))
                    .count() as f64;
                let tfidf: f64 = freq * (count_sources / sources_with_im).ln();
                // debug!("(count_sources / sources_with_im).ln(): {}", (count_sources / sources_with_im).ln());
                debug!(
                    "sources with import {}: {} -> tf-idf {}",
                    &im, sources_with_im, &tfidf
                );

                // *result.source.import_set_tfidf(im, tfid)
            }
        }
    }

    pub fn analyse_single(
        &'e mut self,
        path: PathBuf,
        show_all_override: bool,
    ) -> Option<EvaluatorResult> {
        self.add_sourcefile(path)?;
        let source = self.sources.last()?;
        let mut result = vec![self.evaluate_source(source, show_all_override)?];
        self.calculate_tfidf(&mut result);
        Some(result.pop().unwrap())
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
