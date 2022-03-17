use crate::source::SourceFile;
use crate::EvaluatorResult;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use super::density_evaluator::FieldType;
use super::{Bulletin, BulletinReason, DensityEvaluator, Rule, RuleSet};

#[derive(Debug)]
// This is a link between the rule and the rule set for reverse lookups
// TODO: should probably re-design this aspect of it
pub struct RuleEntry<'a>(&'a Rule, &'a RuleSet);

#[derive(Debug)]
pub struct Evaluator<'a> {
    function_rules: HashMap<String, RuleEntry<'a>>,
    import_rules: HashMap<String, RuleEntry<'a>>,
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

    fn get_last_attr<'b>(&self, full_identifier: &'b str) -> &'b str {
        match full_identifier
            .split('.')
            .collect::<Vec<&str>>()
            .iter()
            .last()
        {
            Some(last) => last,
            None => full_identifier,
        }
    }

    // pub fn evaluate_all(&self, sources: Vec<SourceFile>) -> EvaluatorResult {
    //     let mut alerts_functions: i32 = 0;
    //     let mut alerts_imports: i32 = 0;

    //     let mut density_evaluator = DensityEvaluator::new(source.get_loc());
    //     let mut bulletins = vec![];

    //     EvaluatorResult {
    //         alerts_functions,
    //         alerts_imports,
    //         density_evaluator,
    //         bulletins?,
    //         source,
    //         message: String::from(""),
    //     }
    // }

    pub fn check(&self, source: SourceFile, show_all_override: bool) -> EvaluatorResult {
        let mut alerts_functions: i32 = 0;
        let mut alerts_imports: i32 = 0;

        let mut density_evaluator = DensityEvaluator::new(source.get_loc());
        let mut bulletins = vec![];
        let mut discovered: HashSet<String> = HashSet::new();

        for entry in source.get_imports() {
            self.import_rules
                .iter()
                .for_each(|(identifier, rule_entry)| {
                    if entry.module.to_string() == *identifier {
                        //&& !discovered.contains(identifier) {
                        let notif = Bulletin::new(
                            identifier.to_string(),
                            BulletinReason::SuspiciousImport,
                            entry.location,
                            Some(rule_entry.0.functionality()),
                            rule_entry.1.threshold,
                        );
                        bulletins.push(notif);
                        density_evaluator.add_density(FieldType::Imports, entry.location.row());
                        alerts_imports += 1;

                        discovered.insert(identifier.to_string());

                        if entry.context == "function" {
                            let notif = Bulletin::new(
                                entry.module.to_string(),
                                BulletinReason::ImportInsideFunction,
                                entry.location,
                                None,
                                0.3f64,
                            );
                            bulletins.push(notif);
                            density_evaluator.add_density(FieldType::Imports, entry.location.row());
                            alerts_imports += 1;
                        }
                    }
                });
        }

        for entry in &source.function_visitor.entries {
            self.function_rules
                .iter()
                .for_each(|(identifier, rule_entry)| {
                    if self.get_last_attr(entry.full_identifier.as_str()) == identifier {
                        let notif = Bulletin::new(
                            entry.full_identifier.to_string(),
                            BulletinReason::SuspiciousFunction,
                            entry.location,
                            Some(rule_entry.0.functionality()),
                            rule_entry.1.threshold,
                        );
                        bulletins.push(notif);
                        density_evaluator.add_density(FieldType::Functions, entry.location.row());
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
            message: String::from(""),
            show_all: show_all_override,
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
