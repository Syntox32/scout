use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use ast_walker::AstVisitor;
use rustpython_parser::ast::{Expression, Suite};
use rustpython_parser::ast::{ImportSymbol, Parameters};
use serde::{Deserialize, Serialize};

use super::{CallEntry, Location, VariableType};

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportEntry {
    pub module: String,
    pub symbol: Option<String>,
    pub location: Location,
    pub alias: Option<String>,
    pub context: String,
    pub is_dynamic: bool,
}

impl Hash for ImportEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.module.hash(state);
        self.location.row().hash(state);
        self.location.column().hash(state);
    }
}

impl PartialEq for ImportEntry {
    fn eq(&self, other: &Self) -> bool {
        (self.module == other.module)
            && (self.location.row() == other.location.row())
            && (self.location.column() == other.location.column())
    }
}
impl Eq for ImportEntry {}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ImportVisitor {
    imports: HashSet<ImportEntry>,
    aliases: HashMap<String, String>,
    count: HashMap<String, usize>,
    tf_idf: HashMap<String, f64>,
    call_context: Vec<String>, // TODO: change this to an enum?
}

impl ImportVisitor {
    pub fn new() -> Self {
        ImportVisitor {
            imports: HashSet::new(),
            aliases: HashMap::new(),
            count: HashMap::new(),
            tf_idf: HashMap::new(),
            call_context: vec![String::from("global")],
        }
    }

    pub fn get_imports(&self) -> &HashSet<ImportEntry> {
        trace!("imports: {:?}", &self.imports);
        &self.imports
    }

    pub fn get_aliases(&self) -> &HashMap<String, String> {
        &self.aliases
    }

    pub fn get_counts(&self) -> &HashMap<String, usize> {
        &self.count
    }

    pub fn has_import(&self, contains: &str) -> bool {
        self.count.contains_key(contains)
    }

    pub fn get_count(&self, import: &str) -> Option<usize> {
        Some(self.count.get(import)?.to_owned())
    }

    fn set_call_context(&mut self, ctx: String) {
        self.call_context.push(ctx);
    }

    fn get_call_context(&self) -> String {
        self.call_context.last().unwrap().to_string()
    }

    fn clear_call_context(&mut self) {
        self.call_context.pop();
    }

    fn add_to_count(&mut self, import: &str) {
        if let Some(count) = self.count.get_mut(&import.to_string()) {
            *count += 1;
        } else {
            self.count.insert(import.to_string(), 1);
        }
    }

    pub fn get_tfidf(&self, im: &str) -> Option<&f64> {
        self.tf_idf.get(im)
    }

    pub fn set_tfidf(&mut self, im: &str, tfidf: f64) {
        if !self.tf_idf.contains_key(im) {
            self.tf_idf.insert(im.to_owned(), tfidf);
        }
    }

    pub fn add_import(&mut self, entry: ImportEntry) {
        self.add_to_count(&entry.module);

        if let Some(a) = &entry.alias {
            if let Some(symbol) = &entry.symbol {
                self.aliases.insert(
                    a.to_string(),
                    format!("{}.{}", entry.module.to_string(), &symbol),
                );
            } else {
                self.aliases.insert(a.to_string(), entry.module.to_string());
            }
        }

        self.imports.insert(entry);
    }

    pub fn resolve_dynamic_imports(
        &mut self,
        entries: &Vec<CallEntry>,
        variables: &HashMap<String, VariableType>,
    ) {
        for entry in entries {
            if *entry.get_identifier() == String::from("__import__")
                || *entry.get_identifier() == String::from("importlib.import_module")
            {
                if let Some(arg) = entry.args.first() {
                    if let Some(import_name) = arg {
                        let mut import: String = String::from("");

                        if import_name.is_identifier() {
                            if let Some(key) = import_name.get_identifier() {
                                if let Some(val) = variables.get(key) {
                                    if val.is_string() {
                                        import = val.get_string().unwrap().to_string();
                                    }
                                }
                            }
                        } else if import_name.is_string() {
                            import = import_name.get_string().unwrap().to_string();
                        } else {
                            warn!("import was not identifier or string: {:?}", &import_name);
                        }

                        let entry = ImportEntry {
                            module: import,
                            symbol: None,
                            location: entry.location,
                            alias: None,
                            context: String::from("global"), // TODO: Change this to keep track of context in the call_visitor
                            is_dynamic: true,                // default
                        };

                        self.add_import(entry);
                    }
                }
            }
        }
    }
}

impl AstVisitor for ImportVisitor {
    // import os
    // import os.path as awdawd
    fn visit_import(
        &mut self,
        location: &rustpython_parser::ast::Location,
        names: &Vec<ImportSymbol>,
    ) {
        for name in names {
            let entry = ImportEntry {
                module: name.symbol.to_string(),
                symbol: None,
                location: Location::from_rustpython(*location),
                alias: name.alias.clone(),
                context: self.get_call_context(),
                is_dynamic: false, // default
            };

            self.add_import(entry);
        }
    }

    // from importlib import import_module as im
    fn visit_import_from(
        &mut self,
        location: &rustpython_parser::ast::Location,
        _level: &usize,
        module: &Option<String>,
        names: &Vec<ImportSymbol>,
    ) {
        for name in names {
            let full_name = match module {
                Some(m) => m.to_string(), //format!("{}.{}", m, name.symbol),
                None => name.symbol.to_string(),
            };
            trace!("full name: {}", &full_name);

            let entry = ImportEntry {
                module: full_name.clone(),
                symbol: Some(name.symbol.to_string()),
                location: Location::from_rustpython(*location),
                alias: name.alias.clone(),
                context: self.get_call_context(),
                is_dynamic: false, // default
            };

            self.add_import(entry);
        }
    }

    fn visit_function_def(
        &mut self,
        _is_async: bool,
        _name: &String,
        _args: &Box<Parameters>,
        body: &Suite,
        decorator_list: &Vec<Expression>,
        returns: &Option<Expression>,
    ) {
        self.set_call_context(String::from("function"));

        self.walk_statements(body);
        self.walk_expressions(decorator_list);
        self.walk_opt_expression(returns);

        self.clear_call_context();
    }
}
