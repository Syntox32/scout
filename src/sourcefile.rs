use rustpython_parser::ast::{
    ExpressionType, ImportSymbol, Keyword, Located, Operator, Program, Statement, StatementType,
    StringGroup,
};
use rustpython_parser::parser;
use std::collections::HashMap;
use std::io;
use std::ops::Deref;
use std::path::PathBuf;

use crate::ast_visitor::{AstVisitor, AstWalker};
use crate::utils;

fn get_absolute_identifier(expr: &Located<ExpressionType>) -> Option<String> {
    match &expr.node {
        ExpressionType::Identifier { name } => Some(name.to_owned()),
        ExpressionType::Attribute { name, value } => {
            Some(format!("{}.{}", get_absolute_identifier(value)?, name))
        }
        _ => None, //panic!("get_absolute_identifier cannot handle expression type: {}", expr.name())
    }
}

#[derive(Debug)]
pub struct ImportVisitor {
    pub imports: Vec<String>,
}

impl ImportVisitor {
    pub fn new() -> Self {
        ImportVisitor {
            imports: Vec::new(),
        }
    }
}

impl AstVisitor for ImportVisitor {
    fn visit_import(&mut self, names: &Vec<ImportSymbol>) {
        for name in names {
            println!("Visited import: {}", name.to_owned().symbol);
        }
    }

    fn visit_import_from(
        &mut self,
        _level: &usize,
        module: &Option<String>,
        _names: &Vec<ImportSymbol>,
    ) {
        match module {
            Some(import) => {
                let m = import;
                self.imports.push(m.clone());
                if m.contains(".") {
                    let m: Vec<&str> = m.split(".").collect();
                    self.imports.push(m[0].to_owned());
                }
            }
            None => {
                println!("ImportFrom was None. Should not happen.");
            }
        }
    }

    fn visit_call(
        &mut self,
        function: &Box<Located<ExpressionType>>,
        args: &Vec<Located<ExpressionType>>,
        keywords: &Vec<Keyword>,
    ) {
        if let Some(str) = get_absolute_identifier(function) {
            println!("visit call: {}", str);
        }

        self.walk_expression(function);

        args.iter().for_each(|arg| self.walk_expression(arg));

        keywords
            .iter()
            .for_each(|kw| self.walk_expression(&kw.value));
    }
}

#[derive(Debug)]
pub struct SourceFile<'a> {
    pub source_path: &'a PathBuf,
    pub source: String,

    pub function_calls: HashMap<String, Vec<String>>,
    pub constants: Vec<String>,
}

impl<'a> SourceFile<'a> {
    pub fn load(path: &PathBuf) -> io::Result<SourceFile> {
        let source = match utils::load_from_file(path) {
            Ok(source_code) => source_code,
            Err(err) => return Err(err),
        };

        let map: HashMap<String, Vec<String>> = HashMap::new();
        let sf = SourceFile {
            source_path: path,
            source: source,
            function_calls: map,
            constants: vec![],
        };

        let iv: ImportVisitor = sf.visit()?;

        Ok(sf)
    }

    pub fn visit(&self) -> io::Result<ImportVisitor> {
        let mut iv = ImportVisitor::new();

        AstWalker::visit(&mut iv, &self.get_statements()?);

        Ok(iv)
    }

    pub fn display_list(&self, list: &Vec<String>) -> String {
        let indented = utils::indent(list, String::from("\t"));
        indented.join("\n")
    }

    pub fn display_functions(&self) -> String {
        let funcs: Vec<String> = self
            .function_calls
            .iter()
            .map(|item| format!("{}({})", item.0, item.1.join(", ")))
            .collect();
        funcs.join("\n")
    }

