use serde::{Deserialize, Serialize};

use crate::{utils, SourceFile};

#[derive(Debug, Serialize, Deserialize)]
pub struct Hotspot {
    pub startx: f64,
    pub endx: f64,
    pub peak: f64,
}

impl Hotspot {
    pub fn new() -> Self {
        Self {
            startx: 0.0,
            endx: 0.0,
            peak: 0.0,
        }
    }

    pub fn peak(&self) -> f64 {
        self.peak
    }

    pub fn line_low(&self) -> usize {
        self.startx.round() as usize
    }

    pub fn line_high(&self) -> usize {
        self.endx.round() as usize
    }

    pub fn get_code<'a>(&self, source: &'a SourceFile) -> Vec<(usize, String)> {
        let source = utils::load_from_file(&source.source_path).unwrap();
        let hotspot_code: Vec<(usize, String)> = source
            .lines()
            .enumerate()
            .map(|(idx, s)| (idx, s.to_owned()))
            .collect::<Vec<(usize, String)>>()
            .drain(self.line_low()..self.line_high())
            .collect::<Vec<(usize, String)>>();
        hotspot_code
    }
}
