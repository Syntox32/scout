use crate::utils::format_empty_arg;

use ast_walker::AstVisitor;
use rustpython_parser::{
    ast::{ExpressionType, Keyword, Located, Operator, StringGroup},
    location::Location,
};
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

#[derive(Debug)]
pub struct CallEntry {
    pub full_identifier: String,
    pub location: Location,
    pub args: Vec<Option<String>>,
}

impl CallEntry {
    /// Returns the identifier of a full_identifier.
    /// Example full_identifier "<identifier>.<attribute>.<attribute>" returns "<identifier>"
    /// In the case of only having an identifier as full identifier, it will just return this.
    pub fn get_base_identifier(&self) -> &str {
        self.full_identifier
            .split_once(".")
            .unwrap_or_else(|| (&self.full_identifier, ""))
            .0
    }

    pub fn get_identifier(&self) -> &String {
        &self.full_identifier
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

    pub fn get_entries(&self) -> &Vec<CallEntry> {
        &self.entries
    }

    pub fn resolve_imports(&mut self, aliases: &HashMap<String, String>) {
        for entry in self.entries.iter_mut() {
            if aliases.contains_key(entry.get_base_identifier()) {
                let module_identifier = aliases.get(entry.get_base_identifier()).unwrap();
                let old = entry.full_identifier.clone();
                entry.full_identifier = entry
                    .full_identifier
                    .replace(entry.get_base_identifier(), module_identifier);
                trace!(
                    "Resolving import for function: '{}' -> '{}'",
                    old,
                    entry.full_identifier
                );
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

    fn try_to_string(&self, expr: &Located<ExpressionType>) -> Option<String> {
        match &expr.node {
            // ExpressionType::Call { .. } => self.resolve_call(arg),
            ExpressionType::Binop { a, op, b } => self.resolve_binop(a, b, op),
            ExpressionType::String { value } => self.resolve_string_group(&value),
            _ => None,
        }
    }

    fn resolve_args(&mut self, args: &[Located<ExpressionType>]) -> Vec<Option<String>> {
        // trace!("{:#?}", args);
        let results: Vec<Option<String>> = args
            .iter()
            .map(|arg| self.try_to_string(arg))
            .collect();
        results
    }

    fn resolve_string_group(&self, value: &StringGroup) -> Option<String> {
        match value {
            StringGroup::Constant { value } => Some(value.to_owned()),
            _ => None, //String::from("unsupported by resolve_string_group") }
        }
    }

    fn resolve_binop(
        &self,
        a: &Box<Located<ExpressionType>>,
        b: &Box<Located<ExpressionType>>,
        op: &Operator,
    ) -> Option<String> {

        let aa = self.try_to_string(a)?;
        let bb = self.try_to_string(b)?;
        self.do_binop(aa, bb, op)
    }

    fn do_binop(&self, a: String, b: String, op: &Operator) -> Option<String> {
        // trace!("doing bin op: {} {:?} {}", a, op, b);
        match op {
            Operator::Add => Some(format!("{}{}", a.to_owned(), b.to_owned())),
            _ => None, //format!("{} binop {}", a, b)
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
            let args = self.resolve_args(args);
            trace!("args for func {} = {:?}", f, args);
            let entry = CallEntry {
                full_identifier: f,
                location: function.location,
                args,
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
