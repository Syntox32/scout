use crate::utils::format_empty_arg;

use ast_walker::AstVisitor;
use rustpython_parser::{
    ast::{ExpressionType, Keyword, Located},
    location::Location,
};
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

use super::import_visitor::ImportEntry;

#[derive(Debug)]
pub struct CallEntry {
    pub full_identifier: String,
    pub location: Location,
}

impl CallEntry {
    pub fn get_identifier(&self) -> &str {
        self.full_identifier
            .split_once(".")
            .unwrap_or_else(|| (&self.full_identifier, ""))
            .0
    }
}

impl Hash for CallEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.full_identifier.hash(state);
        self.location.row().hash(state);
        self.location.column().hash(state);
    }
}

impl PartialEq for CallEntry {
    fn eq(&self, other: &Self) -> bool {
        (self.full_identifier == other.full_identifier)
            && (self.location.row() == other.location.row())
            && (self.location.column() == other.location.column())
    }
}

impl Eq for CallEntry {}

#[derive(Debug)]
pub struct CallVisitor {
    pub entries: Vec<CallEntry>,
    pub errors: Vec<(String, Location)>,
}

impl CallVisitor {
    pub fn new() -> Self {
        Self {
            entries: vec![],
            errors: vec![],
        }
    }

    // this is used by the tests, but it can't find it
    #[allow(unused)]
    pub fn has_function(&self, function: &str) -> bool {
        for entry in &self.entries {
            if entry.full_identifier.ends_with(function) {
                return true;
            }
        }
        false
    }

    pub fn resolve_imports(
        &mut self,
        imports: &HashMap<String, ImportEntry>,
        aliases: &HashMap<String, String>,
    ) {
        for entry in self.entries.iter_mut() {
            if aliases.contains_key(entry.get_identifier()) {
                if let Some(import) = imports.get(aliases.get(entry.get_identifier()).unwrap()) {
                    let old = entry.full_identifier.clone();
                    entry.full_identifier = entry
                        .full_identifier
                        .replace(entry.get_identifier(), import.module.as_str());
                    trace!(
                        "Resolving import for function: '{}' -> '{}'",
                        old,
                        entry.full_identifier
                    );
                } else {
                    error!(
                        "Fetching import from alias '{}' did not work.",
                        entry.get_identifier()
                    );
                }
            }
        }
    }

    pub fn get_absolute_identifier(&mut self, expr: &Located<ExpressionType>) -> Option<String> {
        match &expr.node {
            ExpressionType::Identifier { name } => Some(name.to_owned()),
            ExpressionType::Attribute { name, value } => {
                Some(format!("{}.{}", self.get_absolute_identifier(value)?, name))
            }
            ExpressionType::Call {
                function,
                args,
                keywords,
            } => {
                self.visit_call(function, args, keywords);
                Some(format_empty_arg(&None))
            }
            _ => {
                let e = format!(
                    "get_absolute_identifier cannot handle expression type: {}",
                    expr.name()
                );
                trace!("{}", &e);
                self.errors.push((e, expr.location));
                None
            }
        }
    }
}

impl AstVisitor for CallVisitor {
    fn visit_call(
        &mut self,
        function: &Located<ExpressionType>,
        args: &[Located<ExpressionType>],
        keywords: &[Keyword],
    ) {
        let func = match &function.node {
            ExpressionType::Call {
                function,
                args,
                keywords,
            } => {
                self.visit_call(function, args, keywords);
                None
            }
            _ => self.get_absolute_identifier(function),
        };

        if let Some(f) = func {
            let entry = CallEntry {
                full_identifier: f,
                location: function.location,
            };

            self.entries.push(entry);
        }

        // boilerplate
        self.walk_expression(function);
        args.iter().for_each(|arg| self.walk_expression(arg));
        keywords
            .iter()
            .for_each(|kw| self.walk_expression(&kw.value));
    }
}

// fn do_binop(&self, a: String, b: String, op: &Operator) -> Option<String> {
//     match op {
//         Operator::Add => Some(format!("{}{}", a.to_owned(), b.to_owned())),
//         _ => None, //format!("{} binop {}", a, b)
//     }
// }

// fn resolve_string_group(&self, value: &StringGroup) -> Option<String> {
//     match value {
//         StringGroup::Constant { value } => Some(value.to_owned()),
//         _ => None, //String::from("unsupported by resolve_string_group") }
//     }
// }

// fn resolve_binop(&self, bin_expr: &Located<ExpressionType>) -> Option<String> {
//     match &bin_expr.node {
//         ExpressionType::Binop { a, b, op } => match &a.deref().node {
//             ExpressionType::String { value } => {
//                 let a_str = self.resolve_string_group(&value)?;

//                 match &b.deref().node {
//                     ExpressionType::String { value } => {
//                         let b_str = self.resolve_string_group(&value)?;

//                         self.do_binop(a_str, b_str, op)
//                     }
//                     _ => None,
//                 }
//             }
//             _ => None,
//         },
//         _ => panic!(
//             "resolve_binop only expects bin_op expressions: {}",
//             bin_expr.name()
//         ),
//     }
// }

// fn resolve_args(&mut self, args: &Vec<Located<ExpressionType>>) -> Vec<String> {
//     let results: Vec<Option<String>> = args
//         .iter()
//         .map(|arg| match &arg.node {
//             ExpressionType::Call { .. } => self.resolve_call(arg),
//             ExpressionType::Binop { .. } => self.resolve_binop(arg),
//             ExpressionType::String { value } => self.resolve_string_group(&value),
//             _ => None,
//         })
//         .collect();

//     results
//         .iter()
//         .map(|s| self.format_empty_arg(s))
//         .collect::<Vec<String>>()
// }

// fn resolve_call(&mut self, call_expr: &Located<ExpressionType>) -> Option<String> {
//     match &call_expr.node {
//         ExpressionType::Call {
//             function,
//             args,
//             keywords,
//         } => {
//             let fname = get_absolute_identifier(function.as_ref())?;
//             let fargs = self.resolve_args(&args);
//             let f = format!("{}({})", fname, fargs.join(", "));
//             // self.function_calls.insert(fname, fargs);
//             return Some(f);
//         }
//         _ => panic!(
//             "resolve_call only expects call expressions: {}",
//             call_expr.name()
//         ),
//     }
// }

// fn find_function_visitor(&mut self) -> io::Result<()> {
//     for statement in self.get_statements()? {
//         match statement.node {
//             StatementType::Expression { expression } => match expression.node {
//                 ExpressionType::Attribute { value: _, name } => {
//                     println!("attr name: {}", name);
//                 }
//                 ExpressionType::Identifier { name } => {
//                     println!("attr name: {}", name);
//                 }
//                 ExpressionType::Call { .. } => {
//                     self.resolve_call(&expression);
//                 }
//                 _ => println!("unhandled expr name: {}", &expression.name()),
//             },
//             _ => {}
//         }
//     }
//     Ok(())
// }
