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

    fn rule_check_module(
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
                let multiplier: f64 = if self.opt_enable_multiplier {
                    source.get_import_tfidf(ident).unwrap_or(&1.0f64).to_owned()
                } else {
                    1.0f64
                };
                debug!("TFIDF value for identifier {} set to {}", ident, multiplier);

                let notif = Bulletin::new(
                    ident.to_string(),
                    BulletinReason::SuspiciousImport,
                    entry.location,
                    Some(*func),
                    set.threshold,
                );
                bulletins.push(notif);
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

    fn misc_import_checks(
        &self,
        _source: &SourceFile,
        entry: &ImportEntry,
        de: &mut DensityEvaluator,
        bulletins: &mut Bulletins,
        alerts: &mut i32,
    ) {
        if entry.is_dynamic {
            let notif = Bulletin::new(
                entry.module.to_string(),
                BulletinReason::DynamicImport,
                entry.location,
                None,
                0.3f64,
            );
            bulletins.push(notif);
            de.add_density(FieldType::Behavior, entry.location.row(), 1.0f64);
            *alerts += 1;
        }
    }

    pub fn evaluate(&self, analysis: &mut SourceAnalysis) {

        for entry in analysis.source.get_imports() {
            self.misc_import_checks(
                &analysis.source,
                entry,
                &mut analysis.density_evaluator,
                &mut analysis.bulletins,
                &mut analysis.alerts_imports,
            );
        }

        for set in self.rule_sets.iter() {
            for entry in analysis.source.get_imports() {
                for rule in set.get_module_rules() {
                    self.rule_check_module(
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
