use crate::{
    evaluator::{AnalysisResult, Evaluator, RuleManager, SourceAnalysis},
    source::SourceFile,
    utils::collect_files,
    Result,
};
use colored::Colorize;
use std::{
    collections::HashMap,
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

    fn add_sourcefile(&self, path: &PathBuf, target: &mut Vec<SourceFile>) -> Result<()> {
        match SourceFile::load(path) {
            Ok(source) => {
                target.push(source);
                Ok(())
            }
            Err(err) => Err(format!("Could not add source: {}", err.to_string()).into()),
        }
    }

    fn load_sources(&self) -> Vec<SourceFile> {
        trace!("Ackquiring sources...");

        let mut sources: Vec<SourceFile> = vec![];
        let files = collect_files(&self.path, ".py");

        for file in files.iter() {
            trace!("Loading file: {:?}", file.file_name());
            let _ = self.add_sourcefile(file, &mut sources);
        }

        sources
    }

    pub fn analyse(self) -> Result<AnalysisResult> {
        let results: Vec<SourceAnalysis> = self.run_pipeline(self.load_sources())?;
        Ok(AnalysisResult(results))
    }

    pub fn analyse_single(&mut self) -> Result<AnalysisResult> {
        trace!(
            "Analysing single file: '{}'",
            &self.path.as_path().as_os_str().to_str().unwrap()
        );

        let mut sources: Vec<SourceFile> = vec![];
        self.add_sourcefile(&self.path, &mut sources)?;
        let source = sources.pop().unwrap();

        let result: Vec<SourceAnalysis> = self.run_pipeline(vec![source])?;

        Ok(AnalysisResult(result))
    }

    fn run_pipeline(&self, sources: Vec<SourceFile>) -> Result<Vec<SourceAnalysis>> {
        let mut analyses: Vec<SourceAnalysis> = vec![];

        for source in sources {
            analyses.push(SourceAnalysis::new(
                source,
                self.show_all_override,
                self.threshold,
            ));
        }

        self.calculate_import_tfidf(&mut analyses);
        self.calculate_call_tfidf(&mut analyses);

        for analysis in analyses.iter_mut() {
            self.checker.evaluate(analysis);

            if let Some(report) = self.create_evaluation_report(&analysis) {
                analysis.message = Some(report);
            }
        }

        analyses = analyses.into_iter()
            .filter_map(|a| if a.any_bulletins_over_threshold() {
                Some(a)
            } else { 
                None 
            }).collect();

        Ok(analyses)
    }

    fn calculate_import_tfidf(&self, results: &mut Vec<SourceAnalysis>) {
        let mut lookup: HashMap<String, HashMap<String, bool>> = HashMap::new();
        for result in results.iter() {
            let mut im_lookup: HashMap<String, bool> = HashMap::new();
            for (im, count) in result.source.get_import_counts() {
                let exists = match count {
                    0 => false,
                    _ => true,
                };
                im_lookup.insert(im.to_string(), exists);
            }
            lookup.insert(result.source.get_path().to_string(), im_lookup);
        }

        let count_sources = results.len() as f64;
        debug!("count_sources: {}", count_sources);

        for result in results.iter_mut() {
            let term_freq: HashMap<String, f64> = result.source.import_term_frequency_table();
            debug!("TFIDF table for result: {:?}", term_freq);

            for (im, freq) in term_freq {
                let sources_with_im = lookup
                    .iter()
                    .filter(|&(_, im_lookup)| im_lookup.contains_key(&im))
                    .count() as f64;
                let tf = self.calc_idf_smooth(count_sources, sources_with_im);
                let tfidf: f64 = freq * tf;

                debug!(
                    "sources with import {}: {} -> tf-idf {}",
                    &im, sources_with_im, &tfidf
                );

                result.source.set_import_tfidf(im.as_str(), tfidf);
            }
        }
    }

    fn calc_df(&self, num_cases: f64, cases_with_term: f64) -> f64 {
        cases_with_term / num_cases
    }

    fn calc_idf(&self, num_cases: f64, cases_with_term: f64) -> f64 {
        (num_cases / cases_with_term).ln()
    }

    fn calc_idf_smooth(&self, num_cases: f64, cases_with_term: f64) -> f64 {
        (num_cases / (1.0f64 + cases_with_term)).ln() + 1.0f64
    }

    fn calculate_call_tfidf(&self, results: &mut Vec<SourceAnalysis>) {
        let mut lookup: HashMap<String, HashMap<String, bool>> = HashMap::new();
        for result in results.iter() {
            let mut call_lookup: HashMap<String, bool> = HashMap::new();
            for (call, count) in result.source.get_call_counts() {
                let exists = match count {
                    0 => false,
                    _ => true,
                };
                call_lookup.insert(call.to_string(), exists);
            }
            lookup.insert(result.source.get_path().to_string(), call_lookup);
        }

        let count_sources = results.len() as f64;
        debug!("count_sources: {}", count_sources);

        for result in results.iter_mut() {
            let term_freq: HashMap<String, f64> = result.source.calc_term_frequency_table();
            debug!("TFIDF table for result: {:?}", term_freq);

            for (call, freq) in term_freq {
                let sources_with_call = lookup
                    .iter()
                    .filter(|&(_, call_lookup)| call_lookup.contains_key(&call))
                    .count() as f64;
                // let idf: f64 = self.calc_idf(count_sources, sources_with_call);
                let idf: f64 = self.calc_df(count_sources, sources_with_call);
                let tfidf: f64 = freq * idf;

                debug!(
                    "sources with import {}: {} -> tf-idf {}",
                    &call, sources_with_call, &tfidf
                );

                result.source.set_call_tfidf(call.as_str(), tfidf);
            }
        }
    }

    fn create_evaluation_report(&self, eval_result: &SourceAnalysis) -> Option<String> {
        let mut message: String = String::from("");

        trace!(
            "Bulletins before purge: {:?}",
            eval_result.get_all_bulletins()
        );

        // NOTE: override check 1 happens here
        if !self.show_all_override {
            if !(eval_result.found_anything() && eval_result.any_bulletins_over_threshold()) {
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
            eval_result.source.display_functions()
        );
        debug!(
            "Imports found in source file: [{}]",
            eval_result.source.display_imports()
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
                    if (bulletin.line() == line
                        && ((hotspot.peak() >= bulletin.threshold) || self.show_all_override))
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
        Some(pkg_path)
    }
}
