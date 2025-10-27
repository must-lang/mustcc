use colored::Colorize;

use crate::{
    common::Position,
    error::diagnostic::{Diagnostic, Label},
};

pub mod ariadne_renderer;
pub mod context;
pub mod diagnostic;

#[derive(Debug)]
pub enum InternalError {
    Any,
    AnyMsg(String),
}

impl From<ParsingError> for Diagnostic {
    fn from(value: ParsingError) -> Self {
        match value {
            ParsingError::InvalidToken { pos } => Diagnostic::error(&pos)
                .with_label(Label::new(&pos).with_msg(Box::new(|| format!("Invalid token")))),
            ParsingError::UnrecognizedEof { pos, expected } => Diagnostic::error(&pos)
                .with_label(
                    Label::new(&pos).with_msg(Box::new(|| format!("Unexpected end-of-file."))),
                )
                .with_note(format!("Expected one of:\n{}", expected.join("\n"))),
            ParsingError::UnrecognizedToken {
                pos,
                token,
                expected,
            } => Diagnostic::error(&pos)
                .with_label(Label::new(&pos).with_msg(Box::new(move || {
                    format!("Unexpected token: {}", token.bright_red())
                })))
                .with_note(format!("Expected one of:\n{}", expected.join("\n"))),
            ParsingError::ExtraToken { pos, token } => {
                Diagnostic::error(&pos).with_label(Label::new(&pos).with_msg(Box::new(move || {
                    format!("Unexpected token: {}", token.bright_red())
                })))
            }
        }
    }
}

#[derive(Debug)]
pub enum ParsingError {
    InvalidToken {
        pos: Position,
    },
    UnrecognizedEof {
        pos: Position,
        expected: Vec<String>,
    },
    UnrecognizedToken {
        pos: Position,
        token: String,
        expected: Vec<String>,
    },
    ExtraToken {
        pos: Position,
        token: String,
    },
}
