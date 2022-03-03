use serde::{Deserialize, Serialize};

use crate::source::SourceFile;
use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;
use std::fs::{self, File};
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

use std::process::Command;
use std::str::FromStr;

use super::{Bulletin, BulletinReason, BulletinSeverity, Bulletins, Functionality, Rule, RuleSet};

fn gaussian_density(x: f64, mu: f64, variance: f64) -> f64 {
    let sigma = variance.sqrt();
    (1f64 / (sigma * (2f64 * PI as f64).sqrt())) * ((-(x - mu).powi(2)) / (2f64 * variance)).exp()
}

#[derive(Serialize, Deserialize)]
struct Plot {
    x: Vec<f64>,
    y: Vec<f64>,
}

#[derive(Debug)]
pub struct Hotspot {
    startx: f64,
    endx: f64,
    peak: f64,
}

impl Hotspot {
    pub fn new() -> Self {
        Self {
            startx: 0.0,
            endx: 0.0,
            peak: 0.0,
        }
    }

    pub fn peak(&self) -> f64 {
        self.peak
    }

    pub fn line_low(&self) -> usize {
        self.startx.round() as usize
    }

    pub fn line_high(&self) -> usize {
        self.endx.round() as usize
    }

    pub fn get_code<'a>(&self, source: &'a SourceFile) -> Vec<(usize, &'a str)> {
        let hotspot_code: Vec<(usize, &str)> = source
            .source
            .lines()
            .enumerate()
            .collect::<Vec<(usize, &str)>>()
            .drain(self.line_low()..self.line_high())
            .collect::<Vec<(usize, &str)>>();
        hotspot_code
    }
}

#[derive(Debug)]
pub struct DensityEvaluator {
    pub loc: usize,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

impl DensityEvaluator {
    const RESOLUTION: f64 = 0.1;

    pub fn new(loc: usize) -> Self {
        let num_points: u32 = ((loc as f64) / DensityEvaluator::RESOLUTION) as u32;

        let mut x: Vec<f64> = Vec::with_capacity(num_points as usize);
        let mut y: Vec<f64> = Vec::with_capacity(num_points as usize);
        let mut curr: f64 = 0f64;
        for _ in 0..num_points {
            x.push(curr);
            y.push(0f64);
            curr += DensityEvaluator::RESOLUTION;
        }

        Self { loc, x, y }
    }

    pub fn add_density(&mut self, row: usize) {
        let variance: f64 = 5.0;
        let line: f64 = row as f64;
        // println!("line: {}, variance: {}", line, variance);

        for (y, x) in self.y.iter_mut().zip(self.x.iter_mut()) {
            *y += gaussian_density(*x, line, variance);
        }
    }

    // get maximum Y value in the range [startX, endX]
    fn get_max_y(&self, start: f64, end: f64) -> Option<f64> {
        if self.y.is_empty() {
            return None;
        }

        let (_, maxy) = self
            .x
            .iter()
            .zip(self.y.iter())
            .filter(|(&x, _)| x >= start && x <= end)
            .max_by(|(_, y1), (_, y2)| y1.partial_cmp(y2).unwrap())?;
        Some(*maxy)
    }

    pub fn hotspots(&self) -> Vec<Hotspot> {
        let mut spots: Vec<Hotspot> = vec![];
        const THRESHOLD: f64 = 0.01;

        let mut curr: Hotspot = Hotspot::new();
        let mut in_group: bool = false;

        for (y, x) in self.y.iter().zip(self.x.iter()) {
            if *y > THRESHOLD && !in_group {
                in_group = true;
                curr.startx = *x;
            } else if *y <= THRESHOLD && in_group {
                in_group = false;
                curr.endx = *x;
                curr.peak = self.get_max_y(curr.startx, curr.endx).unwrap_or(0.0);
                spots.push(curr);
                curr = Hotspot::new();
            }
        }

        if in_group && !self.x.is_empty() {
            let lastx = self.x.last().unwrap();
            curr.endx = *lastx;
            curr.peak = self.get_max_y(curr.startx, curr.endx).unwrap_or(0.0);
            spots.push(curr);
        }

        spots
    }

    // debug function only
    pub fn _plot(&self) {
        let plt = Plot {
            x: self.x.clone(),
            y: self.y.clone(),
        };
        let json = serde_json::to_string(&plt).unwrap();
        fs::write(&PathBuf::from_str("conf/plot.json").unwrap(), &json).unwrap();

        let _ = Command::new("python3")
            .arg("../ast_experiment/plot.py")
            .arg("--file")
            .arg("../engine/conf/plot.json")
            .spawn()
            .expect("failed to execute process");
    }
}

#[cfg(test)]
mod tests {
    use super::gaussian_density;

