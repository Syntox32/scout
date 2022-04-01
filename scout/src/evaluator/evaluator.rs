use crate::source::SourceFile;
use crate::visitors::{CallEntry, ImportEntry};
use crate::{utils, Config, SourceAnalysis};

use super::canary::Canaries;
use super::density_evaluator::FieldType;
use super::{Bulletin, BulletinReason, Bulletins, DensityEvaluator, Rule, RuleSet};

use crate::Result;

#[derive(Debug)]
// This is a link between the rule and the rule set for reverse lookups
// TODO: should probably re-design this aspect of it
pub struct RuleEntry<'r>(&'r Rule, &'r RuleSet);

#[derive(Debug)]
pub struct Evaluator {
    rule_sets: Vec<RuleSet>,
    canaries: Canaries,

    /// Enable or disable the use of the multiplier in adding curves.
    opt_enable_multiplier: bool,
}

impl Evaluator {
    pub fn new(rule_sets: Vec<RuleSet>) -> Result<Self> {
        Ok(Self {
            rule_sets,
            canaries: Canaries::new(&None)?,
            opt_enable_multiplier: true,
        })
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
        config: &Config,
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
                de.add_density(
                    FieldType::Imports,
                    entry.location.row(),
                    multiplier,
                    config.tw_imports,
                );
                *alerts += 1;

                // not affected by TFIDF
                if entry.context == "function" {
                    let notif = Bulletin::new(
                        entry.module.to_string(),
                        BulletinReason::ImportInsideFunction,
                        entry.location,
                        None,
                        0.3f64,
                    );
                    bulletins.push(notif);
                    de.add_density(
                        FieldType::Imports,
                        entry.location.row(),
                        1.0f64,
                        config.tw_imports,
                    );
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
        config: &Config,
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
                    source
                        .get_call_tfidf(entry.get_identifier().as_str())
                        .unwrap_or(&1.0f64)
                        .to_owned()
                } else {
                    1.0f64
                };
                debug!(
                    "TFIDF value for identifier {} set to {}",
                    entry.get_identifier().as_str(),
                    multiplier
                );

                de.add_density(
                    FieldType::Functions,
                    entry.location.row(),
                    multiplier,
                    config.tw_functions,
                );
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
        config: &Config,
    ) {
        if entry.is_dynamic {
            let notif = Bulletin::new(
                entry.module.to_string(),
                BulletinReason::DynamicImport,
                entry.location,
                None,
                0.2f64,
            );
            bulletins.push(notif);
            de.add_density(
                FieldType::Behavior,
                entry.location.row(),
                1.0f64,
                config.tw_imports,
            );
            *alerts += 1;
        }
    }

    fn variable_check(
        &self,
        source: &SourceFile,
        de: &mut DensityEvaluator,
        bulletins: &mut Bulletins,
        alerts: &mut i32,
        _config: &Config,
    ) {
        let canaries = self.canaries.get_canaries();
        let locations = source.variable_visitor.get_locations();

        let keys: Vec<String> = canaries.keys().map(|k| k.to_owned()).collect();
        for (identifier, variable) in source.variable_visitor.get_variables() {
            if variable.is_string() {
                if let Some(str_var) = variable.get_string() {
                    for key in &keys {
                        if str_var.starts_with(key) {
                            // shouldn't crash
                            let canary_info = canaries.get(key).unwrap();
                            let location = locations.get(identifier).unwrap();

                            let notif = Bulletin::new(
                                canary_info.identifier.to_string(),
                                BulletinReason::Canary(format!(
                                    "detected '{}' using transform '{}'",
                                    canary_info.identifier, canary_info.transform
                                )),
                                location.clone(),
                                None,
                                0.2f64,
                            );
                            bulletins.push(notif);
                            de.add_density(FieldType::Strings, location.row(), 1.0f64, 1.0f64);
                            *alerts += 1; // TODO: should have its own alert entry
                        }
                    }
                }
            }
        }
    }

    pub fn evaluate(&self, analysis: &mut SourceAnalysis, config: &Config) {
        self.variable_check(
            &analysis.source,
            &mut analysis.density_evaluator,
            &mut analysis.bulletins,
            &mut analysis.alerts_imports,
            config,
        );

        for entry in analysis.source.get_imports() {
            self.misc_import_checks(
                &analysis.source,
                entry,
                &mut analysis.density_evaluator,
                &mut analysis.bulletins,
                &mut analysis.alerts_imports,
                config,
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
                        config,
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
                        config,
                    );
                }
            }
        }
    }
}
