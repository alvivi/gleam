use crate::{ast::TypedModule, line_numbers::LineNumbers};

mod expression;

#[cfg(test)]
mod tests;

pub const GO_VERSION: &str = "1.21";

pub fn module(module: &TypedModule, line_numbers: &LineNumbers, package_name: &str) -> String {
    let mut generator = expression::Generator::new(module, line_numbers, package_name);
    generator.compile().to_pretty_string(80)
}

pub fn go_mod(project_name: &str) -> String {
    format!(
        "module gleam/{project_name}\n\ngo {version}\n",
        version = GO_VERSION,
    )
}

pub fn go_package_name(gleam_module_name: &str) -> String {
    let last = gleam_module_name
        .rsplit('/')
        .next()
        .unwrap_or(gleam_module_name);
    sanitize_package_name(last)
}

fn sanitize_package_name(name: &str) -> String {
    let needs_prefix = name
        .chars()
        .next()
        .is_none_or(|c| c.is_ascii_digit() || !c.is_ascii_alphabetic());
    let mut out = String::with_capacity(name.len() + usize::from(needs_prefix));
    if needs_prefix {
        out.push('p');
    }
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if is_go_reserved_word(&out) {
        out.push('_');
    }
    out
}

/// Go language reserved words (spec section "Keywords"). A sanitized
/// identifier that lands on one of these would fail to compile as a package
/// name, so we suffix it.
fn is_go_reserved_word(word: &str) -> bool {
    matches!(
        word,
        "break"
            | "case"
            | "chan"
            | "const"
            | "continue"
            | "default"
            | "defer"
            | "else"
            | "fallthrough"
            | "for"
            | "func"
            | "go"
            | "goto"
            | "if"
            | "import"
            | "interface"
            | "map"
            | "package"
            | "range"
            | "return"
            | "select"
            | "struct"
            | "switch"
            | "type"
            | "var"
    )
}
