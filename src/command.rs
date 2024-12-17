use crate::ast;
use crate::errors::*;
use crate::redirect::validate_redir;
use crate::{Context, FindingCondition};
use yash_syntax::syntax::{self, BackquoteUnit, SimpleCommand, TextUnit, Value, Word, WordUnit};

pub const REASONABLE_BINARIES: &[&str] = &[
    "/bin/true",
    "[",
    "cat",
    "chmod",
    "chown",
    "getent",
    "grep",
    "head",
    "hostname",
    "killall",
    "mkdir",
    "pgrep",
    "printf",
    "rm",
    "rmdir",
    "setcap",
    "sha256sum",
    "systemd-sysusers",
    "touch",
    "true",
    "uname",
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
        WordUnit::DollarSingleQuote(_) => false,
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

// TODO: this function can likely get removed, but code needs some rewriting
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

fn validate_text_unit(ctx: &mut Context, text: &TextUnit, function_stack: &[String]) -> Result<()> {
    trace!("text unit={text:?}");
    match text {
        TextUnit::CommandSubst { content, .. } => {
            let parsed = ast::parse(content)?;
            ast::validate_ast(ctx, &parsed, function_stack)?;
        }
        TextUnit::Backquote { content, .. } => {
            let mut script = String::new();
            for unit in content {
                match unit {
                    BackquoteUnit::Literal(c) => script.push(*c),
                    BackquoteUnit::Backslashed(c) => script.push(*c),
                }
            }
            trace!("Assembled script from backticks: {script:?}");
            let parsed = ast::parse(&script)?;
            ast::validate_ast(ctx, &parsed, function_stack)?;
        }
        _ => (),
    }

    Ok(())
}

fn validate_word(ctx: &mut Context, word: &Word, function_stack: &[String]) -> Result<()> {
    for unit in &word.units {
        match unit {
            WordUnit::Unquoted(text) => validate_text_unit(ctx, text, function_stack)?,
            WordUnit::DoubleQuote(text) => {
                for unit in &text.0 {
                    validate_text_unit(ctx, unit, function_stack)?;
                }
            }
            WordUnit::SingleQuote(_) | WordUnit::DollarSingleQuote(_) | WordUnit::Tilde(_) => (),
        }
    }

    Ok(())
}

fn validate_subshell_command_subst(
    ctx: &mut Context,
    simple: &SimpleCommand,
    function_stack: &[String],
) -> Result<()> {
    for word in &simple.words {
        validate_word(ctx, word, function_stack)?;
    }

    Ok(())
}

pub fn validate_simple_command(
    ctx: &mut Context,
    simple: &SimpleCommand,
    function_stack: &[String],
) -> Result<()> {
    info!("Entering simple command processor");
    trace!("simple={simple:?}");

    for assign in &simple.assigns {
        let name = assign.name.to_string();
        let value = assign.value.to_string();
        debug!("assign: {name:?}={value:?}");
        match &assign.value {
            Value::Scalar(word) => {
                validate_word(ctx, word, function_stack)?;
            }
            Value::Array(words) => {
                for word in words {
                    validate_word(ctx, word, function_stack)?;
                }
            }
        }
    }

    validate_subshell_command_subst(ctx, simple, function_stack)?;

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
    use crate::validate;
    use std::collections::BTreeSet;

    #[test]
    fn test_binary_builtin_no_overlap() {
        let binaries = BTreeSet::from_iter(REASONABLE_BINARIES);
        let builtins = BTreeSet::from_iter(REASONABLE_BUILTINS);
        let intersection = binaries.intersection(&builtins).collect::<Vec<_>>();
        println!("number of intersections: {}", intersection.len());
        assert_eq!(intersection, Vec::<&&&str>::new());
    }

    #[test]
    fn test_assign_subshell_command_subst() {
        let script = r#"
        post_install() {
            cmd="$(date > /tmp/pwn)"
        }

        post_upgrade() {
          post_install
        }
        "#;
        let findings = validate(script).unwrap();
        assert_eq!(
            findings,
            vec![
                "Running unrecognized command: \"date >/tmp/pwn\"",
                "File write to: \"/tmp/pwn\""
            ]
        );
    }

    #[test]
    fn test_misc_subshell_command_subst() {
        let script = r#"
        post_install() {
            x=$(echo hax > /etc/hax1)
            x=""$(echo hax > /etc/hax2)
            echo ""$(echo hax > /etc/hax3)
            echo "" $(echo hax > /etc/hax4)
            echo "" "$(echo hax > /etc/hax5)"''$(echo hax > /etc/hax6)
            arr=(a b $(echo hax > /etc/hax7) ''"$(echo hax > /etc/hax8)"'')
        }
        "#;
        let findings = validate(script).unwrap();
        assert_eq!(
            findings,
            vec![
                "File write to: \"/etc/hax1\"",
                "File write to: \"/etc/hax2\"",
                "File write to: \"/etc/hax3\"",
                "File write to: \"/etc/hax4\"",
                "File write to: \"/etc/hax5\"",
                "File write to: \"/etc/hax6\"",
                "File write to: \"/etc/hax7\"",
                "File write to: \"/etc/hax8\"",
            ]
        );
    }

    #[test]
    fn test_misc_backticks() {
        let script = r#"
        post_install() {
            x=`echo \\'hax\\' > /etc/hax1`
            x=""`echo hax > /etc/hax2`
            echo ""`echo hax > /etc/hax3`
            echo "" `echo hax > /etc/hax4`
            echo "" "`echo hax > /etc/hax5`"''`echo hax > /etc/hax6`
            arr=(a b `echo hax > /etc/hax7` ''"`echo hax > /etc/hax8`"'')
        }
        "#;
        let findings = validate(script).unwrap();
        assert_eq!(
            findings,
            vec![
                "File write to: \"/etc/hax1\"",
                "File write to: \"/etc/hax2\"",
                "File write to: \"/etc/hax3\"",
                "File write to: \"/etc/hax4\"",
                "File write to: \"/etc/hax5\"",
                "File write to: \"/etc/hax6\"",
                "File write to: \"/etc/hax7\"",
                "File write to: \"/etc/hax8\"",
            ]
        );
    }
}
