use serde::{Deserialize, Serialize};

use crate::SourceFile;

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

    pub fn get_code<'a>(&self, source: &'a SourceFile) -> Vec<(usize, &'a str)> {
        let hotspot_code: Vec<(usize, &str)> = source
            .source
            .lines()
            .enumerate()
            .collect::<Vec<(usize, &str)>>()
            .drain(self.line_low()..self.line_high())
            .collect::<Vec<(usize, &str)>>();
        hotspot_code
    }
}
