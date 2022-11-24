use crate::{
    evaluator::{AnalysisResult, Evaluator, RuleManager, SourceAnalysis},
    source::SourceFile,
    utils::{self},
    visitors::VariableType,
    Config, Result,
};
use colored::Colorize;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};
use walkdir::WalkDir;

use rayon::prelude::*;
use tokio::runtime::Runtime;

pub struct Package<'cfg> {
    pub path: PathBuf,
    checker: Evaluator,
    threshold: f64,
    show_all_override: bool,
    config: &'cfg Config,
}

#[derive(Debug)]
pub struct Metadata {
    pub name: String,
    pub deps: Vec<String>,
}

impl Metadata {
    pub fn get_deps(&self) -> &Vec<String> {
        &self.deps
    }
}

impl<'cfg> Package<'cfg> {
    pub fn new(
        path: PathBuf,
        rules: RuleManager,
        threshold: f64,
        show_all_override: bool,
        config: &'cfg Config,
    ) -> Result<Self> {
        Ok(Self {
            path: path.to_owned(),
            checker: Evaluator::new(rules.get_rule_sets())?,
            threshold,
            show_all_override,
            config,
        })
    }

    // fn add_sourcefile(&self, path: &PathBuf, target: &mut Vec<SourceFile>) -> Result<()> {
    //     let source = block_on(tokio::fs::read_to_string(path))?;
    //     match SourceFile::load(path, source) {
    //         Ok(source) => {
    //             target.push(source);
    //             Ok(())
    //         }
    //         Err(err) => Err(format!("Could not add source: {}", err.to_string()).into()),
    //     }
    // }

    pub fn analyse(self) -> Result<AnalysisResult> {
        let analyses: Vec<SourceAnalysis> = self.get_source_analyses()?;
        let results = self.run_evaluation(analyses)?;

        // let metadata = match self.get_metadata(&self.path) {
        //     Ok(metadata) => Some(metadata),
        //     Err(err) => {
        //         error!(
        //             "Error getting metadate for package '{}' in package '{}'",
        //             err.to_string(),
        //             &self.path.as_path().to_str().unwrap()
        //         );
        //         None
        //     }
        // };
        let metadata = None;

        Ok(AnalysisResult::new(results, metadata))
    }

    pub fn analyse_single(&mut self) -> Result<AnalysisResult> {
        trace!(
            "Analysing single file: '{}'",
            &self.path.as_path().as_os_str().to_str().unwrap()
        );

        let source = std::fs::read_to_string(&self.path)?;
        let source = self.get_sourcefile(&self.path, source)?;

        let analyses: Vec<SourceAnalysis> = self.run_precalc(vec![source])?;
        let results = self.run_evaluation(analyses)?;

        Ok(AnalysisResult::new(results, None))
    }

    fn get_from_cache(&self) -> Result<Vec<SourceAnalysis>> {
        let json = std::fs::read_to_string("cache.json")?;
        let obj: Vec<SourceAnalysis> = serde_json::from_str(json.as_str())?;
        Ok(obj)
    }

    fn save_to_cache(&self, analyses: &Vec<SourceAnalysis>) -> Result<()> {
        let json = serde_json::to_string(analyses)?;
        std::fs::write("cache.json", json)?;
        Ok(())
    }

    fn get_source_analyses(&self) -> Result<Vec<SourceAnalysis>> {
        if self.config.use_cache {
            return self.get_from_cache();
        }

        let sources = self.load_sources()?;
        let analyses = self.run_precalc(sources)?;

        if self.config.save_cache {
            self.save_to_cache(&analyses)?;
        }

        Ok(analyses)
    }

    fn get_sourcefile(&self, path: &PathBuf, source: String) -> Result<SourceFile> {
        match SourceFile::load(path, source) {
            Ok(source) => Ok(source),
            Err(err) => Err(format!("Could not add source: {}", err.to_string()).into()),
        }
    }

    async fn get_file(&self, path: &Path) -> Result<String> {
        Ok(tokio::fs::read_to_string(path).await?)
    }

    // fn get_source<F: Future>(&self, entries: Vec<walkdir::DirEntry>) -> Vec<(&Path, String)> {

    //     results
    // }

    fn load_sources(&self) -> Result<Vec<SourceFile>> {
        trace!("Ackquiring sources...");

        let entries = WalkDir::new(&self.path)
            .follow_links(false)
            .into_iter()
            .filter(|e| e.is_ok())
            .map(|e| e.unwrap());
        // .collect::<Vec<walkdir::DirEntry>>();

        let results = async {
            let mut sources: Vec<(PathBuf, String)> = vec![];
            for entry in entries {
                if let Ok(source) = self.get_file(entry.path()).await {
                    sources.push((entry.path().to_owned(), source));
                }
            }
            sources
        };

        let rt = Runtime::new()?;
        let results = rt.block_on(results);

        let source: Vec<SourceFile> = results
            .into_par_iter()
            .filter_map(|(e, source)| self.get_sourcefile(&e, source).ok())
            .collect();

        // let val = Arc::try_unwrap(target).unwrap().into_inner().unwrap();
        Ok(source)
    }

