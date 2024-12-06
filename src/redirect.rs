use crate::errors::*;
use yash_syntax::syntax::{self, RedirBody, RedirOp};

pub fn validate_redir(redir: &syntax::Redir, findings: &mut Vec<String>) -> Result<()> {
    // TODO: for inputs we should check redir.fd is none
    match &redir.body {
        RedirBody::Normal { operator, operand } => match operator {
            RedirOp::FileIn => {
                if let Some(fd) = redir.fd {
                    findings.push(format!("File input on unusual descriptor: fd={fd:?}"));
                }
            }
            RedirOp::FileInOut => {
                findings.push(format!(
                    "Redirects are not being fully checked yet: operator={operator:?}, operand={operand:?}"
                ));
            }
            RedirOp::FileOut => {
                let file = operand.to_string();
                if file != "/dev/null" {
                    findings.push(format!("File write to: {:?}", operand.to_string()));
                }
            }
            RedirOp::FileAppend => {
                let file = operand.to_string();
                if file != "/dev/null" {
                    findings.push(format!("File write to: {:?}", operand.to_string()));
                }
            }
            RedirOp::FileClobber => {
                findings.push(format!(
                    "Redirects are not being fully checked yet: operator={operator:?}, operand={operand:?}"
                ));
            }
            RedirOp::FdIn => {
                findings.push(format!(
                    "Redirects are not being fully checked yet: operator={operator:?}, operand={operand:?}"
                ));
            }
            RedirOp::FdOut => {
                let fd = operand.to_string();
                if fd != "1" && fd != "2" && fd != "-" {
                    findings.push(format!(
                        "File descriptor redirect to unusual descriptor: {fd:?}"
                    ));
                }
            }
            RedirOp::Pipe => {
                findings.push(format!(
                    "Redirects are not being fully checked yet: operator={operator:?}, operand={operand:?}"
                ));
            }
            RedirOp::String => {
                findings.push(format!(
                    "Redirects are not being fully checked yet: operator={operator:?}, operand={operand:?}"
                ));
            }
        },
        RedirBody::HereDoc(_) => (),
    }

    Ok(())
}
