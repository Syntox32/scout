use std::collections::hash_map::Keys;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use serde::Deserialize;

use crate::utils;

use crate::Result;

#[derive(Debug, Deserialize)]
pub struct CanaryInfo {
    pub identifier: String,
    pub transform: String,
}

#[derive(Debug, Deserialize)]
pub struct Canaries {
    canaries: HashMap<String, CanaryInfo>,
}

impl Canaries {
    pub const DEFAULT_CANARY_FILE: &'static str = "canary.json";
    const DEFAULT_CANARIES: &'static str = include_str!("canary.json");

    fn load_canaries(canary_path: &Option<String>) -> Result<HashMap<String, CanaryInfo>> {
        let canaries: String = match canary_path {
            Some(canary_path) => {
                let path = PathBuf::from_str(canary_path.as_str())?;
                trace!("Loading canaries from: '{}'", &canary_path);
                utils::load_from_file(path)?
            }
            None => {
                trace!("Using default ruleset: '{}'", Canaries::DEFAULT_CANARY_FILE);
                Canaries::DEFAULT_CANARIES.to_owned()
            }
        };

        let canaries: HashMap<String, CanaryInfo> = serde_json::from_str(canaries.as_str())?;
        Ok(canaries)
    }

    pub fn new(canary_path: &Option<String>) -> Result<Self> {
        Ok(Self {
            canaries: Canaries::load_canaries(canary_path)?,
        })
    }

    pub fn get_canaries(&self) -> &HashMap<String, CanaryInfo> {
        &self.canaries
    }
}
