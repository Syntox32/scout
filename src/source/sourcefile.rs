use regex::Regex;
use rustpython_parser::ast::{Program, Statement};
use rustpython_parser::error::ParseError;
use rustpython_parser::parser;
use std::io::{self, Error};
use std::path::{Path, PathBuf};

use crate::ast::{AstVisitor, AstWalker};
use crate::utils;
use crate::visitors::{CallVisitor, ImportVisitor};

pub struct ParseErrorFixer {
    attempts: i32,
    current_attempts: i32,
    regex_py2except: Regex,
}

impl ParseErrorFixer {
    pub fn new(attempts: i32) -> Self {
        Self {
            attempts,
            current_attempts: 0,
            regex_py2except: Regex::new(r"except\s[\w]+[\s]*[,][\s]*[\w+]:").unwrap(),
            // tried_except_fix: false,
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
        for (idx, line_content) in lines.iter_mut().enumerate() {
            if idx == line {
                if self.regex_py2except.is_match(line_content) {
                    *line_content = line_content.replace(",", " as ");
                } else {
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
    pub source: String,
    loc: usize,

    pub constants: Vec<String>,
    pub import_visitor: ImportVisitor,
    pub function_visitor: CallVisitor,
}

impl SourceFile {
    fn try_parse(
        fixer: &mut ParseErrorFixer,
        source: &str,
        init_err: ParseError,
    ) -> Result<Program, ParseError> {
        if fixer.attempts_left() {
            warn!("initial error: {}", init_err);
            let source = fixer.attempt_fix(&init_err, source);
            match parser::parse_program(&source) {
                Ok(program) => Ok(program),
                Err(err) => SourceFile::try_parse(fixer, &source, err),
            }
        } else {
            warn!("last error: {}", init_err);
            Err(init_err)
        }
    }

    fn parse_file(source: &str) -> Result<Program, ParseError> {
        let result = parser::parse_program(source);

        match result {
            Ok(program) => Ok(program),
            Err(err) => {
                let mut error_fixer = ParseErrorFixer::new(3);
                Ok(SourceFile::try_parse(&mut error_fixer, source, err)?)
            }
        }
    }

    fn get_statements(source: &str) -> Result<Vec<Statement>, ParseError> {
        Ok(SourceFile::parse_file(source)?.statements)
    }

    fn visit<T>(statements: &[Statement], mut visitor: T) -> T
    where
        T: AstVisitor,
    {
        AstWalker::visit(&mut visitor, statements);
        visitor
    }

    pub fn load(path: &Path) -> io::Result<SourceFile> {
        let source = utils::load_from_file(&path)?;
        let statements = match SourceFile::get_statements(&source) {
            Ok(statements) => statements,
            Err(err) => {
                let e = Error::new(std::io::ErrorKind::Other, format!("{}", err.error));
                return Err(e);
            }
        };
        let loc = source.lines().count().to_owned();

        let sf = SourceFile {
            source_path: path.to_path_buf(),
            source,
            loc,
            constants: vec![],
            import_visitor: SourceFile::visit(&statements, ImportVisitor::new()),
            function_visitor: SourceFile::visit(&statements, CallVisitor::new()),
        };

        Ok(sf)
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
}

#[cfg(test)]
mod tests {
    use crate::source::SourceFile;
    use std::path::PathBuf;
    use std::str::FromStr;

    #[test]
    fn test_sourcefile_import() {
        let test = PathBuf::from_str("../ast_experiment/tests/test-5.py").unwrap();
        let sf = SourceFile::load(&test).unwrap();

        assert!(sf.import_visitor.has_import("urllib.request"));
    }

    #[test]
    fn test_sourcefile_function_visitor() {
        let test = PathBuf::from_str("../ast_experiment/tests/test-5.py").unwrap();
        let sf = SourceFile::load(&test).unwrap();

        let function_visitor_test: Vec<String> = vec!["print", "urllib.request.urlopen"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        for test in function_visitor_test {
            assert!(sf.function_visitor.has_function(&test));
        }
    }

    #[test]
    fn test_sourcefile_import_visitor_expanded() {
        let test = PathBuf::from_str("../ast_experiment/tests/test-7.py").unwrap();
        let sf = SourceFile::load(&test).unwrap();

        assert!(sf.import_visitor.has_import("importlib"));
        assert!(sf.import_visitor.has_import("base64"));
    }

    #[test]
    fn test_sourcefile_function_visitor_expanded() {
        let test = PathBuf::from_str("../ast_experiment/tests/test-7.py").unwrap();
        let sf = SourceFile::load(&test).unwrap();

        assert!(sf.function_visitor.has_function("b64decode"));
    }
}