    #[test]
    fn test_gaussian_density() {
        let val = gaussian_density(11f64, 10f64, 1f64);
        println!("{}", val);
        assert_eq!(val, 0.24197072451914337f64);
    }
}

#[derive(Debug)]
// This is a link between the rule and the rule set for reverse lookups
pub struct RuleEntry<'a>(&'a Rule, &'a RuleSet);

#[derive(Debug)]
pub struct Evaluator<'a> {
    //pub rule_sets: Vec<RuleSet>,
    function_rules: HashMap<String, RuleEntry<'a>>,
    import_rules: HashMap<String, RuleEntry<'a>>,
}

#[derive(Debug)]
pub struct EvaluatorResult<'a> {
    pub alerts_functions: i32,
    pub alerts_imports: i32,
    pub density_evaluator: DensityEvaluator,
    pub bulletins: Bulletins,
    pub source: &'a SourceFile,
}

impl<'a> EvaluatorResult<'a> {
    // pub fn _display_notifications(&self) {
    //     self.bulletins
    //         .iter()
    //         .for_each(|notif| println!("{}", notif.generate(self)));
    // }

    pub fn found_anything(&self) -> bool {
        // trace!("Found anything: {} function alerts, {} import alerts", self.alerts_functions, self.alerts_imports);
        (self.alerts_functions > 0 && self.alerts_imports > 0) || !self.bulletins.is_empty()
    }

    pub fn any_bulletins_over_threshold(&self) -> bool {
        for (group, hotspot) in self.bulletins_by_hotspot() {
            for (line, _) in hotspot.get_code(self.source) {
                let line = line + 1;
                for bulletin in group.iter() {
                    // add one cause its a 0 based index because of enumerate
                    if bulletin.line() == line && hotspot.peak() >= bulletin.threshold {
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

impl<'a> Evaluator<'a> {
    pub fn new(rule_sets: &[RuleSet]) -> Evaluator {
        let mut import_rules: HashMap<String, RuleEntry> = HashMap::new();
        let mut function_rules: HashMap<String, RuleEntry> = HashMap::new();

        rule_sets.iter().for_each(|rs| {
            rs.rules.iter().for_each(|rule| {
                match rule {
                    Rule::Module(_, identifier, _, _) => {
                        import_rules.insert(identifier.to_string(), RuleEntry(rule, rs))
                    }
                    Rule::Function(_, identifier, _, _) => {
                        function_rules.insert(identifier.to_string(), RuleEntry(rule, rs))
                    }
                };
            })
        });

        Evaluator {
            import_rules,
            function_rules,
        }
    }

    pub fn check(&self, source: &'a SourceFile) -> EvaluatorResult<'a> {
        let mut alerts_functions: i32 = 0;
        let mut alerts_imports: i32 = 0;

        let mut density_evaluator = DensityEvaluator::new(source.get_loc());
        let mut bulletins = vec![];

        for (module, entry) in &source.import_visitor.imports {
            self.import_rules
                .iter()
                .for_each(|(identifier, rule_entry)| {
                    if module.starts_with(identifier) {
                        let notif = Bulletin::new(
                            identifier.to_string(),
                            BulletinReason::SuspiciousImport,
                            BulletinSeverity::Informative,
                            entry.location,
                            Some(rule_entry.0.functionality()),
                            rule_entry.1.threshold,
                        );
                        bulletins.push(notif);
                        density_evaluator.add_density(entry.location.row());
                        alerts_imports += 1;

                        if entry.context == "function" {
                            let notif = Bulletin::new(
                                module.to_string(),
                                BulletinReason::ImportInsideFunction,
                                BulletinSeverity::Suspicious,
                                entry.location,
                                None,
                                0.3f64,
                            );
                            bulletins.push(notif);
                            density_evaluator.add_density(entry.location.row());
                            alerts_imports += 1;
                        }
                    }
                });
        }

        for entry in &source.function_visitor.entries {
            self.function_rules
                .iter()
                .for_each(|(identifier, rule_entry)| {
                    if entry.full_identifier.ends_with(identifier) {
                        let notif = Bulletin::new(
                            entry.full_identifier.to_string(),
                            BulletinReason::SuspiciousFunction,
                            BulletinSeverity::Informative,
                            entry.location,
                            Some(rule_entry.0.functionality()),
                            rule_entry.1.threshold,
                        );
                        bulletins.push(notif);
                        density_evaluator.add_density(entry.location.row());
                        alerts_functions += 1;
                    }
                });
        }

        EvaluatorResult {
            alerts_functions,
            alerts_imports,
            density_evaluator,
            bulletins,
            source,
        }
    }

    fn _create_hashset<T>(filename: T) -> HashSet<String>
    where
        T: AsRef<Path>,
    {
        let f = File::open(filename).unwrap();
        io::BufReader::new(f)
            .lines()
            .map(|l| l.unwrap().replace("\n", "").to_lowercase())
            .collect::<HashSet<String>>()
    }
}
