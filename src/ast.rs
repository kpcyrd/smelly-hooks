use crate::command;
use crate::errors::*;
use crate::redirect;
use crate::Context;
use yash_syntax::syntax::{self, CompoundCommand};

pub fn parse(script: &str) -> Result<syntax::List> {
    script
        .parse()
        .map_err(|err| anyhow!("Failed to parse shell script: {err:#}"))
}

fn validate_compound_command(
    ctx: &mut Context,
    compound: &CompoundCommand,
    function_stack: &[String],
) -> Result<()> {
    ctx.inside_compound += 1;
    match compound {
        CompoundCommand::Grouping(list) => {
            validate_ast(ctx, list, function_stack)?;
        }
        CompoundCommand::Subshell { body, .. } => {
            validate_ast(ctx, body, function_stack)?;
        }
        CompoundCommand::For { body, .. } => {
            validate_ast(ctx, body, function_stack)?;
        }
        CompoundCommand::While {
            condition, body, ..
        } => {
            validate_ast(ctx, condition, function_stack)?;
            validate_ast(ctx, body, function_stack)?;
        }
        CompoundCommand::Until {
            condition, body, ..
        } => {
            validate_ast(ctx, condition, function_stack)?;
            validate_ast(ctx, body, function_stack)?;
        }
        CompoundCommand::If {
            condition,
            body,
            elifs,
            r#else,
        } => {
            info!("Entering if-expression processor");
            validate_ast(ctx, condition, function_stack)?;
            validate_ast(ctx, body, function_stack)?;
            for elif in elifs {
                validate_ast(ctx, &elif.condition, function_stack)?;
                validate_ast(ctx, &elif.body, function_stack)?;
            }
            if let Some(or_else) = r#else {
                validate_ast(ctx, or_else, function_stack)?;
            }
            info!("Exiting if-expression processor");
        }
        CompoundCommand::Case { items, .. } => {
            for item in items {
                validate_ast(ctx, &item.body, function_stack)?;
            }
        }
    }
    ctx.inside_compound -= 1;

    Ok(())
}

pub fn validate_ast(
    ctx: &mut Context,
    script: &syntax::List,
    function_stack: &[String],
) -> Result<()> {
    for item in &script.0 {
        for cmd in &item.and_or.first.commands {
            match cmd.as_ref() {
                syntax::Command::Function(fun) => {
                    let name = fun.name.to_string();
                    info!("Discovered function: {name:?}");

                    if !function_stack.is_empty() {
                        ctx.finding(format!("Function {name:?} is defined nested in another function: {function_stack:?}"));
                    } else if ctx.is_inside_compound() {
                        ctx.finding(format!("Function {name:?} is defined nested in a compound (possibly declared conditionally)"));
                    } else {
                        ctx.declared_functions.insert(name.clone());
                    }

                    let mut function_stack = function_stack.to_owned();
                    function_stack.push(name);
                    validate_compound_command(ctx, &fun.body.command, &function_stack)?;
                }
                syntax::Command::Simple(simple) => {
                    command::validate_simple_command(ctx, simple, function_stack)?;
                }
                syntax::Command::Compound(compound) => {
                    validate_compound_command(ctx, &compound.command, function_stack)?;
                    for redir in &*compound.redirs {
                        redirect::validate_redir(ctx, redir)?;
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::validate;

    #[test]
    fn call_declared_function() {
        let script = r#"
        foo() { :; }

        post_install() {
            foo
        }
        "#;
        let findings = validate(script).unwrap();
        assert_eq!(findings, Vec::<String>::new());
    }

    #[test]
    fn nested_functions() {
        let script = r#"
        dummy() { foo() { id; }; }

        post_install() {
            foo
        }
        "#;
        let findings = validate(script).unwrap();
        assert_eq!(
            findings,
            vec![
                "Function \"foo\" is defined nested in another function: [\"dummy\"]",
                "Running unrecognized command: \"id\"",
                "Running unrecognized command: \"foo\""
            ]
        );
    }

    #[test]
    fn conditional_functions() {
        let script = r#"
        case abc in
            def)
                foo() { id; }
                ;;
        esac
        post_install() {
            foo
        }
        "#;
        let findings = validate(script).unwrap();
        assert_eq!(
            findings,
            vec![
                "Function \"foo\" is defined nested in a compound (possibly declared conditionally)",
                "Running unrecognized command: \"id\"",
                "Running unrecognized command: \"foo\"",
            ]
        );
    }

    #[test]
    fn declare_undeclare_function() {
        let script = r#"
        foo() { :; }

        post_install() {
            unset foo
            foo
        }
        "#;
        let findings = validate(script).unwrap();
        assert_eq!(
            findings,
            vec!["Running unrecognized command: \"unset foo\"",]
        );
    }
}
