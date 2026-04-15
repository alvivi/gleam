use ecow::EcoString;
use itertools::Itertools;

use crate::{
    ast::*,
    docvec,
    line_numbers::LineNumbers,
    pretty::{Document, Documentable, line},
    type_::{Type, ValueConstructorVariant},
};

use super::{go_package_name, is_go_reserved_word};

const INDENT: isize = 2;

#[derive(Debug)]
pub(crate) struct Generator<'a> {
    module: &'a TypedModule,
    #[allow(dead_code)]
    line_numbers: &'a LineNumbers,
    package_name: &'a str,
    prelude_used: bool,
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
            prelude_used: false,
        }
    }

    pub fn compile(&mut self) -> Document<'a> {
        let package_decl = docvec![
            "package ",
            Document::eco_string(go_package_name(self.package_name).into()),
        ];

        let functions = self
            .module
            .definitions
            .functions
            .iter()
            .filter(|f| f.external_go.is_none())
            .map(|f| self.function(f))
            .collect_vec();

        let imports = if self.prelude_used {
            let path = format!("gleam/{}/prelude", self.package_name);
            docvec![
                line(),
                "import prelude \"",
                Document::eco_string(path.into()),
                "\"",
                line(),
            ]
        } else {
            Document::Vec(vec![])
        };

        if functions.is_empty() {
            docvec![package_decl, line(), imports]
        } else {
            let separated = Itertools::intersperse(functions.into_iter(), line()).collect();
            docvec![
                package_decl,
                line(),
                imports,
                line(),
                Document::Vec(separated)
            ]
        }
    }

    fn function(&mut self, f: &'a TypedFunction) -> Document<'a> {
        let name = match &f.name {
            Some((_, n)) => go_identifier(n, f.publicity.is_importable()),
            None => "anonymous".to_string(),
        };

        let params = f
            .arguments
            .iter()
            .map(|arg| {
                let name = arg
                    .names
                    .get_variable_name()
                    .map(|n| go_local_name(n))
                    .unwrap_or_else(|| "_".to_string());
                docvec![
                    Document::eco_string(name.into()),
                    " ",
                    self.go_type(&arg.type_),
                ]
            })
            .collect_vec();

        let params_doc =
            Document::Vec(Itertools::intersperse(params.into_iter(), ", ".to_doc()).collect());
        let return_type = self.go_type(&f.return_type);

        let body = self.function_body(&f.body, &f.return_type);

        docvec![
            "func ",
            Document::eco_string(name.into()),
            "(",
            params_doc,
            ") ",
            return_type,
            " {",
            docvec![line(), body].nest(INDENT),
            line(),
            "}",
            line(),
        ]
    }

    fn function_body(&mut self, body: &'a [TypedStatement], return_type: &Type) -> Document<'a> {
        let mut statements = Vec::with_capacity(body.len());
        let last_idx = body.len().saturating_sub(1);
        for (i, stmt) in body.iter().enumerate() {
            let is_last = i == last_idx;
            statements.push(self.statement(stmt, is_last, return_type));
        }
        Document::Vec(Itertools::intersperse(statements.into_iter(), line()).collect())
    }

    fn statement(
        &mut self,
        statement: &'a TypedStatement,
        is_last: bool,
        _return_type: &Type,
    ) -> Document<'a> {
        match statement {
            Statement::Expression(expr) => {
                let doc = self.expression(expr);
                if is_last {
                    docvec!["return ", doc]
                } else {
                    docvec!["_ = ", doc]
                }
            }
            Statement::Assignment(_) | Statement::Use(_) | Statement::Assert(_) => {
                unimplemented!("Go codegen: non-expression statements land in a later M2 step")
            }
        }
    }

    fn expression(&mut self, expression: &'a TypedExpr) -> Document<'a> {
        match expression {
            TypedExpr::Int { value, .. } => self.int(value),
            TypedExpr::Float { value, .. } => self.float(value),
            TypedExpr::String { value, .. } => string_literal(value),
            TypedExpr::Var {
                name, constructor, ..
            } => self.var(name, constructor),
            TypedExpr::Call { fun, arguments, .. } => self.call(fun, arguments),
            TypedExpr::BinOp {
                name, left, right, ..
            } => self.bin_op(*name, left, right),
            TypedExpr::NegateBool { value, .. } => {
                docvec!["!", self.expression(value)]
            }
            TypedExpr::NegateInt { value, .. } => {
                docvec!["-", self.expression(value)]
            }
            _ => unimplemented!("Go codegen: expression kind lands in a later M2 step"),
        }
    }

    fn int(&self, value: &str) -> Document<'a> {
        docvec!["int64(", Document::eco_string(value.into()), ")"]
    }

    fn float(&self, value: &str) -> Document<'a> {
        docvec!["float64(", Document::eco_string(value.into()), ")"]
    }

    fn var(&self, name: &EcoString, constructor: &crate::type_::ValueConstructor) -> Document<'a> {
        match &constructor.variant {
            ValueConstructorVariant::Record { .. } => match name.as_str() {
                "True" => "true".to_doc(),
                "False" => "false".to_doc(),
                "Nil" => "struct{}{}".to_doc(),
                _ => unimplemented!("Go codegen: custom-type constructors land in M4"),
            },
            ValueConstructorVariant::LocalVariable { .. } => {
                Document::eco_string(go_local_name(name).into())
            }
            ValueConstructorVariant::ModuleFn { module, .. } => {
                self.module_fn_reference(name, module)
            }
            ValueConstructorVariant::ModuleConstant { .. } => {
                unimplemented!("Go codegen: module constants land in a later M2 step")
            }
        }
    }

    fn module_fn_reference(&self, name: &EcoString, module: &EcoString) -> Document<'a> {
        if module == &self.module.name {
            let ident = go_identifier(name, is_exported_function(name, &self.module));
            Document::eco_string(ident.into())
        } else {
            unimplemented!("Go codegen: cross-module calls land in a later milestone")
        }
    }

    fn call(&mut self, fun: &'a TypedExpr, arguments: &'a [CallArg<TypedExpr>]) -> Document<'a> {
        let fun_doc = self.expression(fun);
        let args = arguments
            .iter()
            .map(|arg| self.expression(&arg.value))
            .collect_vec();
        let args_doc =
            Document::Vec(Itertools::intersperse(args.into_iter(), ", ".to_doc()).collect());
        docvec![fun_doc, "(", args_doc, ")"]
    }

    fn bin_op(&mut self, name: BinOp, left: &'a TypedExpr, right: &'a TypedExpr) -> Document<'a> {
        match name {
            BinOp::DivInt => self.prelude_call("DivInt", left, right),
            BinOp::RemainderInt => self.prelude_call("RemInt", left, right),
            BinOp::DivFloat => self.prelude_call("DivFloat", left, right),
            _ => {
                let op: Document<'a> = match name {
                    BinOp::And => " && ".to_doc(),
                    BinOp::Or => " || ".to_doc(),
                    BinOp::Eq => " == ".to_doc(),
                    BinOp::NotEq => " != ".to_doc(),
                    BinOp::LtInt | BinOp::LtFloat => " < ".to_doc(),
                    BinOp::LtEqInt | BinOp::LtEqFloat => " <= ".to_doc(),
                    BinOp::GtInt | BinOp::GtFloat => " > ".to_doc(),
                    BinOp::GtEqInt | BinOp::GtEqFloat => " >= ".to_doc(),
                    BinOp::AddInt | BinOp::AddFloat => " + ".to_doc(),
                    BinOp::SubInt | BinOp::SubFloat => " - ".to_doc(),
                    BinOp::MultInt | BinOp::MultFloat => " * ".to_doc(),
                    BinOp::Concatenate => " + ".to_doc(),
                    BinOp::DivInt | BinOp::DivFloat | BinOp::RemainderInt => unreachable!(),
                };
                docvec!["(", self.expression(left), op, self.expression(right), ")",]
            }
        }
    }

    fn prelude_call(
        &mut self,
        function: &'static str,
        left: &'a TypedExpr,
        right: &'a TypedExpr,
    ) -> Document<'a> {
        self.prelude_used = true;
        docvec![
            "prelude.",
            function,
            "(",
            self.expression(left),
            ", ",
            self.expression(right),
            ")",
        ]
    }

    fn go_type(&self, type_: &Type) -> Document<'a> {
        if type_.is_int() {
            "int64".to_doc()
        } else if type_.is_float() {
            "float64".to_doc()
        } else if type_.is_bool() {
            "bool".to_doc()
        } else if type_.is_string() {
            "string".to_doc()
        } else if type_.is_nil() {
            "struct{}".to_doc()
        } else {
            unimplemented!("Go codegen: complex type lowering lands in later milestones")
        }
    }
}

