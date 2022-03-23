use crate::source::SourceFile;
use crate::visitors::{CallEntry, ImportEntry};
use crate::{utils, SourceAnalysis};

use super::density_evaluator::FieldType;
use super::{Bulletin, BulletinReason, Bulletins, DensityEvaluator, Rule, RuleSet};

#[derive(Debug)]
// This is a link between the rule and the rule set for reverse lookups
// TODO: should probably re-design this aspect of it
pub struct RuleEntry<'r>(&'r Rule, &'r RuleSet);

#[derive(Debug)]
pub struct Evaluator {
    rule_sets: Vec<RuleSet>,

    /// Enable or disable the use of the multiplier in adding curves.
    opt_enable_multiplier: bool,
}

impl Evaluator {
    pub fn new(rule_sets: Vec<RuleSet>) -> Self {
        Self {
            rule_sets,
            opt_enable_multiplier: true,
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

    fn check_module(
        &self,
        source: &SourceFile,
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

                let multiplier: f64 = if self.opt_enable_multiplier {
                    source.get_tfidf_value(ident).unwrap_or(&1.0f64).to_owned()
                } else {
                    1.0f64
                };
                debug!("TFIDF value for identifier {} set to {}", ident, multiplier);

                de.add_density(FieldType::Imports, entry.location.row(), multiplier);
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
                    de.add_density(FieldType::Imports, entry.location.row(), 1.0f64);
                    *alerts += 1;
                }
            }
        }
    }

    fn rule_check_function(
        &self,
        source: &SourceFile,
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

                let multiplier: f64 = if self.opt_enable_multiplier {
                    source.get_call_tfidf(entry.get_identifier().as_str()).unwrap_or(&1.0f64).to_owned()
                } else {
                    1.0f64
                };
                debug!("TFIDF value for identifier {} set to {}", entry.get_identifier().as_str(), multiplier);

                de.add_density(FieldType::Functions, entry.location.row(), multiplier);
                *alerts += 1;
            }
        }
    }

    pub fn evaluate(&self, analysis: &mut SourceAnalysis) {
        for set in self.rule_sets.iter() {
            for entry in analysis.source.get_imports() {
                for rule in set.get_module_rules() {
                    self.check_module(
                        &analysis.source,
                        entry,
                        rule,
                        set,
                        &mut analysis.density_evaluator,
                        &mut analysis.bulletins,
                        &mut analysis.alerts_imports,
                    )
                }
            }

            for entry in analysis.source.get_entries() {
                for rule in set.get_function_rules() {
                    self.rule_check_function(
                        &analysis.source,
                        entry,
                        rule,
                        set,
                        &mut analysis.density_evaluator,
                        &mut analysis.bulletins,
                        &mut analysis.alerts_functions,
                    );
                }
            }
        }
    }
}
