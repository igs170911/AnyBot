use fluent_syntax::ast;

pub fn collect_names<S: NameSource>(pattern: &ast::Pattern<S>, out: &mut Vec<String>) {
    for element in &pattern.elements {
        if let ast::PatternElement::Placeable { expression } = element {
            collect_expression_names(expression, out);
        }
    }
}

fn collect_expression_names<S: NameSource>(expression: &ast::Expression<S>, out: &mut Vec<String>) {
    match expression {
        ast::Expression::Select { selector, variants } => {
            collect_inline_names(selector, out);
            for variant in variants {
                collect_names(&variant.value, out);
            }
        }
        ast::Expression::Inline(inline) => collect_inline_names(inline, out),
    }
}

fn collect_inline_names<S: NameSource>(inline: &ast::InlineExpression<S>, out: &mut Vec<String>) {
    match inline {
        ast::InlineExpression::VariableReference { id }
        | ast::InlineExpression::MessageReference { id, .. }
        | ast::InlineExpression::TermReference { id, .. } => out.push(id.identifier_name()),
        ast::InlineExpression::FunctionReference { arguments, .. } => {
            for named in &arguments.named {
                out.push(named.name.identifier_name());
            }
        }
        ast::InlineExpression::Placeable { expression } => {
            collect_expression_names(expression, out);
        }
        ast::InlineExpression::StringLiteral { .. }
        | ast::InlineExpression::NumberLiteral { .. } => {}
    }
}

pub trait NameSource {
    fn identifier_name(&self) -> String;
}

impl NameSource for String {
    fn identifier_name(&self) -> String {
        self.clone()
    }
}

impl NameSource for &'_ str {
    fn identifier_name(&self) -> String {
        (*self).to_owned()
    }
}

impl<S: NameSource> NameSource for ast::Identifier<S> {
    fn identifier_name(&self) -> String {
        self.name.identifier_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_syntax::parser;

    #[test]
    fn test_names_collects_variables_from_single_pattern() {
        let resource = parser::parse("a = b { $one } { $two }").unwrap();
        let Some(ast::Entry::Message(msg)) = resource.body.first() else {
            panic!("expected message entry");
        };
        let mut names = Vec::new();
        collect_names(msg.value.as_ref().unwrap(), &mut names);
        names.sort();
        names.dedup();
        assert_eq!(names, vec!["one".to_owned(), "two".to_owned()]);
    }

    #[test]
    fn test_names_collects_selector_and_variant_variables() {
        let resource =
            parser::parse("a = { $count ->\n    [one] one { $extra }\n   *[other] many\n}")
                .unwrap();
        let Some(ast::Entry::Message(msg)) = resource.body.first() else {
            panic!("expected message entry");
        };
        let mut names = Vec::new();
        collect_names(msg.value.as_ref().unwrap(), &mut names);
        names.sort();
        names.dedup();
        assert_eq!(names, vec!["count".to_owned(), "extra".to_owned()]);
    }
}
