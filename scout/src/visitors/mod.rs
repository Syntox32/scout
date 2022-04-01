mod call_visitor;
mod import_visitor;
mod variable_visitor;

pub use call_visitor::{CallEntry, CallVisitor};
pub use import_visitor::ImportEntry;
pub(crate) use import_visitor::ImportVisitor;

use rustpython_parser::location;
use serde::{Deserialize, Serialize};
pub use variable_visitor::{VariableType, VariableVisitor};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Location {
    pub(crate) row: usize,
    pub(crate) column: usize,
}

impl Location {
    pub fn row(&self) -> usize {
        self.row
    }

    pub fn column(&self) -> usize {
        self.column
    }

    pub fn from_rustpython(location: location::Location) -> Self {
        Self {
            row: location.row(),
            column: location.column(),
        }
    }
}
