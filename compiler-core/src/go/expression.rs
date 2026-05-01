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

        let params = f.arguments.iter().map(|arg| self.parameter(arg));
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
            Statement::Expression(expr) => self.expression_statement(expr, is_last),
            Statement::Assignment(assignment) => self.assignment(assignment),
            Statement::Assert(assert) => self.assert(assert),
            // `use` reaches codegen pre-desugared by the typechecker into the
            // call it expands to (`use a <- f(x); rest` becomes
            // `f(x, fn(a) { rest })`), so it lowers like any other call.
            Statement::Use(use_) => self.expression_statement(&use_.call, is_last),
        }
    }

    fn expression_statement(&mut self, expr: &'a TypedExpr, is_last: bool) -> Document<'a> {
        let doc = self.expression(expr);
        if is_last {
            docvec!["return ", doc]
        } else {
            docvec!["_ = ", doc]
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
                self.bind_local(&mangled, value)
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

    /// Lower a function or closure parameter declaration: registers the
    /// binding (or `_` for a discard) and pairs it with its Go type.
    fn parameter(&mut self, arg: &'a TypedArg) -> Document<'a> {
        let param_name = match arg.names.get_variable_name() {
            Some(n) => self.register_local(n),
            None => "_".into(),
        };
        docvec![param_name.to_doc(), " ", self.go_type(&arg.type_)]
    }

    /// Run `f` in a saved-and-restored `local_names` scope. `local_counts`
    /// and `used_go_names` deliberately stay mutated: a later rebind of the
    /// same Gleam name must still get a fresh suffix and never collide with
    /// one a now-closed scope already claimed in the same function body.
    fn with_scope<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let saved = self.local_names.clone();
        let result = f(self);
        self.local_names = saved;
        result
    }

    /// Emit `mangled := value` followed by `_ = mangled`. The discard line
    /// keeps Go's unused-variable rule happy when the Gleam source never
    /// references the binding.
    fn bind_local(&self, mangled: &EcoString, value: Document<'a>) -> Document<'a> {
        docvec![
            mangled.clone().to_doc(),
            " := ",
            value,
            line(),
            "_ = ",
            mangled.clone().to_doc(),
        ]
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
            TypedExpr::Case {
                subjects, clauses, ..
            } => self.case_expr(subjects, clauses, &expression.type_()),
            TypedExpr::Fn {
                arguments, body, ..
            } => self.closure(arguments, body, &expression.type_()),
            _ => unimplemented!("Go codegen: expression kind not yet supported"),
        }
    }

    fn closure(
        &mut self,
        arguments: &'a [TypedArg],
        body: &'a [TypedStatement],
        fn_type: &Type,
    ) -> Document<'a> {
        let return_type = fn_type
            .return_type()
            .expect("anonymous function expression must have a function type");

        let (params, body_doc) = self.with_scope(|s| {
            let params: Vec<Document<'a>> = arguments
                .iter()
                .map(|arg| s.parameter(arg))
                .collect();
            let body_doc = s.function_body(body, &return_type);
            (params, body_doc)
        });

        docvec![
            "func(",
            join(params, ", ".to_doc()),
            ") ",
            self.go_type(&return_type),
            " {",
            docvec![line(), body_doc].nest(INDENT),
            line(),
            "}",
        ]
    }

    fn case_expr(
        &mut self,
        subjects: &'a [TypedExpr],
        clauses: &'a [TypedClause],
        return_type: &Type,
    ) -> Document<'a> {
        // Each subject must be evaluated exactly once even though it is
        // referenced from every clause's per-position condition. Bind each
        // to a synthetic local; `register_local` mangles against
        // `used_go_names` so concurrent or nested case expressions get
        // distinct slots automatically.
        let body_docs = self.with_scope(|s| {
            let mut body_docs: Vec<Document<'a>> =
                Vec::with_capacity(subjects.len() + clauses.len() + 1);
            let mut subject_names: Vec<EcoString> = Vec::with_capacity(subjects.len());
            for subject in subjects {
                let subject_doc = s.expression(subject);
                let name = s.register_local(&"_case_subject".into());
                body_docs.push(s.bind_local(&name, subject_doc));
                subject_names.push(name);
            }

            let mut catch_all_emitted = false;
            for clause in clauses {
                if !clause.alternative_patterns.is_empty() {
                    unimplemented!("Go codegen: alternative case patterns not yet supported");
                }
                // Pattern bindings (e.g. `x -> ...`) only live for one clause,
                // so the per-clause scope is saved and restored around
                // lowering and body generation.
                let (conditions, bindings, guard_doc, body) = s.with_scope(|s| {
                    let mut conditions: Vec<Document<'a>> = Vec::new();
                    let mut bindings: Vec<Document<'a>> = Vec::new();
                    for (subject_name, pattern) in
                        subject_names.iter().zip(clause.pattern.iter())
                    {
                        let (condition, pattern_bindings) =
                            s.lower_case_pattern(subject_name, pattern);
                        if let Some(condition) = condition {
                            conditions.push(condition);
                        }
                        bindings.extend(pattern_bindings);
                    }
                    // The guard runs with pattern bindings in scope, so it
                    // must be lowered *after* `lower_case_pattern` registers
                    // them and before the clause-local scope is restored.
                    let guard_doc = clause.guard.as_ref().map(|g| s.lower_guard(g));
                    let body = s.expression(&clause.then);
                    (conditions, bindings, guard_doc, body)
                });

                let return_doc = docvec!["return ", body];
                let conditional_return = match &guard_doc {
                    Some(guard) => docvec![
                        "if ",
                        guard.clone(),
                        " {",
                        docvec![line(), return_doc].nest(INDENT),
                        line(),
                        "}",
                    ],
                    None => return_doc,
                };
                let mut clause_body: Vec<Document<'a>> = bindings;
                clause_body.push(conditional_return);
                let clause_body_doc = join(clause_body, line());

                if conditions.is_empty() {
                    // No pattern conditions: bindings + body live at the IIFE's
                    // top level. With a guard the clause may still fail, so
                    // execution can fall through to the next clause.
                    body_docs.push(clause_body_doc);
                    if guard_doc.is_none() {
                        catch_all_emitted = true;
                        break;
                    }
                } else {
                    // `==` binds tighter than `&&` in Go, so the per-position
                    // equality tests do not need parentheses around them.
                    let condition_doc = join(conditions, " && ".to_doc());
                    body_docs.push(docvec![
                        "if ",
                        condition_doc,
                        " {",
                        docvec![line(), clause_body_doc].nest(INDENT),
                        line(),
                        "}",
                    ]);
                }
            }

            // Gleam's exhaustiveness check guarantees every value is matched,
            // but Go's flow analyser cannot see that. A trailing panic on the
            // absent catch-all keeps the generated function well-formed.
            if !catch_all_emitted {
                body_docs.push("panic(\"non-exhaustive case\")".to_doc());
            }
            body_docs
        });

        self.iife(self.go_type(return_type), join(body_docs, line()))
    }

    /// Lower a single clause pattern against the bound subject. Returns the
    /// match condition (`None` means the pattern always matches and so
    /// terminates the if-chain) plus any bindings the pattern introduces
    /// into the clause body.
    fn lower_case_pattern(
        &mut self,
        subject: &EcoString,
        pattern: &'a TypedPattern,
    ) -> (Option<Document<'a>>, Vec<Document<'a>>) {
        match pattern {
            Pattern::Discard { .. } => (None, vec![]),
            Pattern::Variable { name, .. } => {
                let mangled = self.register_local(name);
                let binding = self.bind_local(&mangled, subject.clone().to_doc());
                (None, vec![binding])
            }
            Pattern::Int { value, .. } => (
                Some(docvec![
                    subject.clone().to_doc(),
                    " == ",
                    numeric_literal("int64", value),
                ]),
                vec![],
            ),
            Pattern::Float { value, .. } => (
                Some(docvec![
                    subject.clone().to_doc(),
                    " == ",
                    numeric_literal("float64", value),
                ]),
                vec![],
            ),
            Pattern::String { value, .. } => (
                Some(docvec![
                    subject.clone().to_doc(),
                    " == ",
                    string_literal(value),
                ]),
                vec![],
            ),
            Pattern::Constructor {
                name, arguments, ..
            } if arguments.is_empty() => {
                let go_value = match name.as_str() {
                    "True" => "true",
                    "False" => "false",
                    _ => unimplemented!(
                        "Go codegen: case on custom-type constructors not yet supported"
                    ),
                };
                (
                    Some(docvec![subject.clone().to_doc(), " == ", go_value]),
                    vec![],
                )
            }
            _ => unimplemented!("Go codegen: case pattern kind not yet supported"),
        }
    }

    fn lower_guard(&mut self, guard: &'a TypedClauseGuard) -> Document<'a> {
        match guard {
            ClauseGuard::Block { value, .. } => self.lower_guard(value),
            ClauseGuard::Not { expression, .. } => {
                docvec!["!", self.lower_guard(expression)]
            }
            ClauseGuard::Var { name, .. } => self.lookup_local(name).to_doc(),
            ClauseGuard::Constant(constant) => self.lower_guard_constant(constant),
            ClauseGuard::BinaryOperator {
                operator,
                left,
                right,
                ..
            } => {
                let left_doc = self.lower_guard(left);
                let right_doc = self.lower_guard(right);
                self.lower_guard_binop(*operator, left_doc, right_doc)
            }
            _ => unimplemented!("Go codegen: guard expression kind not yet supported"),
        }
    }

    fn lower_guard_constant(&mut self, constant: &'a TypedConstant) -> Document<'a> {
        match constant {
            Constant::Int { value, .. } => numeric_literal("int64", value),
            Constant::Float { value, .. } => numeric_literal("float64", value),
            Constant::String { value, .. } => string_literal(value),
            Constant::Var { name, .. } => match name.as_str() {
                "True" => "true".to_doc(),
                "False" => "false".to_doc(),
                _ => unimplemented!("Go codegen: guard constant references not yet supported"),
            },
            _ => unimplemented!("Go codegen: guard constant kind not yet supported"),
        }
    }

    fn lower_guard_binop(
        &mut self,
        name: BinOp,
        left: Document<'a>,
        right: Document<'a>,
    ) -> Document<'a> {
        if let Some(prelude_fn) = binop_prelude_function(name) {
            self.prelude_used = true;
            return docvec!["prelude.", prelude_fn, "(", left, ", ", right, ")"];
        }
        let op = binop_operator(name).expect("non-prelude BinOp must have a Go operator");
        docvec!["(", left, op, right, ")"]
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
        // Single-line IIFE on purpose: the body is a one-shot `panic` call,
        // so the multi-line `iife` helper would just add noise.
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
        let body_docs = self.with_scope(|s| {
            let mut body_docs: Vec<Document<'a>> = Vec::with_capacity(assignments.len() + 2);
            body_docs.push(s.pipeline_assignment(first_value));
            for (assignment, _kind) in assignments {
                body_docs.push(s.pipeline_assignment(assignment));
            }
            let finally_doc = s.expression(finally);
            body_docs.push(docvec!["return ", finally_doc]);
            body_docs
        });
        self.iife(self.go_type(type_), join(body_docs, line()))
    }

    fn pipeline_assignment(&mut self, assignment: &'a TypedPipelineAssignment) -> Document<'a> {
        let value = self.expression(&assignment.value);
        let mangled = self.register_local(&assignment.name);
        self.bind_local(&mangled, value)
    }

    fn block(&mut self, statements: &'a [TypedStatement], type_: &Type) -> Document<'a> {
        // Gleam blocks are expressions; Go blocks are statements. Wrap in an
        // IIFE so the block's value slots into the surrounding expression
        // context. Statement lifting is the cleaner encoding but requires a
        // statement accumulator threaded through every expression site.
        let body_docs = self.with_scope(|s| {
            let last_idx = statements.len().saturating_sub(1);
            statements
                .iter()
                .enumerate()
                .map(|(i, stmt)| s.statement(stmt, i == last_idx))
                .collect::<Vec<_>>()
        });
        self.iife(self.go_type(type_), join(body_docs, line()))
    }

    fn bin_op(&mut self, name: BinOp, left: &'a TypedExpr, right: &'a TypedExpr) -> Document<'a> {
        let left_doc = self.expression(left);
        let right_doc = self.expression(right);
        if let Some(prelude_fn) = binop_prelude_function(name) {
            self.prelude_used = true;
            return docvec!["prelude.", prelude_fn, "(", left_doc, ", ", right_doc, ")"];
        }
        let op = binop_operator(name).expect("non-prelude BinOp must have a Go operator");
        docvec!["(", left_doc, op, right_doc, ")"]
    }

    /// Wrap a multi-statement body in an immediately-invoked Go function
    /// literal returning `return_type`. Lets statement-shaped Gleam constructs
    /// (blocks, pipelines, case) slot into Go expression positions without
    /// lifting a statement accumulator out to the call site.
    fn iife(&self, return_type: Document<'a>, body: Document<'a>) -> Document<'a> {
        docvec![
            "(func() ",
            return_type,
            " {",
            docvec![line(), body].nest(INDENT),
            line(),
            "}())",
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
        } else if let Some((args, return_type)) = type_.fn_types() {
            let args_doc = join(args.iter().map(|a| self.go_type(a)), ", ".to_doc());
            docvec!["func(", args_doc, ") ", self.go_type(&return_type)]
        } else {
            unimplemented!("Go codegen: complex types not yet supported")
        }
    }
}

/// Go infix operator for a Gleam BinOp, or `None` if the operator routes
/// through the prelude instead (see `binop_prelude_function`).
fn binop_operator(name: BinOp) -> Option<&'static str> {
    match name {
        BinOp::And => Some(" && "),
        BinOp::Or => Some(" || "),
        BinOp::Eq => Some(" == "),
        BinOp::NotEq => Some(" != "),
        BinOp::LtInt | BinOp::LtFloat => Some(" < "),
        BinOp::LtEqInt | BinOp::LtEqFloat => Some(" <= "),
        BinOp::GtInt | BinOp::GtFloat => Some(" > "),
        BinOp::GtEqInt | BinOp::GtEqFloat => Some(" >= "),
        BinOp::AddInt | BinOp::AddFloat => Some(" + "),
        BinOp::SubInt | BinOp::SubFloat => Some(" - "),
        BinOp::MultInt | BinOp::MultFloat => Some(" * "),
        BinOp::Concatenate => Some(" + "),
        BinOp::DivInt | BinOp::DivFloat | BinOp::RemainderInt => None,
    }
}

