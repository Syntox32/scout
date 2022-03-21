use crate::SourceFile;

use super::{
    density_evaluator::{DensityEvaluator, Field, FieldType},
    Bulletin, Bulletins, Functionality, Hotspot,
};

use serde::Serialize;
use std::{
    collections::HashMap,
    hash::{Hash, Hasher}, fmt,
};

#[derive(Debug, Serialize)]
pub struct JsonResult<'a> {
    bulletins: HashMap<String, Bulletins>,
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

    pub fn add(&mut self, other: &mut EvaluatorResult) {
        let source_path = other.source.get_path();

        self.bulletins.insert(source_path.to_string(), vec![]);
        if let Some(bulletins) = self.bulletins.get_mut(source_path) {
            bulletins.append(&mut other.bulletins);
        }

        self.hotspots.insert(source_path.to_string(), vec![]);
        if let Some(hotspots) = self.hotspots.get_mut(source_path) {
            hotspots.append(&mut other.get_hotspots());
        }
    }

    // pub fn add_with_fields(&mut self, other: &'a mut EvaluatorResult) {
    //     self.add(other);
    //     self.fields = Some(other.density_evaluator.get_fields());
    //     self.combined_field = Some(other.density_evaluator.calculate_combined_field());
    // }

    pub fn get_json(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}

pub struct EvaluatorCollection(pub Vec<EvaluatorResult>);

impl<'a> EvaluatorCollection {
    pub fn to_json(self) -> String {
        let mut out = JsonResult::new();
        let EvaluatorCollection(results) = self;
        for mut res in results {
            out.add(&mut res);
        }
        out.get_json()
    }

    pub fn get_results(&self) -> &Vec<EvaluatorResult> {
        let EvaluatorCollection(results) = self;
        &results
    }
}

impl fmt::Display for EvaluatorCollection {
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

#[derive(Debug)]
pub struct EvaluatorResult {
    pub alerts_functions: i32,
    pub alerts_imports: i32,
    pub density_evaluator: DensityEvaluator,
    pub bulletins: Bulletins,
    pub source: SourceFile,
    pub message: Option<String>,
    pub show_all: bool,
}

impl Hash for EvaluatorResult {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source.get_path().hash(state);
    }
}

impl PartialEq for EvaluatorResult {
    fn eq(&self, other: &Self) -> bool {
        self.source.get_path() == other.source.get_path()
    }
}
impl Eq for EvaluatorResult {}

impl EvaluatorResult {
    pub fn found_anything(&self) -> bool {
        (self.alerts_functions > 0 && self.alerts_imports > 0) || !self.bulletins.is_empty()
    }

    pub fn any_bulletins_over_threshold(&self, package_threshold: f64) -> bool {
        if self.show_all {
            return true;
        }

        for (group, hotspot) in self.bulletins_by_hotspot() {
            for (line, _) in hotspot.get_code(&self.source) {
                // add 1 because  its a 0 based index because of enumerate
                let line = line + 1;
                for bulletin in group.iter() {
                    if (bulletin.line() == line && hotspot.peak() >= bulletin.threshold)
                        && hotspot.peak() > package_threshold
                    {
                        return true;
                    }
                }
            }
        }

        false
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

    pub fn get_hotspots(&self) -> Vec<Hotspot> {
        let hotspots = self.density_evaluator.hotspots();
        trace!("hotspots: {:?}", hotspots);
        hotspots
    }

    pub fn bulletins_by_hotspot(&self) -> Vec<(Vec<&Bulletin>, Hotspot)> {
        let mut groups: Vec<(Vec<&Bulletin>, Hotspot)> = vec![];

        for hotspot in self.get_hotspots() {
            let mut group: Vec<&Bulletin> = vec![];
            for bulletin in &self.bulletins {
                if bulletin.line() >= hotspot.line_low() && bulletin.line() <= hotspot.line_high() {
                    group.push(bulletin);
                }
            }
            groups.push((group, hotspot));
        }

        trace!("Bulletins by hotspot: {:?}", &groups);
        groups
    }

    // return a vec with references instead of the value, will help with making this code decoupled
    pub fn get_bulletins(&self) -> Vec<&Bulletin> {
        self.bulletins.iter().collect::<Vec<&Bulletin>>()
    }

    pub fn display_functionality(&self) {
        for f in self.get_uniq_functionality(&self.get_bulletins()) {
            debug!("Functionality found: {:?}", f);
        }
    }
}
