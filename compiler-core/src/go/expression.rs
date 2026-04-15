use std::collections::{HashMap, HashSet};

use ecow::EcoString;

use crate::{
    ast::*,
    docvec,
    line_numbers::LineNumbers,
    pretty::{Document, Documentable, join, line, nil},
    type_::{Type, ValueConstructor, ValueConstructorVariant},
};

use super::{go_package_name, is_go_reserved_word};

const INDENT: isize = 2;

#[derive(Debug)]
pub(crate) struct Generator<'a> {
    module: &'a TypedModule,
    line_numbers: &'a LineNumbers,
    package_name: &'a str,
    prelude_used: bool,
    /// Tracks how many times a given Gleam name has been bound in the current
    /// function, so that shadowing produces distinct Go identifiers. Persists
    /// across block scope restores — a rebind *after* a block must still get
    /// a fresh suffix rather than recycling one the block already used.
    local_counts: HashMap<EcoString, usize>,
    /// The Go identifier currently in scope for each Gleam local name.
    /// Saved and restored around block expressions.
    local_names: HashMap<EcoString, EcoString>,
    /// Every Go identifier already handed out in the current function. Used
    /// to skip counter values that would collide with a user-written binding
    /// like `let x_1 = 1` when a later `let x` is shadowed into `x_1`.
    used_go_names: HashSet<EcoString>,
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
            local_counts: HashMap::new(),
            local_names: HashMap::new(),
            used_go_names: HashSet::new(),
        }
    }

    pub fn compile(&mut self) -> Document<'a> {
        let package: EcoString = go_package_name(self.package_name).into();
        let package_decl = docvec!["package ", package.to_doc()];

        let functions = self
            .module
            .definitions
            .functions
            .iter()
            .filter(|f| f.external_go.is_none())
            .map(|f| self.function(f))
            .collect::<Vec<_>>();

        let imports = if self.prelude_used {
            let path: EcoString = format!("gleam/{}/prelude", self.package_name).into();
            docvec![line(), "import prelude \"", path.to_doc(), "\"", line()]
        } else {
            nil()
        };

        if functions.is_empty() {
            docvec![package_decl, line(), imports]
        } else {
            docvec![
                package_decl,
                line(),
                imports,
                line(),
                join(functions, line())
            ]
        }
    }

    fn function(&mut self, f: &'a TypedFunction) -> Document<'a> {
        self.local_counts.clear();
        self.local_names.clear();
        self.used_go_names.clear();

        let name = match &f.name {
            Some((_, n)) => go_identifier(n, f.publicity.is_importable()),
            None => "anonymous".into(),
        };

        let params = f.arguments.iter().map(|arg| {
            let param_name = match arg.names.get_variable_name() {
                Some(n) => self.register_local(n),
                None => "_".into(),
            };
            docvec![param_name.to_doc(), " ", self.go_type(&arg.type_)]
        });

        let params_doc = join(params, ", ".to_doc());
        let return_type = self.go_type(&f.return_type);
        let body = self.function_body(&f.body, &f.return_type);

        docvec![
            "func ",
            name.to_doc(),
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
        let last_idx = body.len().saturating_sub(1);
        let mut statements: Vec<Document<'a>> = body
            .iter()
            .enumerate()
            .map(|(i, stmt)| self.statement(stmt, i == last_idx))
            .collect();
        // Gleam lets `assert` (and other Nil-typed non-expression statements)
        // stand in last position of a `-> Nil` function. Go still wants an
        // explicit return when the signature is non-void.
        if return_type.is_nil() && !matches!(body.last(), Some(Statement::Expression(_))) {
            statements.push("return struct{}{}".to_doc());
        }
        join(statements, line())
    }

    fn statement(&mut self, statement: &'a TypedStatement, is_last: bool) -> Document<'a> {
        match statement {
            Statement::Expression(expr) => {
                let doc = self.expression(expr);
                if is_last {
                    docvec!["return ", doc]
                } else {
                    docvec!["_ = ", doc]
                }
            }
            Statement::Assignment(assignment) => self.assignment(assignment),
            Statement::Assert(assert) => self.assert(assert),
            Statement::Use(_) => unimplemented!("Go codegen: `use` not yet supported"),
        }
    }

    fn assert(&mut self, assert: &'a TypedAssert) -> Document<'a> {
        let condition = self.expression(&assert.value);
        let message_doc = match &assert.message {
            Some(expr) => self.expression(expr),
            None => self.default_message("assertion failed", assert.location.start),
        };
        docvec!["if !(", condition, ") { panic(", message_doc, ") }"]
    }

    fn assignment(&mut self, assignment: &'a TypedAssignment) -> Document<'a> {
        let value = self.expression(&assignment.value);
        match &assignment.pattern {
            Pattern::Variable { name, .. } => {
                let mangled = self.register_local(name);
                // `_ = name` keeps Go's unused-variable rule happy even when
                // Gleam never references the binding.
                docvec![
                    mangled.clone().to_doc(),
                    " := ",
                    value,
                    line(),
                    "_ = ",
                    mangled.to_doc(),
                ]
            }
            Pattern::Discard { .. } => docvec!["_ = ", value],
            _ => unimplemented!("Go codegen: destructuring let patterns not yet supported"),
        }
    }

    fn register_local(&mut self, name: &EcoString) -> EcoString {
        let mangled = loop {
            let counter = self.local_counts.entry(name.clone()).or_insert(0);
            let candidate: EcoString = if *counter == 0 {
                go_local_name(name).into()
            } else {
                format!("{name}_{counter}").into()
            };
            *counter += 1;
            if !self.used_go_names.contains(&candidate) {
                break candidate;
            }
        };
        let _ = self.used_go_names.insert(mangled.clone());
        let _ = self.local_names.insert(name.clone(), mangled.clone());
        mangled
    }

    fn lookup_local(&self, name: &EcoString) -> EcoString {
        self.local_names
            .get(name)
            .cloned()
            .unwrap_or_else(|| go_local_name(name).into())
    }

    fn expression(&mut self, expression: &'a TypedExpr) -> Document<'a> {
        match expression {
            TypedExpr::Int { value, .. } => numeric_literal("int64", value),
            TypedExpr::Float { value, .. } => numeric_literal("float64", value),
            TypedExpr::String { value, .. } => string_literal(value),
            TypedExpr::Var {
                name, constructor, ..
            } => self.var(name, constructor),
            TypedExpr::Call { fun, arguments, .. } => self.call(fun, arguments),
            TypedExpr::Block { statements, .. } => self.block(statements, &expression.type_()),
            TypedExpr::Pipeline {
                first_value,
                assignments,
                finally,
                ..
            } => self.pipeline(first_value, assignments, finally, &expression.type_()),
            TypedExpr::Panic {
                location, message, ..
            } => self.panic_or_todo(
                "panic expression evaluated",
                location.start,
                message.as_deref(),
                &expression.type_(),
            ),
            TypedExpr::Todo {
                location, message, ..
            } => self.panic_or_todo(
                "`todo` expression evaluated",
                location.start,
                message.as_deref(),
                &expression.type_(),
            ),
            TypedExpr::BinOp {
                name, left, right, ..
            } => self.bin_op(*name, left, right),
            TypedExpr::NegateBool { value, .. } => docvec!["!", self.expression(value)],
            TypedExpr::NegateInt { value, .. } => docvec!["-", self.expression(value)],
            _ => unimplemented!("Go codegen: expression kind not yet supported"),
        }
    }

    fn var(&self, name: &EcoString, constructor: &ValueConstructor) -> Document<'a> {
        match &constructor.variant {
            ValueConstructorVariant::Record { .. } => match name.as_str() {
                "True" => "true".to_doc(),
                "False" => "false".to_doc(),
                "Nil" => "struct{}{}".to_doc(),
                _ => unimplemented!("Go codegen: custom-type constructors not yet supported"),
            },
            ValueConstructorVariant::LocalVariable { .. } => self.lookup_local(name).to_doc(),
            ValueConstructorVariant::ModuleFn { module, .. } => {
                self.module_fn_reference(name, module, constructor.publicity.is_importable())
            }
            ValueConstructorVariant::ModuleConstant { .. } => {
                unimplemented!("Go codegen: module constants not yet supported")
            }
        }
    }

    fn module_fn_reference(
        &self,
        name: &EcoString,
        module: &EcoString,
        public: bool,
    ) -> Document<'a> {
        if module == &self.module.name {
            go_identifier(name, public).to_doc()
        } else {
            unimplemented!("Go codegen: cross-module references not yet supported")
        }
    }

    fn call(&mut self, fun: &'a TypedExpr, arguments: &'a [CallArg<TypedExpr>]) -> Document<'a> {
        let fun_doc = self.expression(fun);
        let args = arguments.iter().map(|arg| self.expression(&arg.value));
        docvec![fun_doc, "(", join(args, ", ".to_doc()), ")"]
    }

    fn panic_or_todo(
        &mut self,
        default_prefix: &'static str,
        start: u32,
        message: Option<&'a TypedExpr>,
        type_: &Type,
    ) -> Document<'a> {
        let message_doc = match message {
            Some(expr) => self.expression(expr),
            None => self.default_message(default_prefix, start),
        };
        docvec![
            "(func() ",
            self.go_type(type_),
            " { panic(",
            message_doc,
            ") }())",
        ]
    }

    fn default_message(&self, prefix: &str, start: u32) -> Document<'a> {
        let line_no = self.line_numbers.line_number(start);
        let text: EcoString = format!("{prefix} at {}:{}", self.module.name, line_no).into();
        docvec!["\"", text.to_doc(), "\""]
    }

    fn pipeline(
        &mut self,
        first_value: &'a TypedPipelineAssignment,
        assignments: &'a [(TypedPipelineAssignment, PipelineAssignmentKind)],
        finally: &'a TypedExpr,
        type_: &Type,
    ) -> Document<'a> {
        // Gleam's pipeline `a |> b |> c` is already desugared into a chain of
        // assignments plus a final expression. Emit it as an IIFE so the
        // value slots into an expression context, matching the block
        // encoding; intermediate bindings are registered so later steps can
        // reference them by their synthetic names.
        let saved_names = self.local_names.clone();
        let mut body_docs: Vec<Document<'a>> = Vec::with_capacity(assignments.len() + 2);
        body_docs.push(self.pipeline_assignment(first_value));
        for (assignment, _kind) in assignments {
            body_docs.push(self.pipeline_assignment(assignment));
        }
        let finally_doc = self.expression(finally);
        body_docs.push(docvec!["return ", finally_doc]);
        self.local_names = saved_names;

        docvec![
            "(func() ",
            self.go_type(type_),
            " {",
            docvec![line(), join(body_docs, line())].nest(INDENT),
            line(),
            "}())",
        ]
    }

    fn pipeline_assignment(&mut self, assignment: &'a TypedPipelineAssignment) -> Document<'a> {
        let value = self.expression(&assignment.value);
        let mangled = self.register_local(&assignment.name);
        docvec![
            mangled.clone().to_doc(),
            " := ",
            value,
            line(),
            "_ = ",
            mangled.to_doc(),
        ]
    }

    fn block(&mut self, statements: &'a [TypedStatement], type_: &Type) -> Document<'a> {
        // Gleam blocks are expressions; Go blocks are statements. Wrap in an
        // IIFE so the block's value slots into the surrounding expression
        // context. Statement lifting is the cleaner encoding but requires a
        // statement accumulator threaded through every expression site.
        let saved_names = self.local_names.clone();
        let last_idx = statements.len().saturating_sub(1);
        let body_docs: Vec<_> = statements
            .iter()
            .enumerate()
            .map(|(i, stmt)| self.statement(stmt, i == last_idx))
            .collect();
        self.local_names = saved_names;

        docvec![
            "(func() ",
            self.go_type(type_),
            " {",
            docvec![line(), join(body_docs, line())].nest(INDENT),
            line(),
            "}())",
        ]
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
                docvec!["(", self.expression(left), op, self.expression(right), ")"]
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
            unimplemented!("Go codegen: complex types not yet supported")
        }
    }
}

fn numeric_literal<'a>(type_name: &'static str, value: &str) -> Document<'a> {
    let literal: EcoString = value.into();
    docvec![type_name, "(", literal.to_doc(), ")"]
}

fn string_literal<'a>(value: &str) -> Document<'a> {
    // Gleam stores string literals with escape sequences preserved as source
    // text; Go's string-literal syntax is a superset for those cases so they
    // can be emitted verbatim. Literal newlines in multi-line strings still
    // need re-escaping. Gleam's `\u{NNNN}` escape differs from Go's `\uNNNN`
    // and is not yet rewritten.
    let body: EcoString = if value.contains('\n') {
        value.replace('\n', r"\n").into()
    } else {
        value.into()
    };
    docvec!["\"", body.to_doc(), "\""]
}

fn go_identifier(name: &str, public: bool) -> EcoString {
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
    out.into()
}

fn go_local_name(name: &str) -> EcoString {
    if is_go_reserved_word(name) || is_go_predeclared_identifier(name) {
        format!("{name}_").into()
    } else {
        name.into()
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
