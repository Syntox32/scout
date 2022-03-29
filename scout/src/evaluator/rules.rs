use serde::{Deserialize, Serialize};
use std::{path::PathBuf, str::FromStr};

use crate::utils;
use crate::Result;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
//#[serde(tag = "functionality")]
pub enum Functionality {
    Encryption,
    Encoding,
    Compression,
    FileSystem,
    Network,
    Process,
    System,
    NotSpecific,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Rule {
    /// Module(Functionality, Identifier, Name (optional), Description (optional))
    Module(Functionality, String, Option<String>, Option<String>),
    Function(Functionality, String, Option<String>, Option<String>),
}

impl Rule {
    pub fn functionality(&self) -> Functionality {
        match self {
            Rule::Module(functionality, _, _, _) => *functionality,
            Rule::Function(functionality, _, _, _) => *functionality,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RuleSet {
    pub name: String,
    pub threshold: f64,
    pub rules: Vec<Rule>,
}

impl RuleSet {
    pub fn get_module_rules(&self) -> Vec<&Rule> {
        self.rules
            .iter()
            .filter(|&r| match r {
                Rule::Module(..) => true,
                _ => false,
            })
            .collect::<Vec<&Rule>>()
    }

    pub fn get_function_rules(&self) -> Vec<&Rule> {
        self.rules
            .iter()
            .filter(|&r| match r {
                Rule::Function(..) => true,
                _ => false,
            })
            .collect::<Vec<&Rule>>()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rules(Vec<RuleSet>);

pub struct RuleManager {
    rule_sets: Vec<RuleSet>,
}

impl RuleManager {
    pub const DEFAULT_RULE_FILE: &'static str = "rules.ron";
    const DEFAULT_RULES: &'static str = include_str!("rules.ron");

    fn load_rules(rule_path: &Option<String>) -> Result<Vec<RuleSet>> {
        let rules_content: String = match rule_path {
            Some(rules_path) => {
                let path = PathBuf::from_str(rules_path.as_str())?;
                trace!("Loading rulesets from: '{}'", &rules_path);
                utils::load_from_file(path)?
            }
            None => {
                trace!(
                    "Using default ruleset: '{}'",
                    RuleManager::DEFAULT_RULE_FILE
                );
                RuleManager::DEFAULT_RULES.to_owned()
            }
        };

        match ron::from_str(rules_content.as_str()) {
            Ok(Rules(rule_sets)) => {
                trace!("Loaded {} rulesets.", &rule_sets.len());
                Ok(rule_sets)
            }
            Err(err) => Err(err.into()),
        }
    }

    pub fn new(rule_path: &Option<String>) -> Result<Self> {
        Ok(Self {
            rule_sets: RuleManager::load_rules(rule_path)?,
        })
    }

    pub fn get_rule_sets(self) -> Vec<RuleSet> {
        self.rule_sets
    }
}
