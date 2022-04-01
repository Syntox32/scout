use std::{collections::HashMap, mem};

use ast_walker::AstVisitor;
use rustpython_parser::ast::{Expression, ExpressionType, Operator};
use serde::{Deserialize, Serialize};

use crate::utils::{
    ast::{do_binop, try_identifier, try_to_string},
    format_empty_arg,
};

use super::Location;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VariableType {
    Identifier(String),
    Str(String),
    Dict(HashMap<String, VariableType>),
    List(Vec<Option<VariableType>>),
    Tuple(Vec<Option<VariableType>>),
}

impl VariableType {
    pub fn get_string(&self) -> Option<&String> {
        match self {
            VariableType::Str(str) => Some(str),
            _ => None,
        }
    }

    pub fn get_identifier(&self) -> Option<&String> {
        match self {
            VariableType::Identifier(ident) => Some(ident),
            _ => None,
        }
    }

    pub fn is_identifier(&self) -> bool {
        match *self {
            VariableType::Identifier(..) => true,
            _ => false,
        }
    }

    pub fn is_string(&self) -> bool {
        match *self {
            VariableType::Str(..) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VariableVisitor {
    variables: HashMap<String, VariableType>,
    locations: HashMap<String, Location>,
}

impl VariableVisitor {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            locations: HashMap::new(),
        }
    }

    pub fn get_variables(&self) -> &HashMap<String, VariableType> {
        &self.variables
    }

    pub fn get_locations(&self) -> &HashMap<String, Location> {
        &self.locations
    }

    #[allow(unused)]
    fn get_values_from_expr(&self, expr: &Expression) -> Option<VariableType> {
        match &expr.node {
            ExpressionType::BoolOp { op, values } => None,
            ExpressionType::Binop { a, op, b } => {
                Some(VariableType::Str(format_empty_arg(&try_to_string(&expr))))
            }
            ExpressionType::Subscript { a, b } => None,
            ExpressionType::Unop { op, a } => None,
            ExpressionType::Await { value } => None,
            ExpressionType::Yield { value } => None,
            ExpressionType::YieldFrom { value } => None,
            ExpressionType::Compare { vals, ops } => None,
            ExpressionType::Attribute { value, name } => None,
            ExpressionType::Call {
                function,
                args,
                keywords,
            } => None,
            // self.visit_call(function, args, keywords);
            ExpressionType::Number { value } => None,
            ExpressionType::List { elements } => Some(VariableType::List(
                elements
                    .iter()
                    .map(|expr| {
                        if let Some(str) = try_to_string(&expr) {
                            Some(VariableType::Str(str))
                        } else {
                            None
                        }
                    })
                    .collect(),
            )),
            ExpressionType::Tuple { elements } => None,
            ExpressionType::Dict { elements } => None,
            ExpressionType::Set { elements } => None,
            ExpressionType::Comprehension { kind, generators } => None,
            ExpressionType::Starred { value } => None,
            ExpressionType::Slice { elements } => None,
            ExpressionType::String { value } => {
                Some(VariableType::Str(format_empty_arg(&try_to_string(&expr))))
            }
            ExpressionType::Bytes { value } => None,
            ExpressionType::Identifier { name } => None,
            ExpressionType::Lambda { args, body } => None,
            ExpressionType::IfExpression { test, body, orelse } => None,
            ExpressionType::NamedExpression { left, right } => None,
            ExpressionType::True => None,
            ExpressionType::False => None,
            ExpressionType::None => None,
            ExpressionType::Ellipsis => None,
        }
    }
}

impl AstVisitor for VariableVisitor {
    fn visit_assign(&mut self, target: &Vec<Expression>, value: &Expression) {
        let values = self.get_values_from_expr(value);

        for (t, v) in target.iter().zip(values.into_iter()) {
            if let Some(ident) = try_to_string(t) {
                self.variables.insert(ident.clone(), v);
                self.locations
                    .insert(ident, Location::from_rustpython(t.location));
            }
        }

        self.walk_expressions(target);
        self.walk_expression(value);
    }

    fn visit_aug_assign(&mut self, target: &Expression, op: &Operator, value: &Expression) {
        let value_value = match try_identifier(value) {
            Some(ident) => {
                match self.variables.get(&ident) {
                    Some(ident_val) => Some(ident_val.clone()),
                    None => {
                        // An identifier was encountered but it was not in our variable list?
                        let identifier = format_empty_arg(&try_to_string(value));
                        warn!(
                            "An identifier was encountered but it was not in our variable list: {}",
                            identifier
                        );
                        None
                    }
                }
            }
            None => {
                // it's some other value or expression
                match try_to_string(value) {
                    Some(val) => Some(VariableType::Str(val)),
                    None => None,
                }
            }
        };

        if let Some(target_ident) = try_to_string(target) {
            if let Some(target_val) = self.variables.get_mut(&target_ident) {
                let new_target_val = if let Some(value_value) = value_value {
                    if mem::discriminant(target_val) == mem::discriminant(&value_value) {
                        match target_val {
                            VariableType::Str(target_val) => {
                                let value_value = match value_value {
                                    VariableType::Str(value_value) => Some(value_value),
                                    VariableType::Dict(_) => None,
                                    VariableType::List(_) => None,
                                    VariableType::Tuple(_) => None,
                                    VariableType::Identifier(_) => None,
                                };

                                if let Some(value_value) = value_value {
                                    let result = do_binop(target_val.to_string(), value_value, op);
                                    result
                                } else {
                                    None
                                }
                            }
                            VariableType::Dict(_) => None,
                            VariableType::List(_) => None,
                            VariableType::Tuple(_) => None,
                            VariableType::Identifier(_) => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(new_value) = new_target_val {
                    *target_val = VariableType::Str(new_value);
                }
            }
        }

        self.walk_expression(target);
        self.walk_expression(value);
    }

    fn visit_ann_assign(
        &mut self,
        target: &Box<Expression>,
        annotation: &Box<Expression>,
        value: &Option<Expression>,
    ) {
        self.walk_expression(target);
        self.walk_expression(annotation);
        self.walk_opt_expression(value);
    }

    fn visit_global(&mut self, _names: &Vec<String>) {}
    fn visit_nonlocal(&mut self, _names: &Vec<String>) {}
}
