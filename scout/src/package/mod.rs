use crate::{
    evaluator::{Evaluator, EvaluatorResult, RuleManager, EvaluatorCollection},
    source::{SourceFile, self}, Result, utils::collect_files,
};
use colored::Colorize;
use std::{
    collections::HashMap,
    fs::{self, ReadDir},
    io,
    path::{Path, PathBuf},
    str::FromStr,
};

pub struct Package {
    pub path: PathBuf,
    checker: Evaluator,
    threshold: f64,
    show_all_override: bool,
}

impl Package {
    pub fn new(path: PathBuf, rules: RuleManager, threshold: f64, show_all_override: bool) -> Self {
        Self {
            path: path.to_owned(),
            checker: Evaluator::new(rules.get_rule_sets()),
            threshold,
            show_all_override,
        }
    }

    // TODO Add proper error handling on this one
    fn add_sourcefile(&self, path: &PathBuf, target: &mut Vec<SourceFile>) -> Result<()> {
        // for debugging stack sizes
        // println!("size of sources vec (len {}): {}", self.sources.len(), stack_size(&self.sources));

        match SourceFile::load(path) {
            Ok(source) => {
                target.push(source);
                Ok(())
            }
            Err(err) => Err(format!("Could not add source: {}", err.to_string()).into())
        }
    }

    fn load_sources(&self) -> Vec<SourceFile> {
        trace!("Ackquiring sources...");

        let mut sources: Vec<SourceFile> = vec![];

        // let mut queue: Box<Vec<io::Result<ReadDir>>> = Box::new(vec![fs::read_dir(&self.path)]);
        // while !queue.is_empty() {
        //     if let Some(item) = queue.pop() {
        //         if let Ok(entries) = item {
        //             for entry in entries {
        //                 if let Ok(dir_entry) = entry {
        //                     if let Ok(ftype) = dir_entry.file_type() {
        //                         if ftype.is_file() {
        //                             if dir_entry
        //                                 .path()
        //                                 .file_name()
        //                                 .unwrap()
        //                                 .to_str()
        //                                 .unwrap()
        //                                 .ends_with(".py")
        //                             {
        //                                 // println!("{}", dir_entry.path().file_name().unwrap().to_str().unwrap());
        //                                 match self.add_sourcefile(&dir_entry.path(), &mut sources) {
        //                                     Err(err) => warn!("{}", err.to_string()),
        //                                     _ => {},
        //                                 }
        //                             }
        //                         } else if ftype.is_dir() {
        //                             queue.push(fs::read_dir(dir_entry.path()));
        //                         }
        //                     }
        //                 }
        //             }
        //         } else {
        //             error!(
        //                 "Path is not a directory: {}",
        //                 &self
        //                     .path
        //                     .as_path()
        //                     .to_path_buf()
        //                     .file_name()
        //                     .unwrap()
        //                     .to_str()
        //                     .unwrap()
        //             );
        //         }
        //     }
        // }

        let files = collect_files(&self.path, ".py");
        for file in files.iter() {
            trace!("Loading file: {:?}",file.file_name());
            let _ = self.add_sourcefile(file, &mut sources);
        }

        sources
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

    pub fn analyse(self) -> Result<EvaluatorCollection> {
        let mut results: Vec<EvaluatorResult> = vec![];
        for source in self.load_sources() {
            if let Some(eval_result) = self.evaluate_source(source) {
                results.push(eval_result);
            }
        }

        self.calculate_tfidf(&mut results);

        Ok(EvaluatorCollection(results))
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

        for result in results.iter() {
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

    pub fn analyse_single(&mut self) -> Result<EvaluatorCollection> {
        trace!(
            "Analysing single file: '{}'",
            &self.path.as_path().as_os_str().to_str().unwrap()
        );

        let mut sources: Vec<SourceFile> = vec![];
        self.add_sourcefile(&self.path, &mut sources)?;
        let source = sources.pop().unwrap();
        let mut result: Vec<EvaluatorResult> = vec![];
        if let Some(eval_result) = self.evaluate_source(source) {
            result.push(eval_result);
        }
        self.calculate_tfidf(&mut result);

        Ok(EvaluatorCollection(result))
    }

    fn check_source(&self, source: SourceFile) -> EvaluatorResult {
        self.checker.check(source, self.show_all_override, self.threshold)
    }

    fn create_evaluation_report(&self, eval_result: &EvaluatorResult) -> Option<String> {
        let mut message: String = String::from("");

        trace!("Bulletins before purge: {:?}", eval_result.get_all_bulletins());

        // NOTE: override check 1 happens here
        if !self.show_all_override {
            if !(eval_result.found_anything()
                && eval_result.any_bulletins_over_threshold())
            {
                trace!(
                    "File was skipped because no bulletins showed: {}",
                    eval_result.source.get_path()
                );
                return None;
            }
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
                    // NOTE: override check 2 happens here
                    if (bulletin.line() == line && ((hotspot.peak() >= bulletin.threshold) || self.show_all_override))
                        && ((hotspot.peak() > self.threshold) || self.show_all_override)
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
        &self,
        source: SourceFile
    ) -> Option<EvaluatorResult> {
        let mut eval_result = self.check_source(source);

        let report = self.create_evaluation_report(&eval_result)?;
        eval_result.message = Some(report);

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

    pub fn locate_package(path: &str) -> Option<PathBuf> {
        let p = PathBuf::from_str(path).unwrap();
        debug!("Locating package: {:?}", &p);

        let pkg_path = Package::get_package_dir(&p)?;
        // println!("pkg path: {}", &pkg_path.as_os_str().to_str().unwrap());
        Some(pkg_path)
    }
}
