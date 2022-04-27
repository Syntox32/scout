use std::path::PathBuf;
use std::str::FromStr;

use serde::Deserialize;

use crate::utils;
use crate::Result;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub use_cache: bool,
    pub save_cache: bool,

    /// Field weight for functions
    pub fw_functions: f64,
    /// Field weight for imports
    pub fw_imports: f64,
    /// Field weight for behavior
    pub fw_behavior: f64,
    /// Field weight for strings
    pub fw_strings: f64,

    /// TFIDF weight for functions
    pub tw_functions: f64,
    /// /// TFIDF weight for imports
    pub tw_imports: f64,
    /// /// TFIDF weight for behavior
    // pub tw_behavior: f64,
    // /// /// TFIDF weight for strings
    // pub tw_strings: f64,
    pub feature_tfidf_calls: bool,
    pub feature_tfidf_imports: bool,
}

impl Config {
    pub const DEFAULT_CONFIG_FILE: &'static str = "config.jsonc";
    const DEFAULT_CONFIG: &'static str = include_str!("config.jsonc");

    fn load_config(canary_path: &Option<String>) -> Result<Config> {
        let config: String = match canary_path {
            Some(canary_path) => {
                let path = PathBuf::from_str(canary_path.as_str())?;
                trace!("Loading canaries from: '{}'", &canary_path);
                utils::load_from_file(path)?
            }
            None => {
                trace!("Using default ruleset: '{}'", Config::DEFAULT_CONFIG_FILE);
                Config::DEFAULT_CONFIG.to_owned()
            }
        };

        Config::parse_json(config)
    }

    fn parse_json(json: String) -> Result<Config> {
        let config: Config = serde_json::from_str(json.as_str())?;
        Ok(config)
    }

    pub fn new(config_path: &Option<String>) -> Result<Self> {
        Ok(Config::load_config(config_path)?)
    }

    pub fn from_str(config_json: String) -> Result<Self> {
        Config::parse_json(config_json)
    }
}
 