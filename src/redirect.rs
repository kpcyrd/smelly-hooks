use crate::errors::*;
use crate::Context;
use yash_syntax::syntax::{self, RedirBody, RedirOp};

pub fn validate_redir(ctx: &mut Context, redir: &syntax::Redir) -> Result<()> {
    // TODO: for inputs we should check redir.fd is none
    match &redir.body {
        RedirBody::Normal { operator, operand } => match operator {
            RedirOp::FileIn => {
                if let Some(fd) = redir.fd {
                    ctx.finding(format!("File input on unusual descriptor: fd={fd:?}"));
                }
            }
            RedirOp::FileInOut => {
                ctx.finding(format!(
                    "Redirects are not being fully checked yet: operator={operator:?}, operand={operand:?}"
                ));
            }
            RedirOp::FileOut => {
                let file = operand.to_string();
                if file != "/dev/null" {
                    ctx.finding(format!("File write to: {:?}", operand.to_string()));
                }
            }
            RedirOp::FileAppend => {
                let file = operand.to_string();
                if file != "/dev/null" {
                    ctx.finding(format!("File write to: {:?}", operand.to_string()));
                }
            }
            RedirOp::FileClobber => {
                ctx.finding(format!(
                    "Redirects are not being fully checked yet: operator={operator:?}, operand={operand:?}"
                ));
            }
            RedirOp::FdIn => {
                ctx.finding(format!(
                    "Redirects are not being fully checked yet: operator={operator:?}, operand={operand:?}"
                ));
            }
            RedirOp::FdOut => {
                let fd = operand.to_string();
                if fd != "1" && fd != "2" && fd != "-" {
                    ctx.finding(format!(
                        "File descriptor redirect to unusual descriptor: {fd:?}"
                    ));
                }
            }
            RedirOp::Pipe => {
                ctx.finding(format!(
                    "Redirects are not being fully checked yet: operator={operator:?}, operand={operand:?}"
                ));
            }
            RedirOp::String => {
                ctx.finding(format!(
                    "Redirects are not being fully checked yet: operator={operator:?}, operand={operand:?}"
                ));
            }
        },
        RedirBody::HereDoc(_) => (),
    }

    Ok(())
}