    fn run_evaluation(&self, mut analyses: Vec<SourceAnalysis>) -> Result<Vec<SourceAnalysis>> {
        for analysis in analyses.iter_mut() {
            self.checker.evaluate(analysis, self.config);

            if let Some(report) = self.create_evaluation_report(&analysis) {
                analysis.message = Some(report);
            }
        }

        analyses = analyses
            .into_iter()
            .filter_map(|a| {
                if a.any_bulletins_over_threshold() {
                    Some(a)
                } else {
                    None
                }
            })
            .collect();

        Ok(analyses)
    }

    fn run_precalc(&self, sources: Vec<SourceFile>) -> Result<Vec<SourceAnalysis>> {
        let mut analyses: Vec<SourceAnalysis> = sources
            .into_iter()
            .map(|source| {
                SourceAnalysis::new(source, self.show_all_override, self.threshold, self.config)
            })
            .collect();

        if self.config.feature_tfidf_imports {
            self.calculate_import_tfidf(&mut analyses);
        }

        if self.config.feature_tfidf_calls {
            self.calculate_call_tfidf(&mut analyses);
        }

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
                let tf = self.calc_idf(count_sources, sources_with_im);
                let tfidf: f64 = freq * tf;

                debug!(
                    "sources with import {}: {} -> tf-idf {}",
                    &im, sources_with_im, &tfidf
                );

                result.source.set_import_tfidf(im.as_str(), tfidf);
            }
        }
    }

    // fn calc_df(&self, num_cases: f64, cases_with_term: f64) -> f64 {
    //     cases_with_term / num_cases
    // }

    fn calc_idf(&self, num_cases: f64, cases_with_term: f64) -> f64 {
        (num_cases / cases_with_term).ln()
    }

    // fn calc_idf_smooth(&self, num_cases: f64, cases_with_term: f64) -> f64 {
    //     (num_cases / (1.0f64 + cases_with_term)).ln() + 1.0f64
    // }

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
                let idf: f64 = self.calc_idf(count_sources, sources_with_call);
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

    pub fn get_metadata(&self, path: &PathBuf) -> Result<Metadata> {
        let metadata_files =
            utils::collect_files_matching(path, vec!["METADATA", "PKG-INFO", "setup.py"]);

        if let Some(path) = metadata_files.get("METADATA") {
            return Ok(Package::parse_metadata_file(path)?);
        } else {
            let mut metadata = Metadata {
                name: String::from(""),
                deps: vec![],
            };

            if let Some(pkg_info_path) = metadata_files.get("PKG-INFO") {
                metadata.name = Package::parse_name_from_pkg(pkg_info_path)?;
            }
            if let Some(setup_path) = metadata_files.get("setup.py") {
                metadata.deps = self.parse_deps_from_setup(setup_path)?;
            }

            return Ok(metadata);
        }
    }

    fn parse_deps_from_setup(&self, path: &PathBuf) -> Result<Vec<String>> {
        let mut deps: Vec<String> = vec![];

        // let mut sources: Vec<SourceFile> = vec![];
        // self.add_sourcefile(path, &mut sources)?;
        // let source = sources.pop().unwrap();
        let source = std::fs::read_to_string(path)?;
        let source = self.get_sourcefile(path, source)?;

        let entries = source.get_entries();
        for entry in entries {
            if entry.get_identifier() == "setup" {
                for (key, word) in &entry.keywords {
                    if let Some(key) = key.as_ref() {
                        if key == &String::from("install_requires") {
                            if let Some(list) = word {
                                match list {
                                    VariableType::List(items) => {
                                        for item in items.iter() {
                                            if let Some(var) = item {
                                                if var.is_string() {
                                                    let str = var.get_string().unwrap();
                                                    deps.push(str.to_string());
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(deps)
    }

    fn parse_name_from_pkg(path: &PathBuf) -> Result<String> {
        let mut name: String = String::from("");

        for line in utils::read_lines(path)? {
            if let Ok(line) = line {
                if line.starts_with("Name:") {
                    if let Some((_, right)) = line.split_once(':') {
                        name = right.trim().to_string();
                    }
                }
            }
        }

        Ok(name)
    }

    fn parse_metadata_file(path: &PathBuf) -> Result<Metadata> {
        let mut name: String = String::from("");
        let mut deps: Vec<String> = vec![];

        for line in utils::read_lines(path)? {
            if let Ok(line) = line {
                if line.starts_with("Name:") {
                    if let Some((_, right)) = line.split_once(':') {
                        name = right.trim().to_string();
                    }
                } else if line.starts_with("Requires-Dist:") {
                    if let Some((_, right)) = line.split_once(':') {
                        if let Some(dep) = right.split_ascii_whitespace().into_iter().next() {
                            deps.push(dep.to_string());
                        }
                    }
                }
            }
        }

        Ok(Metadata { name, deps })
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

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, str::FromStr};

    use crate::{Metadata, Package};

    #[test]
    fn test_parse_metadata_file() {
        let metadata_file = PathBuf::from_str("../tests/test_files/wheel-metadata").unwrap();

        let metadata: Metadata = Package::parse_metadata_file(&metadata_file).unwrap();

        assert_eq!(metadata.name, String::from("apache-beam"));
        assert!(metadata.deps.contains(&String::from("crcmod")));
        assert!(metadata
            .deps
            .contains(&String::from("google-cloud-bigquery")));
        assert!(metadata.deps.contains(&String::from("pytest")));
    }
}
