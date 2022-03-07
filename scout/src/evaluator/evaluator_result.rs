use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::SourceFile;

use super::{density_evaluator::DensityEvaluator, Bulletin, Bulletins, Functionality, Hotspot};

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonResult {
    bulletins: HashMap<String, Bulletins>,
    hotspots: HashMap<String, Vec<Hotspot>>,
}

impl JsonResult {
    pub fn new() -> Self {
        Self {
            bulletins: HashMap::new(),
            hotspots: HashMap::new(),
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

    pub fn get_json(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}

#[derive(Debug)]
pub struct EvaluatorResult {
    pub alerts_functions: i32,
    pub alerts_imports: i32,
    pub density_evaluator: DensityEvaluator,
    pub bulletins: Bulletins,
    pub source: SourceFile,
    pub message: String,
}

impl EvaluatorResult {
    pub fn found_anything(&self) -> bool {
        (self.alerts_functions > 0 && self.alerts_imports > 0) || !self.bulletins.is_empty()
    }

    pub fn any_bulletins_over_threshold(&self, package_threshold: f64) -> bool {
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
        self.density_evaluator.hotspots()
    }

    pub fn bulletins_by_hotspot(&self) -> Vec<(Vec<&Bulletin>, Hotspot)> {
        let mut groups: Vec<(Vec<&Bulletin>, Hotspot)> = vec![];

        for hotspot in self.density_evaluator.hotspots() {
            let mut group: Vec<&Bulletin> = vec![];
            for bulletin in &self.bulletins {
                if bulletin.line() >= hotspot.line_low() && bulletin.line() <= hotspot.line_high() {
                    group.push(bulletin);
                }
            }
            groups.push((group, hotspot));
        }

        groups
    }

    // return a vec with references instead of the value, will help with making this code decoupled
    pub fn bulletins(&self) -> Vec<&Bulletin> {
        self.bulletins.iter().collect::<Vec<&Bulletin>>()
    }

    pub fn display_functionality(&self) {
        for f in self.get_uniq_functionality(&self.bulletins()) {
            debug!("Functionality found: {:?}", f);
        }
    }
}
