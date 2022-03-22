mod evaluator;
mod package;
mod source;
mod utils;
mod visitors;

#[macro_use]
extern crate log;

pub use evaluator::{Evaluator, RuleManager, SourceAnalysis};
pub use package::Package;
pub use source::SourceFile;

pub use engine::Engine;

use std::error;
pub type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

mod engine {

    use std::{path::PathBuf, str::FromStr};

    use crate::evaluator::AnalysisResult;
    use crate::Result;
    use crate::{Package, RuleManager};

    pub struct Engine {
        rule_path: String,

        opt_show_all: bool,
        opt_threshold: f64,
    }

    impl<'e> Engine {
        pub fn get_default_rule_file() -> String {
            RuleManager::DEFAULT_RULE_FILE.to_string()
        }

        pub fn new() -> Self {
            let rule_path = RuleManager::DEFAULT_RULE_FILE.to_string();

            Engine {
                rule_path,
                opt_show_all: false,
                opt_threshold: 0.0,
            }
        }

        pub fn set_show_all(mut self, show_all: bool) -> Self {
            self.opt_show_all = show_all;
            self
        }

        pub fn set_threshold(mut self, threshold: f64) -> Self {
            self.opt_threshold = threshold;
            self
        }

        pub fn set_rule_path(mut self, rule_path: &str) -> Self {
            self.rule_path = rule_path.to_string();
            self
        }

        fn get_rule_manager(&self) -> Result<RuleManager> {
            Ok(RuleManager::new(self.rule_path.as_str())?)
        }

        pub fn analyse_package(self, path: &str) -> Result<AnalysisResult> {
            trace!("Analysing package: '{}'", &path);
            let pkg = match Package::locate_package(&path) {
                Some(path) => {
                    debug!("Detected package: '{:?}'", &path);
                    path
                }
                None => return Err("Could not detect package".into()),
            };

            let rule_manager = match self.get_rule_manager() {
                Ok(rm) => rm,
                Err(err) => {
                    return Err(format!(
                        "Rule manager could not be initalized: {}",
                        err.to_string()
                    )
                    .into())
                }
            };

            let results =
                Package::new(pkg, rule_manager, self.opt_threshold, self.opt_show_all).analyse()?;
            Ok(results)
        }

        pub fn analyse_file(self, path: &str) -> Result<AnalysisResult> {
            let path = PathBuf::from_str(path)?;

            let rule_manager = match self.get_rule_manager() {
                Ok(rm) => rm,
                Err(err) => {
                    return Err(format!(
                        "Rule manager could not be initalized: {}",
                        err.to_string()
                    )
                    .into())
                }
            };

            let results = Package::new(path, rule_manager, self.opt_threshold, self.opt_show_all)
                .analyse_single()?;
            Ok(results)
        }
    }
}
