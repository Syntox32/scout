use crate::{Config, Metadata, SourceFile};

use super::{
    density_evaluator::{DensityEvaluator, Field, FieldType},
    Bulletin, Bulletins, Functionality, Hotspot,
};

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt,
    hash::{Hash, Hasher},
};

#[derive(Debug, Serialize)]
pub struct JsonResult<'a> {
    bulletins: HashMap<String, Vec<&'a Bulletin>>,
    hotspots: HashMap<String, Vec<Hotspot>>,
    fields: Option<&'a HashMap<FieldType, Field>>,
    combined_field: Option<Field>,
}

impl<'a> JsonResult<'a> {
    pub fn new() -> Self {
        Self {
            bulletins: HashMap::new(),
            hotspots: HashMap::new(),
            fields: None,
            combined_field: None,
        }
    }

    pub fn add(&mut self, other: &'a SourceAnalysis) {
        let source_path = other.source.get_path();

        self.bulletins.insert(source_path.to_string(), vec![]);
        self.hotspots.insert(source_path.to_string(), vec![]);

        for (mut other_bulletins, hotspot) in other.bulletins_by_hotspot() {
            if let Some(json_hotspots) = self.hotspots.get_mut(source_path) {
                json_hotspots.push(hotspot);
            }

            if let Some(json_bulletins) = self.bulletins.get_mut(source_path) {
                json_bulletins.append(&mut other_bulletins);
            }
        }
    }

    pub fn add_with_fields(&mut self, other: &'a SourceAnalysis) {
        self.add(other);
        self.fields = Some(other.density_evaluator.get_fields());
        self.combined_field = Some(other.density_evaluator.calculate_combined_field());
    }

    pub fn get_json(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}

#[derive(Debug)]
pub struct AnalysisResult {
    results: Vec<SourceAnalysis>,
    metadata: Option<Metadata>,
}

impl<'a> AnalysisResult {
    pub fn new(results: Vec<SourceAnalysis>, metadata: Option<Metadata>) -> Self {
        Self { results, metadata }
    }

    pub fn get_metadata(&self) -> &Option<Metadata> {
        &self.metadata
    }

    pub fn get_results(&self) -> &Vec<SourceAnalysis> {
        &self.results
    }

    pub fn get_dependencies(&self) -> Option<&Vec<String>> {
        Some(self.metadata.as_ref()?.get_deps())
    }

    fn get_json(&self, with_fields: bool) -> String {
        let mut out = JsonResult::new();
        for res in self.get_results() {
            out.add(res);

            if with_fields {
                out.add_with_fields(res);
            }
        }
        out.get_json()
    }

    pub fn to_json(&self) -> String {
        self.get_json(false)
    }

    pub fn to_json_with_fields(&self) -> String {
        self.get_json(true)
    }
}

impl fmt::Display for AnalysisResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut result_str: String = String::from("");
        for result in self.get_results() {
            if let Some(message) = &result.message {
                result_str.push_str(message);
            }
        }
        write!(f, "{}", result_str)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SourceAnalysis {
    pub alerts_functions: i32,
    pub alerts_imports: i32,
    pub density_evaluator: DensityEvaluator,
    pub bulletins: Bulletins,
    pub source: SourceFile,
    pub message: Option<String>,
    pub show_all: bool,
    pub global_threshold: f64,
}

impl Hash for SourceAnalysis {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source.get_path().hash(state);
    }
}

impl PartialEq for SourceAnalysis {
    fn eq(&self, other: &Self) -> bool {
        self.source.get_path() == other.source.get_path()
    }
}
impl Eq for SourceAnalysis {}

impl<'a> SourceAnalysis {
    pub fn new(
        source: SourceFile,
        show_all_override: bool,
        global_threshold: f64,
        config: &Config,
    ) -> Self {
        Self {
            alerts_functions: 0,
            alerts_imports: 0,
            density_evaluator: DensityEvaluator::new(source.get_loc(), config),
            bulletins: vec![],
            source,
            message: None,
            show_all: show_all_override,
            global_threshold,
        }
    }

    pub fn found_anything(&self) -> bool {
        (self.alerts_functions > 0 && self.alerts_imports > 0) || !self.bulletins.is_empty()
    }

    pub fn any_bulletins_over_threshold(&self) -> bool {
        if self.bulletins.is_empty() {
            return false;
        }

        if self.show_all {
            return true;
        }

        !self.get_visible_bulletins().is_empty()
    }

    fn bulletin_display_check(&self, bulletin: &Bulletin, hotspot: &Hotspot) -> bool {
        (bulletin.line() >= hotspot.line_low()
            && bulletin.line() <= hotspot.line_high()
            && hotspot.peak() >= bulletin.threshold
            && hotspot.peak() >= self.global_threshold)
            || self.show_all
    }

    pub fn get_visible_bulletins(&self) -> Vec<&Bulletin> {
        let mut bulletins: Vec<&Bulletin> = vec![];
        for (group, hotspot) in self.bulletins_by_hotspot() {
            for &bulletin in group.iter() {
                if self.bulletin_display_check(bulletin, &hotspot) {
                    bulletins.push(bulletin);
                }
            }
        }
        bulletins
    }

    pub fn get_source(&self) -> &SourceFile {
        &self.source
    }

    pub fn get_uniq_functionality(&self, bulletins: &[&Bulletin]) -> Vec<Functionality> {
        let mut functionality: Vec<Functionality> = bulletins
            .iter()
            .filter_map(|b| b.functionality)
            .collect::<Vec<Functionality>>();
        functionality.sort();
        functionality.dedup();
        functionality
    }

    /// This is the list of bulletins before the filtering by threshold.
    ///
    /// Calling this function is effectivley getting bulletins with show_all enabled.
    pub fn get_all_bulletins(&self) -> Vec<&Bulletin> {
        self.bulletins.iter().collect::<Vec<&Bulletin>>()
    }

    pub fn get_hotspots(&self) -> Vec<Hotspot> {
        let hotspots = self.density_evaluator.hotspots();
        trace!("hotspots: {:?}", hotspots);
        hotspots
    }

    pub fn bulletins_by_hotspot(&'a self) -> Vec<(Vec<&'a Bulletin>, Hotspot)> {
        let mut groups: Vec<(Vec<&'a Bulletin>, Hotspot)> = vec![];

        for hotspot in self.get_hotspots() {
            let mut group: Vec<&'a Bulletin> = vec![];

            for bulletin in self.bulletins.iter() {
                if self.bulletin_display_check(bulletin, &hotspot) {
                    group.push(bulletin);
                }
            }

            if !group.is_empty() {
                groups.push((group, hotspot));
            }
        }

        trace!("Bulletins by hotspot: {:?}", &groups);
        groups
    }

    pub fn display_functionality(&self) {
        for f in self.get_uniq_functionality(&self.get_all_bulletins()) {
            debug!("Functionality found: {:?}", f);
        }
    }
}
