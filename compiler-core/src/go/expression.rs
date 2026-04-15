use crate::{
    ast::TypedModule,
    docvec,
    line_numbers::LineNumbers,
    pretty::{Document, Documentable, line},
};

use super::go_package_name;

#[derive(Debug)]
pub(crate) struct Generator<'a> {
    #[allow(dead_code)]
    module: &'a TypedModule,
    #[allow(dead_code)]
    line_numbers: &'a LineNumbers,
    package_name: &'a str,
}

impl<'a> Generator<'a> {
    pub fn new(
        module: &'a TypedModule,
        line_numbers: &'a LineNumbers,
        package_name: &'a str,
    ) -> Self {
        Self {
            module,
            line_numbers,
            package_name,
        }
    }

    pub fn compile(&mut self) -> Document<'a> {
        let package = go_package_name(self.package_name);
        docvec!["package ", Document::eco_string(package.into()), line()]
    }
}
