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
    pub const DEFAULT_RULE_FILE: &'static str = "conf/rules.ron";

    fn load_rules(rule_path: &str) -> Result<Vec<RuleSet>> {
        let path = &PathBuf::from_str(rule_path)?;
        let rules_content = utils::load_from_file(path)?;

        match ron::from_str(rules_content.as_str()) {
            Ok(Rules(rule_sets)) => {
                trace!("Loaded {} rulesets from '{}'", rule_sets.len(), rule_path);
                Ok(rule_sets)
            }
            Err(err) => Err(err.into()),
        }
    }

    pub fn new(rule_path: &str) -> Result<Self> {
        Ok(Self {
            rule_sets: RuleManager::load_rules(rule_path)?,
        })
    }

    pub fn get_rule_sets(self) -> Vec<RuleSet> {
        self.rule_sets
    }
}
