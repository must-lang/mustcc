use colored::Color;

use crate::common::{Position, sources::SourceMap};

/// Severity of a diagnostic.
///
/// Any error aborts compilation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// Represents compiler diagnostic.
///
/// Diagnostic is an error in user's code.
#[derive(Debug)]
pub struct Diagnostic {
    pub severity: Severity,
    pub labels: Vec<Label>,
    pub notes: Vec<String>,
    pub pos: Position,
}

impl Diagnostic {
    pub(crate) fn error(pos: &Position) -> Diagnostic {
        Self {
            severity: Severity::Error,
            pos: pos.clone(),
            labels: vec![],
            notes: vec![],
        }
    }

    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    pub(crate) fn with_note(mut self, note: String) -> Diagnostic {
        self.notes.push(note);
        self
    }
}

/// Label included with a diagnostic.
#[derive(Debug)]
pub struct Label {
    pub pos: Position,
    pub msg: String,
    pub color: Color,
}

impl Label {
    pub fn new(pos: &Position) -> Self {
        Label {
            pos: pos.clone(),
            msg: "<no message for this error>".into(),
            color: colored::Color::Red,
        }
    }

    pub fn with_msg(mut self, msg: String) -> Self {
        self.msg = msg;
        self
    }
}

/// Implementors of this trait can be used as diagnostic sinks.
pub trait DiagnosticRenderer: Send + Sync + std::fmt::Debug {
    /// Show the diagnostic.
    fn show(&self, diag: Diagnostic, sources: &SourceMap) -> std::io::Result<()>;
}
