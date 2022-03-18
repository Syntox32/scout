use ast_walker::{AstVisitor, AstWalker};
use regex::Regex;
use rustpython_parser::ast::{Program, Suite};
use rustpython_parser::error::ParseError;
use rustpython_parser::parser;
use std::collections::HashSet;
use std::io::{self, Error};
use std::path::{Path, PathBuf};

use crate::utils;
use crate::visitors::{CallVisitor, ImportEntry, ImportVisitor};

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

#[derive(Debug)]
pub struct SourceFile {
    pub source_path: PathBuf,
    loc: usize,

    pub constants: Vec<String>,
    import_visitor: ImportVisitor,
    pub function_visitor: CallVisitor,
}

impl SourceFile {
    fn try_parse(
        fixer: &mut ParseErrorFixer,
        source: &str,
        init_err: ParseError,
    ) -> Result<Box<Program>, ParseError> {
        if fixer.attempts_left() {
            warn!("initial error: {}", init_err);
            let source = fixer.attempt_fix(&init_err, source);
            match parser::parse_program(&source) {
                Ok(program) => Ok(Box::new(program)),
                Err(err) => SourceFile::try_parse(fixer, &source, err),
            }
        } else {
            warn!("last error: {}", init_err);
            Err(init_err)
        }
    }

    fn parse_file(source: &str) -> Result<Box<Program>, ParseError> {
        let result = parser::parse_program(source);

        match result {
            Ok(program) => Ok(Box::new(program)),
            Err(err) => {
                let mut error_fixer = ParseErrorFixer::new(3);
                Ok(SourceFile::try_parse(&mut error_fixer, source, err)?)
            }
        }
    }

    fn get_statements(source: &str) -> Result<Box<Suite>, ParseError> {
        Ok(Box::new(SourceFile::parse_file(source)?.statements))
    }

    fn visit<T>(statements: &Suite, mut visitor: T) -> T
    where
        T: AstVisitor,
    {
        AstWalker::visit(&mut visitor, statements);
        visitor
    }

    pub fn load(path: &Path) -> io::Result<SourceFile> {
        let source = utils::load_from_file(&path)?;
        let statements = match SourceFile::get_statements(&source) {
            Ok(statements) => Box::new(statements),
            Err(err) => {
                let e = Error::new(std::io::ErrorKind::Other, format!("{}", err.error));
                return Err(e);
            }
        };
        // let loc = source.lines().count().to_owned();
        let statements: Suite = vec![];
        let loc = 0;

        let mut import_visitor = SourceFile::visit(&statements, ImportVisitor::new());
        let mut function_visitor = SourceFile::visit(&statements, CallVisitor::new());

        // function_visitor.resolve_imports(import_visitor.get_aliases());
        // import_visitor.resolve_dynamic_imports(function_visitor.get_entries());

        let sf = SourceFile {
            source_path: path.to_path_buf(),
            loc,
            constants: vec![],
            import_visitor,
            function_visitor,
        };

        Ok(sf)
    }

    pub fn get_imports(&self) -> &HashSet<ImportEntry> {
        self.import_visitor.get_imports()
    }

    pub fn has_import(&self, import: &str) -> bool {
        self.import_visitor.has_import(import)
    }

    pub fn get_count(&self, import: &str) -> Option<usize> {
        self.import_visitor.get_count(import)
    }

    pub fn get_loc(&self) -> usize {
        self.loc
    }

    pub fn get_path(&self) -> &str {
        self.source_path
            .to_str()
            .unwrap_or("<error getting filename>")
    }

    // pub fn _display_list(&self, list: &Vec<String>) -> String {
    //     let indented = utils::indent(&list, String::from("\t"));
    //     indented.join("\n")
    // }

    pub fn _display_functions(&self) -> String {
        self.function_visitor
            .entries
            .iter()
            .map(|entry| entry.full_identifier.to_string())
            .collect::<Vec<String>>()
            .join(", ")
    }

    pub fn _display_imports(&self) -> String {
        self.import_visitor
            .get_counts()
            .iter()
            .map(|(ident, _)| ident.to_string())
            .collect::<Vec<String>>()
            .join(", ")
    }
}