    fn parse_file(&self) -> io::Result<Program> {
        match parser::parse_program(self.source.as_str()) {
            Ok(program) => Ok(program),
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "parse error",
            )),
        }
    }

    fn get_statements(&self) -> io::Result<Vec<Statement>> {
        match self.parse_file() {
            Ok(program) => Ok(program.statements),
            Err(err) => Err(err),
        }
    }

    fn find_imports(&mut self) -> io::Result<()> {
        for statement in self.get_statements()? {
            match statement.node {
                StatementType::Import { names } => {
                    names.iter().for_each(|im| {
                        let m = &im.symbol;
                        self.imports.push(m.clone());
                        if m.contains(".") {
                            let m: Vec<&str> = m.split(".").collect();
                            self.imports.push(m[0].to_owned());
                        }
                    });
                }
                StatementType::ImportFrom { module, names, .. } => {}
                _ => {
                    println!("info: statement");
                }
            }
        }
        Ok(())
    }

    fn do_binop(&self, a: String, b: String, op: &Operator) -> Option<String> {
        match op {
            Operator::Add => Some(format!("{}{}", a.to_owned(), b.to_owned())),
            _ => None, //format!("{} binop {}", a, b)
        }
    }

    fn resolve_string_group(&self, value: &StringGroup) -> Option<String> {
        match value {
            StringGroup::Constant { value } => Some(value.to_owned()),
            _ => None, //String::from("unsupported by resolve_string_group") }
        }
    }

    fn resolve_binop(&self, bin_expr: &Located<ExpressionType>) -> Option<String> {
        match &bin_expr.node {
            ExpressionType::Binop { a, b, op } => match &a.deref().node {
                ExpressionType::String { value } => {
                    let a_str = self.resolve_string_group(&value)?;

                    match &b.deref().node {
                        ExpressionType::String { value } => {
                            let b_str = self.resolve_string_group(&value)?;

                            self.do_binop(a_str, b_str, op)
                        }
                        _ => None,
                    }
                }
                _ => None,
            },
            _ => panic!(
                "resolve_binop only expects bin_op expressions: {}",
                bin_expr.name()
            ),
        }
    }

    fn format_empty_arg(&self, opt: &Option<String>) -> String {
        match opt {
            Some(value) => value.clone(),
            None => String::from("*"),
        }
    }

    fn resolve_args(&mut self, args: &Vec<Located<ExpressionType>>) -> Vec<String> {
        let results: Vec<Option<String>> = args
            .iter()
            .map(|arg| match &arg.node {
                ExpressionType::Call { .. } => self.resolve_call(arg),
                ExpressionType::Binop { .. } => self.resolve_binop(arg),
                ExpressionType::String { value } => self.resolve_string_group(&value),
                _ => None,
            })
            .collect();

        results
            .iter()
            .map(|s| self.format_empty_arg(s))
            .collect::<Vec<String>>()
    }

    fn resolve_call(&mut self, call_expr: &Located<ExpressionType>) -> Option<String> {
        match &call_expr.node {
            ExpressionType::Call {
                function,
                args,
                keywords,
            } => {
                let fname = get_absolute_identifier(function.as_ref())?;
                let fargs = self.resolve_args(&args);
                let f = format!("{}({})", fname, fargs.join(", "));
                self.function_calls.insert(fname, fargs);
                return Some(f);
            }
            _ => panic!(
                "resolve_call only expects call expressions: {}",
                call_expr.name()
            ),
        }
    }

    fn find_functions(&mut self) -> io::Result<()> {
        for statement in self.get_statements()? {
            match statement.node {
                StatementType::Expression { expression } => match expression.node {
                    ExpressionType::Attribute { value: _, name } => {
                        println!("attr name: {}", name);
                    }
                    ExpressionType::Identifier { name } => {
                        println!("attr name: {}", name);
                    }
                    ExpressionType::Call { .. } => {
                        self.resolve_call(&expression);
                    }
                    _ => println!("unhandled expr name: {}", &expression.name()),
                },
                _ => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::sourcefile::*;
    use std::path::PathBuf;
    use std::str::FromStr;

    #[test]
    fn test_sourcefile_import() {
        let test = PathBuf::from_str("../ast_experiment/tests/test-5.py").unwrap();
        let sf = SourceFile::load(&test).unwrap();

        let import = sf.imports.iter().next().unwrap().to_owned();
        assert!(import == String::from("urllib.request"));
    }

    #[test]
    fn test_sourcefile_functions() {
        let test = PathBuf::from_str("../ast_experiment/tests/test-5.py").unwrap();
        let sf = SourceFile::load(&test).unwrap();

        let functions_test: Vec<String> = vec!["print", "urllib.request.urlopen"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        for test in functions_test {
            assert!(sf.function_calls.contains_key(&test));
        }
    }

    #[test]
    fn test_sourcefile_functions_expanded() {
        let test = PathBuf::from_str("../ast_experiment/tests/test-7.py").unwrap();
        let sf = SourceFile::load(&test).unwrap();

        let s = String::from("importlib");
        assert!(sf.imports.contains(&s));

        let s = String::from("base64");
        assert!(sf.imports.contains(&s));

        let s = String::from("b64decode");
        assert!(sf.function_calls.contains_key(&s));
    }
}