fn is_exported_function(name: &EcoString, module: &TypedModule) -> bool {
    module
        .definitions
        .functions
        .iter()
        .find(|f| f.name.as_ref().map(|(_, n)| n == name).unwrap_or(false))
        .map(|f| f.publicity.is_importable())
        .unwrap_or(false)
}

fn string_literal(value: &str) -> Document<'_> {
    // Gleam stores string literals with escape sequences (`\n`, `\t`, `\"`,
    // `\\`) preserved as source text. Go's string-literal syntax is a
    // superset for those cases so they can be emitted verbatim. Literal
    // newlines can appear in multi-line strings and must be re-escaped.
    // `\u{NNNN}` needs rewriting to Go's `\uNNNN`, handled in a later M2 step.
    let body: EcoString = if value.contains('\n') {
        value.replace('\n', r"\n").into()
    } else {
        value.into()
    };
    docvec!["\"", Document::eco_string(body), "\""]
}

fn go_identifier(name: &str, public: bool) -> String {
    let mut out = String::with_capacity(name.len());
    let mut capitalise = public;
    for ch in name.chars() {
        if ch == '_' {
            capitalise = true;
            continue;
        }
        if capitalise {
            out.extend(ch.to_uppercase());
            capitalise = false;
        } else {
            out.push(ch);
        }
    }
    if out.is_empty() {
        out.push('_');
    }
    if is_go_reserved_word(&out) || is_go_predeclared_identifier(&out) {
        out.push('_');
    }
    out
}

fn go_local_name(name: &str) -> String {
    if is_go_reserved_word(name) || is_go_predeclared_identifier(name) {
        format!("{name}_")
    } else {
        name.to_string()
    }
}

fn is_go_predeclared_identifier(word: &str) -> bool {
    matches!(
        word,
        "any"
            | "bool"
            | "byte"
            | "comparable"
            | "complex64"
            | "complex128"
            | "error"
            | "float32"
            | "float64"
            | "int"
            | "int8"
            | "int16"
            | "int32"
            | "int64"
            | "rune"
            | "string"
            | "uint"
            | "uint8"
            | "uint16"
            | "uint32"
            | "uint64"
            | "uintptr"
            | "true"
            | "false"
            | "iota"
            | "nil"
            | "append"
            | "cap"
            | "clear"
            | "close"
            | "complex"
            | "copy"
            | "delete"
            | "imag"
            | "len"
            | "make"
            | "max"
            | "min"
            | "new"
            | "panic"
            | "print"
            | "println"
            | "real"
            | "recover"
    )
}
