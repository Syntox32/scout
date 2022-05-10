use std::{collections::HashMap, f64::consts::PI, hash::Hash};

use serde::{Deserialize, Serialize};

use crate::{evaluator::Hotspot, Config};

fn gaussian_density(x: f64, mu: f64, variance: f64) -> f64 {
    let sigma = variance.sqrt();
    (1f64 / (sigma * (2f64 * PI as f64).sqrt())) * ((-(x - mu).powi(2)) / (2f64 * variance)).exp()
}

fn mult(arr: &Vec<f64>, mul: &f64, target: &mut Vec<f64>) {
    for (a, t) in arr.iter().zip(target.iter_mut()) {
        *t += a * mul;
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Field {
    multiplier: f64,
    tfidf_weight: f64,
    loc: usize,
    x: Vec<f64>,
    y: Vec<f64>,
}

impl Field {
    fn new(multiplier: &f64, loc: usize, resolution: f64) -> Self {
        let num_points: u32 = ((loc as f64) / resolution) as u32;

        let mut x: Vec<f64> = Vec::with_capacity(num_points as usize);
        let mut y: Vec<f64> = Vec::with_capacity(num_points as usize);
        let mut curr: f64 = 0f64;
        for _ in 0..num_points {
            x.push(curr);
            y.push(0f64);
            curr += resolution;
        }

        Self {
            multiplier: *multiplier,
            tfidf_weight: 0.4,
            loc,
            x,
            y,
        }
    }

    fn tfidf_weight(tfidf_value: f64, weight: f64) -> f64 {
        // (1.0f64 - weight) * (current_value - tfidf_value) + tfidf_value
        1.0f64 - (1.0f64 - tfidf_value) * weight
    }

    fn add_density(&mut self, line: f64, variance: f64, tfidf_multiplier: f64, tfidf_weight: f64) {
        for (y, x) in self.y.iter_mut().zip(self.x.iter()) {
            *y += gaussian_density(*x, line, variance) * self.multiplier;
            *y *= Field::tfidf_weight(tfidf_multiplier, tfidf_weight);
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

    fn hotspots(&self, threshold: f64) -> Vec<Hotspot> {
        let mut spots: Vec<Hotspot> = vec![];

        let mut curr: Hotspot = Hotspot::new();
        let mut in_group: bool = false;

        for (y, x) in self.y.iter().zip(self.x.iter()) {
            if *y > threshold && !in_group {
                in_group = true;
                curr.startx = *x;
            } else if *y <= threshold && in_group {
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
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FieldType {
    Functions,
    Imports,
    Behavior,
    Strings,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DensityEvaluator {
    fields: HashMap<FieldType, Field>,
}

impl DensityEvaluator {
    const RESOLUTION: f64 = 0.5;
    const VARIANCE: f64 = 5.0;
    const HOTSPOT_THRESHOLD: f64 = 0.01;

    pub fn new(loc: usize, config: &Config) -> Self {
        let mut mult_map: HashMap<FieldType, f64> = HashMap::new();
        mult_map.insert(FieldType::Functions, config.fw_functions);
        mult_map.insert(FieldType::Imports, config.fw_imports);
        mult_map.insert(FieldType::Behavior, config.fw_behavior);
        mult_map.insert(FieldType::Strings, config.fw_strings);

        let mut fields: HashMap<FieldType, Field> = HashMap::new();

        fields.insert(
            FieldType::Functions,
            Field::new(
                mult_map.get(&FieldType::Functions).unwrap(),
                loc,
                DensityEvaluator::RESOLUTION,
            ),
        );
        fields.insert(
            FieldType::Imports,
            Field::new(
                mult_map.get(&FieldType::Imports).unwrap(),
                loc,
                DensityEvaluator::RESOLUTION,
            ),
        );
        fields.insert(
            FieldType::Behavior,
            Field::new(
                mult_map.get(&FieldType::Behavior).unwrap(),
                loc,
                DensityEvaluator::RESOLUTION,
            ),
        );
        fields.insert(
            FieldType::Strings,
            Field::new(
                mult_map.get(&FieldType::Strings).unwrap(),
                loc,
                DensityEvaluator::RESOLUTION,
            ),
        );

        Self { fields }
    }

    fn get_combined_field(&self) -> Field {
        let multiplier: f64 = 0.0; // this is ignored when requesting a combined field
        let fields = self.get_fields();

        let funcs = fields.get(&FieldType::Functions).unwrap();
        let imports = fields.get(&FieldType::Imports).unwrap();
        let behavior = fields.get(&FieldType::Behavior).unwrap();
        let strings = fields.get(&FieldType::Strings).unwrap();

        if (funcs.loc + imports.loc + behavior.loc + strings.loc) != funcs.loc * 4 {
            error!("Fields have different lines of code");
        }

        let mut combined_x: Vec<f64> = Vec::with_capacity(funcs.x.len());
        let mut combined_y: Vec<f64> = Vec::with_capacity(funcs.x.len());
        for x in &funcs.x {
            combined_x.push(x.to_owned());
            combined_y.push(0.0);
        }

        mult(&funcs.y, &funcs.multiplier, &mut combined_y);
        mult(&imports.y, &imports.multiplier, &mut combined_y);
        mult(&behavior.y, &behavior.multiplier, &mut combined_y);
        mult(&strings.y, &strings.multiplier, &mut combined_y);

        Field {
            multiplier: multiplier,
            tfidf_weight: multiplier,
            loc: funcs.loc,
            x: combined_x,
            y: combined_y,
        }
    }

    pub fn get_fields(&self) -> &HashMap<FieldType, Field> {
        &self.fields
    }

    pub fn add_density(
        &mut self,
        field_type: FieldType,
        row: usize,
        custom_multiplier: f64,
        tfidf_weight: f64,
    ) {
        let field = self.fields.get_mut(&field_type).unwrap();
        let line: f64 = row as f64;
        field.add_density(
            line,
            DensityEvaluator::VARIANCE,
            custom_multiplier,
            tfidf_weight,
        );
    }

    pub fn calculate_combined_field(&self) -> Field {
        self.get_combined_field()
    }

    pub fn hotspots(&self) -> Vec<Hotspot> {
        let field = self.calculate_combined_field();
        field.hotspots(DensityEvaluator::HOTSPOT_THRESHOLD)
    }
}

#[cfg(test)]
mod tests {
    use super::{gaussian_density, mult};

    #[test]
    fn test_gaussian_density() {
        let val = gaussian_density(11f64, 10f64, 1f64);
        println!("{}", val);
        assert_eq!(val, 0.24197072451914337f64);
    }

    #[test]
    fn test_mult() {
        let x: Vec<f64> = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let y: Vec<f64> = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let mut target: Vec<f64> = vec![];
        for _ in 0..11 {
            target.push(0f64);
        }

        mult(&x, &2.0, &mut target);
        assert_eq!(
            target,
            vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0]
        );

        mult(&y, &2.0, &mut target);
        assert_eq!(
            target,
            vec![0.0, 4.0, 8.0, 12.0, 16.0, 20.0, 24.0, 28.0, 32.0, 36.0, 40.0]
        );
    }
}
