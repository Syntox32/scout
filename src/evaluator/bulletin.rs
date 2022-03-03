use super::Functionality;
use rustpython_parser::location::Location;

#[derive(Debug)]
pub enum BulletinReason {
    SuspiciousImport,
    SuspiciousFunction,
    ImportInsideFunction,
}

#[derive(Debug)]
pub enum BulletinSeverity {
    // FixNow,
    Suspicious,
    Informative,
}

#[derive(Debug)]
pub struct Bulletin {
    pub identifier: String,
    pub location: Location,
    #[allow(unused)]
    severity: BulletinSeverity,
    reason: BulletinReason,
    pub functionality: Option<Functionality>,
    pub threshold: f64,
}

pub type Bulletins = Vec<Bulletin>;

impl Bulletin {
    pub fn new(
        identifier: String,
        reason: BulletinReason,
        severity: BulletinSeverity,
        location: Location,
        functionality: Option<Functionality>,
        threshold: f64,
    ) -> Self {
        Self {
            identifier,
            reason,
            location,
            severity,
            functionality,
            threshold,
        }
    }

    pub fn line(&self) -> usize {
        self.location.row()
    }

    pub fn col(&self) -> usize {
        self.location.column()
    }

    pub fn reason(&self) -> String {
        match self.reason {
            BulletinReason::SuspiciousImport => format!(
                "The import '{}' is often used in malicious activity",
                self.identifier
            ),
            BulletinReason::SuspiciousFunction => {
                format!(
                    "The function '{}' is often used in malicious activity",
                    self.identifier
                )
            }
            BulletinReason::ImportInsideFunction => {
                "Importing inside classes and functions might be done to hide functionality"
                    .to_string()
            }
        }
    }

    // pub fn severity(&self) -> &'static str {
    //     match self.severity {
    //         // BulletinSeverity::FixNow => "FixNow",
    //         BulletinSeverity::Suspicious => "Suspicious",
    //         BulletinSeverity::Informative => "Informative",
    //     }
    // }

    // pub fn generate(&self, eval_result: &EvaluatorResult) -> String {
    //     let mut notif = String::new();
    //     notif.push_str(format!("[{}] {}\n", self.severity(), self.reason()).as_str());
    //     notif.push_str(
    //         format!(
    //             "\tAt {} in {}\n",
    //             self.location,
    //             eval_result.source.get_path()
    //         )
    //         .as_str(),
    //     );
    //     notif.push_str(
    //         format!(
    //             "\t| {}\n",
    //             self.get_code_snippet(&eval_result.source.source)
    //         )
    //         .as_str(),
    //     );
    //     notif
    // }

    // fn get_code_snippet(&self, source_code: &str) -> String {
    //     let row: usize = self.location.row();
    //     source_code
    //         .lines()
    //         .collect::<Vec<&str>>()
    //         .get(row - 1)
    //         .map(|&line| line.to_string())
    //         .unwrap_or_else(|| String::from("<failed to get code>"))
    // }
}
