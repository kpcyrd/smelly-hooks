use crate::ast;
use crate::errors::*;
use crate::redirect::validate_redir;
use crate::{Context, FindingCondition};
use yash_syntax::syntax::{self, SimpleCommand, TextUnit, WordUnit};

pub const REASONABLE_BINARIES: &[&str] = &[
    "/bin/true",
    "[",
    "cat",
    "chmod",
    "chown",
    "getent",
    "grep",
    "killall",
    "mkdir",
    "pgrep",
    "printf",
    "rm",
    "rmdir",
    "setcap",
    "systemd-sysusers",
    "touch",
    "true",
    "unlink",
    "usermod",
    "uuidgen",
    "vercmp",
    "yes",
];
pub const REASONABLE_BUILTINS: &[&str] = &[
    ":", "break", "cd", "continue", "echo", "local", "popd", "pushd", "return", "shift",
];

fn word_contains_variables(word: &syntax::Word) -> bool {
    word.units.iter().any(|unit| match unit {
        WordUnit::Unquoted(unit) => text_unit_contains_variables(unit),
        WordUnit::SingleQuote(_) => false,
        WordUnit::DoubleQuote(text) => text.0.iter().any(text_unit_contains_variables),
        WordUnit::Tilde(_) => false,
    })
}

fn text_unit_contains_variables(unit: &TextUnit) -> bool {
    match unit {
        TextUnit::Literal(_) => false,
        TextUnit::Backslashed(_) => false,
        TextUnit::RawParam { .. } => true,
        TextUnit::BracedParam(_) => true,
        TextUnit::CommandSubst { .. } => true,
        TextUnit::Backquote { .. } => false,
        TextUnit::Arith { content, .. } => content.0.iter().any(text_unit_contains_variables),
    }
}

fn get_subshell_command_subst(command: &SimpleCommand) -> Option<&str> {
    if !command.assigns.is_empty() {
        return None;
    }

    // ensure it's a single word
    let mut words = command.words.iter();
    let word = words.next()?;
    if words.next().is_some() {
        return None;
    }

    // ensure it contains a single unit
    let mut units = word.units.iter();
    let unit = units.next()?;
    if units.next().is_some() {
        return None;
    }

    match unit {
        WordUnit::Unquoted(TextUnit::CommandSubst { content, .. }) => Some(content),
        WordUnit::DoubleQuote(text) => {
            // ensure it contains a single text unit
            let mut text = text.0.iter();
            let first = text.next()?;
            if text.next().is_some() {
                return None;
            }

            match first {
                TextUnit::CommandSubst { content, .. } => Some(content),
                _ => None,
            }
        }
        _ => None,
    }
}

pub fn validate_simple_command(
    ctx: &mut Context,
    simple: &SimpleCommand,
    function_stack: &[String],
) -> Result<()> {
    info!("Entering simple command processor");

    for assign in &simple.assigns {
        let name = assign.name.to_string();
        let value = assign.value.to_string();
        debug!("assign: {name:?}={value:?}");
    }

    // TODO: CommandSubst is not picked up if part of an arithmetic expression
    if let Some(script) = get_subshell_command_subst(simple) {
        debug!("Detected subshell command subst");
        let parsed = ast::parse(script)?;
        ast::validate_ast(ctx, &parsed, function_stack)?;
    } else {
        let mut words = simple.words.iter();
        if let Some(first) = words.next() {
            let cmd = first.to_string();
            debug!("cmd={cmd:?}");

            if function_stack.is_empty() {
                ctx.finding(format!(
                    "Function call outside of any function: {:?}",
                    simple.to_string()
                ));
            }

            if word_contains_variables(first) {
                ctx.finding(format!("Command name contains variable: {cmd:?}"));
            } else if !ctx.trusted_commands.contains(cmd.as_str()) {
                ctx.finding_conditional(
                    format!("Running unrecognized command: {:?}", simple.to_string()),
                    vec![FindingCondition::FunctionUndeclared(cmd)],
                );
            }

            for arg in words {
                let arg = arg.to_string();
                debug!("arg={arg:?}");
            }
        }
    }

    for redir in &*simple.redirs {
        validate_redir(ctx, redir)?;
    }

    info!("Exiting simple command processor");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn test_binary_builtin_no_overlap() {
        let binaries = BTreeSet::from_iter(REASONABLE_BINARIES);
        let builtins = BTreeSet::from_iter(REASONABLE_BUILTINS);
        let intersection = binaries.intersection(&builtins).collect::<Vec<_>>();
        println!("number of intersections: {}", intersection.len());
        assert_eq!(intersection, Vec::<&&&str>::new());
    }
}
