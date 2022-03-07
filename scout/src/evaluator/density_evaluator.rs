use std::{f64::consts::PI, fs, path::PathBuf, process::Command, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::evaluator::Hotspot;

fn gaussian_density(x: f64, mu: f64, variance: f64) -> f64 {
    let sigma = variance.sqrt();
    (1f64 / (sigma * (2f64 * PI as f64).sqrt())) * ((-(x - mu).powi(2)) / (2f64 * variance)).exp()
}

#[derive(Serialize, Deserialize)]
struct Plot {
    x: Vec<f64>,
    y: Vec<f64>,
}

#[derive(Debug)]
pub struct DensityEvaluator {
    pub loc: usize,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

impl DensityEvaluator {
    const RESOLUTION: f64 = 0.1;

    pub fn new(loc: usize) -> Self {
        let num_points: u32 = ((loc as f64) / DensityEvaluator::RESOLUTION) as u32;

        let mut x: Vec<f64> = Vec::with_capacity(num_points as usize);
        let mut y: Vec<f64> = Vec::with_capacity(num_points as usize);
        let mut curr: f64 = 0f64;
        for _ in 0..num_points {
            x.push(curr);
            y.push(0f64);
            curr += DensityEvaluator::RESOLUTION;
        }

        Self { loc, x, y }
    }

    pub fn add_density(&mut self, row: usize) {
        let variance: f64 = 5.0;
        let line: f64 = row as f64;
        // println!("line: {}, variance: {}", line, variance);

        for (y, x) in self.y.iter_mut().zip(self.x.iter_mut()) {
            *y += gaussian_density(*x, line, variance);
        }
    }

    // get maximum Y value in the range [startX, endX]
    fn get_max_y(&self, start: f64, end: f64) -> Option<f64> {
        if self.y.is_empty() {
            return None;
        }

        let (_, maxy) = self
            .x
            .iter()
            .zip(self.y.iter())
            .filter(|(&x, _)| x >= start && x <= end)
            .max_by(|(_, y1), (_, y2)| y1.partial_cmp(y2).unwrap())?;
        Some(*maxy)
    }

    pub fn hotspots(&self) -> Vec<Hotspot> {
        let mut spots: Vec<Hotspot> = vec![];
        const THRESHOLD: f64 = 0.01;

        let mut curr: Hotspot = Hotspot::new();
        let mut in_group: bool = false;

        for (y, x) in self.y.iter().zip(self.x.iter()) {
            if *y > THRESHOLD && !in_group {
                in_group = true;
                curr.startx = *x;
            } else if *y <= THRESHOLD && in_group {
                in_group = false;
                curr.endx = *x;
                curr.peak = self.get_max_y(curr.startx, curr.endx).unwrap_or(0.0);
                spots.push(curr);
                curr = Hotspot::new();
            }
        }

        if in_group && !self.x.is_empty() {
            let lastx = self.x.last().unwrap();
            curr.endx = *lastx;
            curr.peak = self.get_max_y(curr.startx, curr.endx).unwrap_or(0.0);
            spots.push(curr);
        }

        spots
    }

    // debug function only
    pub fn _plot(&self) {
        let plt = Plot {
            x: self.x.clone(),
            y: self.y.clone(),
        };
        let json = serde_json::to_string(&plt).unwrap();
        fs::write(&PathBuf::from_str("conf/plot.json").unwrap(), &json).unwrap();

        let _ = Command::new("python3")
            .arg("../ast_experiment/plot.py")
            .arg("--file")
            .arg("../engine/conf/plot.json")
            .spawn()
            .expect("failed to execute process");
    }
}

#[cfg(test)]
mod tests {
    use super::gaussian_density;

    #[test]
    fn test_gaussian_density() {
        let val = gaussian_density(11f64, 10f64, 1f64);
        println!("{}", val);
        assert_eq!(val, 0.24197072451914337f64);
    }
}
