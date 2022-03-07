use std::collections::HashMap;

use ast_walker::AstVisitor;
use rustpython_parser::{
    ast::{ExpressionType, ImportSymbol, Located, Parameters, StatementType},
    location::Location,
};

#[derive(Debug)]
pub struct ImportEntry {
    pub module: String,
    pub location: Location,
    pub alias: Option<String>,
    pub context: String,
}

#[derive(Debug)]
pub struct ImportVisitor {
    pub imports: HashMap<String, ImportEntry>,
    pub aliases: HashMap<String, String>,
    call_context: Vec<String>,
}

impl ImportVisitor {
    pub fn new() -> Self {
        let imports: HashMap<String, ImportEntry> = HashMap::new();
        let aliases: HashMap<String, String> = HashMap::new();

        ImportVisitor {
            imports,
            aliases,
            call_context: vec![String::from("global")],
        }
    }

    pub fn get_imports(&self) -> &HashMap<String, ImportEntry> {
        &self.imports
    }

    pub fn get_aliases(&self) -> &HashMap<String, String> {
        &self.aliases
    }

    pub fn set_call_context(&mut self, ctx: String) {
        self.call_context.push(ctx);
    }

    pub fn get_call_context(&self) -> String {
        self.call_context.last().unwrap().to_string()
    }

    pub fn clear_call_context(&mut self) {
        self.call_context.pop();
    }

    #[allow(unused)]
    pub fn has_import(&self, contains: &str) -> bool {
        // self.imports.contains_key(&contains.to_owned())
        for key in self.imports.keys() {
            if key.as_str() == contains {
                return true;
            }
        }
        false
    }

    // pub fn _get_imports(&self) -> Vec<String> {
    //     self.imports
    //         .iter()
    //         .map(|(module, _)| module.clone().to_string())
    //         .collect::<Vec<String>>()
    // }
}

impl AstVisitor for ImportVisitor {
    // import os
    // import os.path as awdawd
    fn visit_import(&mut self, stmt: &Located<StatementType>, names: &[ImportSymbol]) {
        for name in names {
            let entry = ImportEntry {
                module: name.symbol.to_string(),
                location: stmt.location,
                alias: name.alias.clone(),
                context: self.get_call_context(),
            };
            self.imports.insert(name.symbol.to_string(), entry);

            if let Some(a) = name.alias.clone() {
                self.aliases.insert(a.to_string(), name.symbol.to_string());
            }
        }
    }

    // from importlib import import_module as im
    fn visit_import_from(
        &mut self,
        stmt: &Located<StatementType>,
        _level: &usize,
        module: &Option<String>,
        names: &[ImportSymbol],
    ) {
        for name in names {
            let full_name = match module {
                Some(m) => format!("{}.{}", m, name.symbol),
                None => name.symbol.to_string(),
            };

            let entry = ImportEntry {
                module: full_name.clone(),
                location: stmt.location,
                alias: name.alias.clone(),
                context: self.get_call_context(),
            };

            self.imports.insert(full_name.clone(), entry);

            if let Some(a) = name.alias.clone() {
                self.aliases.insert(a.to_string(), full_name.clone());
            }
        }
    }

    fn visit_function_def(
        &mut self,
        _is_async: &bool,
        _name: &str,
        _args: &Parameters,
        body: &[Located<StatementType>],
        decorator_list: &[Located<ExpressionType>],
        returns: &Option<Located<ExpressionType>>,
    ) {
        self.set_call_context(String::from("function"));

        self.walk_statements(body);
        self.walk_expressions(decorator_list);
        self.walk_opt_expression(returns);

        self.clear_call_context();
    }
}
