use std::{collections::HashMap, f64::consts::PI, hash::Hash};

use serde::{Deserialize, Serialize};

use crate::evaluator::Hotspot;

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
            loc,
            x,
            y,
        }
    }

    fn new_from(fields: &HashMap<FieldType, Field>) -> Self {
        let multiplier: f64 = 0.0;

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

        Self {
            multiplier: multiplier,
            loc: funcs.loc,
            x: combined_x,
            y: combined_y,
        }
    }

    fn add_density(&mut self, line: f64, variance: f64) {
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
    const RESOLUTION: f64 = 1.0;
    const VARIANCE: f64 = 5.0;
    const HOTSPOT_THRESHOLD: f64 = 0.01;

    pub fn new(loc: usize) -> Self {
        let mut mult_map: HashMap<FieldType, f64> = HashMap::new();
        mult_map.insert(FieldType::Functions, 1.0);
        mult_map.insert(FieldType::Imports, 1.0);
        mult_map.insert(FieldType::Behavior, 1.0);
        mult_map.insert(FieldType::Strings, 1.0);

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

    pub fn get_fields(&self) -> &HashMap<FieldType, Field> {
        &self.fields
    }

    pub fn add_density(&mut self, field_type: FieldType, row: usize) {
        let field = self.fields.get_mut(&field_type).unwrap();
        let line: f64 = row as f64;
        field.add_density(line, DensityEvaluator::VARIANCE);
    }

    pub fn calculate_combined_field(&self) -> Field {
        Field::new_from(&self.fields)
    }

    pub fn hotspots(&self) -> Vec<Hotspot> {
        let field = self.calculate_combined_field();
        field.hotspots(DensityEvaluator::HOTSPOT_THRESHOLD)
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
