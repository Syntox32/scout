use crate::source::SourceFile;
use crate::visitors::{CallEntry, ImportEntry};
use crate::{utils, EvaluatorResult};

use super::density_evaluator::FieldType;
use super::{Bulletin, BulletinReason, Bulletins, DensityEvaluator, Rule, RuleSet};

#[derive(Debug)]
// This is a link between the rule and the rule set for reverse lookups
// TODO: should probably re-design this aspect of it
pub struct RuleEntry<'r>(&'r Rule, &'r RuleSet);

#[derive(Debug)]
pub struct Evaluator {
    rule_sets: Vec<RuleSet>,
}

impl Evaluator {
    pub fn new(rule_sets: Vec<RuleSet>) -> Self {
        Self { rule_sets }
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

    pub fn check_module(
        &self,
        entry: &ImportEntry,
        rule: &Rule,
        set: &RuleSet,
        de: &mut DensityEvaluator,
        bulletins: &mut Bulletins,
        alerts: &mut i32,
    ) {
        if let Rule::Module(func, ident, _name, _desc) = rule {
            if entry.module.to_string() == *ident {
                let notif = Bulletin::new(
                    ident.to_string(),
                    BulletinReason::SuspiciousImport,
                    entry.location,
                    Some(*func),
                    set.threshold,
                );
                bulletins.push(notif);
                de.add_density(FieldType::Imports, entry.location.row());
                *alerts += 1;

                if entry.context == "function" {
                    let notif = Bulletin::new(
                        entry.module.to_string(),
                        BulletinReason::ImportInsideFunction,
                        entry.location,
                        None,
                        0.3f64,
                    );
                    bulletins.push(notif);
                    de.add_density(FieldType::Imports, entry.location.row());
                    *alerts += 1;
                }
            }
        }
    }

    pub fn check_function(
        &self,
        entry: &CallEntry,
        rule: &Rule,
        set: &RuleSet,
        de: &mut DensityEvaluator,
        bulletins: &mut Bulletins,
        alerts: &mut i32,
    ) {
        if let Rule::Function(func, ident, _name, _desc) = rule {
            if utils::get_last_attr(entry.full_identifier.as_str()) == ident {
                let notif = Bulletin::new(
                    entry.full_identifier.to_string(),
                    BulletinReason::SuspiciousFunction,
                    entry.location,
                    Some(*func),
                    set.threshold,
                );
                bulletins.push(notif);
                de.add_density(FieldType::Functions, entry.location.row());
                *alerts += 1;
            }
        }
    }

    pub fn check(&self, source: SourceFile, show_all_override: bool) -> EvaluatorResult {
        let mut alerts_functions: i32 = 0;
        let mut alerts_imports: i32 = 0;
        let mut density_evaluator = DensityEvaluator::new(source.get_loc());
        let mut bulletins: Vec<Bulletin> = vec![];

        for set in self.rule_sets.iter() {

            for entry in source.get_imports() {
                for rule in set.get_module_rules() {
                    self.check_module(
                        entry,
                        rule,
                        set,
                        &mut density_evaluator,
                        &mut bulletins,
                        &mut alerts_imports,
                    )
                }
            }

            for entry in source.function_visitor.get_entries() {
                for rule in set.get_function_rules() {
                    self.check_function(
                        entry,
                        rule,
                        set,
                        &mut density_evaluator,
                        &mut bulletins,
                        &mut alerts_functions,
                    );
                }
            }
        }

        // for set in self.rule_sets.iter() {
        //     for entry in source.function_visitor.get_entries() {
        //         println!("call_entry {:?}", &entry);
        //         let rules = set.get_function_rules();
        //         for rule in rules {
        //             self.check_function(
        //                 entry,
        //                 rule,
        //                 set,
        //                 &mut density_evaluator,
        //                 &mut bulletins,
        //                 &mut alerts_functions,
        //             );
        //         }
        //     }
        // }

        EvaluatorResult {
            alerts_functions,
            alerts_imports,
            density_evaluator,
            bulletins,
            source,
            message: None,
            show_all: show_all_override,
        }

        // for entry in source.get_imports() {
        //     self.import_rules
        //         .iter()
        //         .for_each(|(identifier, rule_entry)| {
        //             if entry.module.to_string() == *identifier {
        //                 //&& !discovered.contains(identifier) {
        //                 let notif = Bulletin::new(
        //                     identifier.to_string(),
        //                     BulletinReason::SuspiciousImport,
        //                     entry.location,
        //                     Some(rule_entry.0.functionality()),
        //                     rule_entry.1.threshold,
        //                 );
        //                 bulletins.push(notif);
        //                 density_evaluator.add_density(FieldType::Imports, entry.location.row());
        //                 alerts_imports += 1;

        //                 discovered.insert(identifier.to_string());

        //                 if entry.context == "function" {
        //                     let notif = Bulletin::new(
        //                         entry.module.to_string(),
        //                         BulletinReason::ImportInsideFunction,
        //                         entry.location,
        //                         None,
        //                         0.3f64,
        //                     );
        //                     bulletins.push(notif);
        //                     density_evaluator.add_density(FieldType::Imports, entry.location.row());
        //                     alerts_imports += 1;
        //                 }
        //             }
        //         });
        // }

        // for entry in &source.function_visitor.entries {
        //     self.function_rules
        //         .iter()
        //         .for_each(|(identifier, rule_entry)| {
        //             if self.get_last_attr(entry.full_identifier.as_str()) == identifier {
        //                 let notif = Bulletin::new(
        //                     entry.full_identifier.to_string(),
        //                     BulletinReason::SuspiciousFunction,
        //                     entry.location,
        //                     Some(rule_entry.0.functionality()),
        //                     rule_entry.1.threshold,
        //                 );
        //                 bulletins.push(notif);
        //                 density_evaluator.add_density(FieldType::Functions, entry.location.row());
        //                 alerts_functions += 1;
        //             }
        //         });
        //     }
        // }
    }
}
