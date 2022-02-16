use crate::sourcefile;

#[allow(unused)]
pub struct Package<'a> {
    pub name: String,
    pub sources: Vec<&'a sourcefile::SourceFile<'a>>,
}
