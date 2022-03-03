use std::{path::PathBuf, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::utils;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Functionality {
    Encryption,
    Encoding,
    Compression,
    Network,
    Process,
    FileSystem,
    System,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Rule {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Rules(Vec<RuleSet>);

pub struct RuleManager {
    rule_sets: Vec<RuleSet>,
}

impl RuleManager {
    const RULE_FILE: &'static str = "conf/rules.ron";

    fn load_rules() -> Vec<RuleSet> {
        trace!("Reading rule file: '{}'", RuleManager::RULE_FILE);
        let rules_content =
            utils::load_from_file(&PathBuf::from_str("conf/rules.ron").unwrap()).unwrap();
        let Rules(rulesets) = ron::from_str(rules_content.as_str()).expect("failed to load rules");
        trace!("Loaded rulesets: {:?}", &rulesets);
        rulesets
    }

    pub fn new() -> Self {
        Self {
            rule_sets: RuleManager::load_rules(),
        }
    }

    pub fn get_rule_sets(&self) -> &Vec<RuleSet> {
        &self.rule_sets
    }
}