/// Prelude helper name for the Gleam BinOps whose Go semantics differ from
/// the native operator (Gleam division and remainder must return zero on a
/// zero divisor rather than panic the way Go's `/` and `%` do).
fn binop_prelude_function(name: BinOp) -> Option<&'static str> {
    match name {
        BinOp::DivInt => Some("DivInt"),
        BinOp::RemainderInt => Some("RemInt"),
        BinOp::DivFloat => Some("DivFloat"),
        _ => None,
    }
}

fn numeric_literal<'a>(type_name: &'static str, value: &str) -> Document<'a> {
    let literal: EcoString = value.into();
    docvec![type_name, "(", literal.to_doc(), ")"]
}

fn string_literal<'a>(value: &str) -> Document<'a> {
    // The Gleam lexer stores `\n \r \t \f \" \\` as two-char sequences and
    // `\u{hex}` as literal `\u{...}` text; all other bytes are verbatim.
    // Go's interpreted-string form accepts the two-char escapes with the
    // same meaning, but rejects `\u{...}` (it demands `\uNNNN` for the BMP
    // or `\UNNNNNNNN` for astral codepoints), and forbids raw newlines.
    // We walk the value in whole-escape units so a literal backslash
    // (encoded `\\`) isn't mistaken for the start of a `\u` sequence.
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        match c {
            '\n' => out.push_str(r"\n"),
            '\\' => match chars.next() {
                Some('u') => {
                    let _ = chars.next();
                    let mut hex = String::new();
                    for d in chars.by_ref() {
                        if d == '}' {
                            break;
                        }
                        hex.push(d);
                    }
                    let codepoint = u32::from_str_radix(&hex, 16)
                        .expect("unicode escape validated by lexer");
                    if codepoint <= 0xFFFF {
                        out.push_str(&format!("\\u{codepoint:04X}"));
                    } else {
                        out.push_str(&format!("\\U{codepoint:08X}"));
                    }
                }
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            },
            _ => out.push(c),
        }
    }
    out.push('"');
    EcoString::from(out).to_doc()
}

fn go_identifier(name: &str, public: bool) -> EcoString {
    // Preserve trailing underscores literally. Gleam uses a trailing `_` to
    // escape keywords as identifiers (`type_`, `foo_`), so `foo` and `foo_`
    // are distinct Gleam functions and must emit distinct Go names.
    let trailing_underscores = name.chars().rev().take_while(|c| *c == '_').count();
    let body = &name[..name.len() - trailing_underscores];
    let mut out = String::with_capacity(name.len());
    let mut capitalise = public;
    for ch in body.chars() {
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
    for _ in 0..trailing_underscores {
        out.push('_');
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
