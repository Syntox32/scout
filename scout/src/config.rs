use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Field weight for functions
    fw_functions: Option<f64>,
    /// Field weight for imports
    fw_imports: Option<f64>,
    /// Field weight for behavior
    fw_behavior: Option<f64>,
    /// Field weight for strings
    fw_strings: Option<f64>,

    /// TFIDF weight for functions
    tw_functions: Option<f64>,
    /// /// TFIDF weight for imports
    tw_imports: Option<f64>,
    /// /// TFIDF weight for behavior
    tw_behavior: Option<f64>,
    /// /// TFIDF weight for strings
    tw_strings: Option<f64>,
}
