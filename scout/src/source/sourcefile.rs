use ast_walker::{AstVisitor, AstWalker};
use regex::Regex;
use rustpython_parser::ast::{Program, Suite};
use rustpython_parser::error::ParseError;
use rustpython_parser::parser;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::visitors::{CallEntry, CallVisitor, ImportEntry, ImportVisitor, VariableVisitor};
use crate::Result;

pub struct ParseErrorFixer {
    attempts: i32,
    current_attempts: i32,
    regex_py2except: Regex,
    regex_leading_zero: Regex,
    tried_common_fixes: bool,
}

impl ParseErrorFixer {
    pub fn new(attempts: i32) -> Self {
        Self {
            attempts,
            current_attempts: 0,
            regex_py2except: Regex::new(r"except\s[\w]+[\s]*[,][\s]*[\w+]:").unwrap(),
            regex_leading_zero: Regex::new(r"[0]+\d").unwrap(),
            tried_common_fixes: false,
        }
    }

    pub fn attempts_left(&self) -> bool {
        self.current_attempts < self.attempts
    }

    pub fn attempt_fix(&mut self, err: &ParseError, source: &str) -> String {
        self.current_attempts += 1;
        let line = err.location.row() - 1;

        let mut lines = source
            .lines()
            .map(|s| s.to_owned())
            .collect::<Vec<String>>();

        if !self.tried_common_fixes {
            for line in lines.iter_mut() {
                if let Some(captures) = self.regex_leading_zero.captures(line) {
                    if let Some(m) = captures.get(0) {
                        debug!("Match: {}", &m.as_str());
                        let new = &m.as_str().replace("0", "");
                        *line = line.replace(&m.as_str(), new);
                    }
                }
                if self.regex_py2except.is_match(line) {
                    *line = line.replace(",", " as ");
                }
            }
            self.tried_common_fixes = true;
        } else {
            // if we have tried common fixes we just remove some lines to see if that works
            for (idx, line_content) in lines.iter_mut().enumerate() {
                if idx == line {
                    *line_content = String::from("");
                }
            }
        }

        lines.join("\n")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SourceFile {
    pub source_path: PathBuf,
    loc: usize,
    source: String,

    pub constants: Vec<String>,
    import_visitor: ImportVisitor,
    call_visitor: CallVisitor,
    pub variable_visitor: VariableVisitor,
}

impl SourceFile {
    fn try_parse(
        fixer: &mut ParseErrorFixer,
        source: &str,
        init_err: ParseError,
    ) -> Result<Box<Program>> {
        if fixer.attempts_left() {
            warn!("initial error: {}", init_err);
            let source = fixer.attempt_fix(&init_err, source);
            match parser::parse_program(&source) {
                Ok(program) => Ok(Box::new(program)),
                Err(err) => SourceFile::try_parse(fixer, &source, err),
            }
        } else {
            warn!("last error: {}", init_err);
            Err(init_err.into())
        }
    }

    fn parse_file(source: &str) -> Result<Box<Program>> {
        let result = parser::parse_program(source);

        match result {
            Ok(program) => Ok(Box::new(program)),
            Err(err) => {
                let mut error_fixer = ParseErrorFixer::new(3);
                Ok(SourceFile::try_parse(&mut error_fixer, source, err)?)
            }
        }
    }

    fn get_statements(source: &str) -> Result<Box<Suite>> {
        Ok(Box::new(SourceFile::parse_file(source)?.statements))
    }

    fn visit<T>(statements: &Suite, mut visitor: T) -> T
    where
        T: AstVisitor,
    {
        AstWalker::visit(&mut visitor, statements);
        visitor
    }

    pub fn load(path: &PathBuf, source: String) -> Result<SourceFile> {
        let statements = match SourceFile::get_statements(&source) {
            Ok(statements) => statements,
            Err(err) => {
                return Err(
                    format!("Failed to get statements from file: {}", err.to_string()).into(),
                )
            }
        };

        // println!("Statements: {:#?}", &statements);

        let loc = source.lines().count().to_owned();
        let variable_visitor = SourceFile::visit(&statements, VariableVisitor::new());
        let mut import_visitor = SourceFile::visit(&statements, ImportVisitor::new());
        let mut function_visitor = SourceFile::visit(&statements, CallVisitor::new());
        // debug!("Variable visitor?: {:#?}", variable_visitor.get_variables());

        function_visitor.resolve_imports(import_visitor.get_aliases());
        function_visitor.resolve_variables(variable_visitor.get_variables());

        import_visitor.resolve_dynamic_imports(
            function_visitor.get_entries(),
            variable_visitor.get_variables(),
        );

        let sf = SourceFile {
            source_path: path.to_owned(),
            loc,
            source,
            constants: vec![],
            import_visitor,
            call_visitor: function_visitor,
            variable_visitor,
        };

        Ok(sf)
    }

    pub fn get_source(&self) -> &String {
        &self.source
    }

    pub fn get_loc(&self) -> usize {
        self.loc
    }

    pub fn get_path(&self) -> &str {
        self.source_path
            .to_str()
            .unwrap_or("<error getting filename>")
    }

    pub fn display_functions(&self) -> String {
        self.call_visitor
            .get_entries()
            .iter()
            .map(|entry| entry.full_identifier.to_string())
            .collect::<Vec<String>>()
            .join(", ")
    }

    pub fn display_imports(&self) -> String {
        self.import_visitor
            .get_counts()
            .iter()
            .map(|(ident, _)| ident.to_string())
            .collect::<Vec<String>>()
            .join(", ")
    }
}

///
/// ImportVisitor functions for TFIDF
///
impl SourceFile {
    pub fn get_imports(&self) -> &HashSet<ImportEntry> {
        self.import_visitor.get_imports()
    }

    pub fn get_import_counts(&self) -> &HashMap<String, usize> {
        self.import_visitor.get_counts()
    }

    pub fn has_import(&self, import: &str) -> bool {
        self.import_visitor.has_import(import)
    }

    pub fn get_import_count(&self, import: &str) -> Option<usize> {
        self.import_visitor.get_count(import)
    }

    pub fn get_import_tfidf(&self, import: &str) -> Option<&f64> {
        self.import_visitor.get_tfidf(import)
    }

    pub fn set_import_tfidf(&mut self, im: &str, tfidf: f64) {
        self.import_visitor.set_tfidf(im, tfidf);
    }

    pub fn import_term_frequency_table(&self) -> HashMap<String, f64> {
        let imports = self.import_visitor.get_counts();
        let total_imports = imports.len() as f64;

        imports
            .iter()
            .map(|(import, im_count)| (import.to_owned(), (*im_count as f64) / total_imports))
            .collect()
    }
}

///
/// CallVisitor functions for TFIDF
///
impl SourceFile {
    pub fn has_function(&self, function: &str) -> bool {
        self.call_visitor.has_function(function)
    }

    pub fn get_entries(&self) -> &Vec<CallEntry> {
        &self.call_visitor.get_entries()
    }

    pub fn get_call_counts(&self) -> &HashMap<String, usize> {
        self.call_visitor.get_counts()
    }

    pub fn get_call_tfidf(&self, call: &str) -> Option<&f64> {
        self.call_visitor.get_tfidf(call)
    }

    pub fn set_call_tfidf(&mut self, call: &str, tfidf: f64) {
        self.call_visitor.set_tfidf(call, tfidf);
    }

    pub fn calc_term_frequency_table(&self) -> HashMap<String, f64> {
        let call_counts = self.call_visitor.get_counts();
        let total_num_calls = call_counts.len() as f64;

        call_counts
            .iter()
            .map(|(call, call_count)| (call.to_owned(), (*call_count as f64) / total_num_calls))
            .collect()
    }
}
