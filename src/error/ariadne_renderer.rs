use crate::{
    common::{Position, sources::SourceMap},
    error::diagnostic::{Diagnostic, DiagnosticRenderer, Label, Severity},
};

/// Renders diagnostics using Ariadne backend.
#[derive(Debug)]
pub struct AriadneRenderer;

impl AriadneRenderer {
    pub fn new() -> Self {
        Self {}
    }
}

impl DiagnosticRenderer for AriadneRenderer {
    fn show(&self, diag: Diagnostic, sources: &SourceMap) -> std::io::Result<()> {
        let pos = diag.pos.clone();
        let filename = pos.filename.clone();
        let report = ariadne::Report::from(diag);
        let source = sources.get(&filename).ok_or(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("error renderer can't get source for {}", &*filename),
        ))?;
        report.eprint((filename, source))
    }
}

impl<'a> From<Diagnostic> for ariadne::Report<'a, Position> {
    fn from(diag: Diagnostic) -> Self {
        let kind = match diag.severity {
            Severity::Error => ariadne::ReportKind::Error,
            Severity::Warning => ariadne::ReportKind::Warning,
        };
        let pos: Position = diag.pos;
        let mut builder = ariadne::Report::build(kind, pos);
        builder.add_labels(diag.labels.into_iter().map(|label| label.into()));
        builder.with_notes(diag.notes);
        builder.finish()
    }
}

impl From<Label> for ariadne::Label<Position> {
    fn from(value: Label) -> Self {
        let color = match value.color {
            colored::Color::Black => ariadne::Color::Black,
            colored::Color::Blue => ariadne::Color::Blue,
            colored::Color::Green => ariadne::Color::Green,
            colored::Color::Red => ariadne::Color::Red,
            colored::Color::Cyan => ariadne::Color::Cyan,
            colored::Color::Magenta => ariadne::Color::Magenta,
            colored::Color::Yellow => ariadne::Color::Yellow,
            colored::Color::White => ariadne::Color::White,
            colored::Color::BrightBlack => ariadne::Color::BrightBlack,
            colored::Color::BrightRed => ariadne::Color::BrightRed,
            colored::Color::BrightGreen => ariadne::Color::BrightGreen,
            colored::Color::BrightYellow => ariadne::Color::BrightYellow,
            colored::Color::BrightBlue => ariadne::Color::BrightBlue,
            colored::Color::BrightMagenta => ariadne::Color::BrightMagenta,
            colored::Color::BrightCyan => ariadne::Color::BrightCyan,
            colored::Color::BrightWhite => ariadne::Color::BrightWhite,
            colored::Color::TrueColor { r, g, b } => ariadne::Color::Rgb(r, g, b),
        };
        ariadne::Label::new(value.pos)
            .with_color(color)
            .with_message(value.msg)
    }
}
