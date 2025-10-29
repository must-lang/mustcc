use std::sync::Arc;

use ariadne::Source;

use crate::{
    common::sources::SourceMap,
    error::{
        InternalError,
        diagnostic::{Diagnostic, DiagnosticRenderer},
    },
};

/// Represents context of compilation.
pub struct Context {
    renderer: Box<dyn DiagnosticRenderer>,
    diagnostics: Vec<Diagnostic>,
    sources: SourceMap,
    err_count: usize,
}

impl Context {
    /// Create a new context.
    pub(crate) fn init(renderer: Box<dyn DiagnosticRenderer>) -> Self {
        Self {
            renderer,
            diagnostics: vec![],
            sources: SourceMap::new(),
            err_count: 0,
        }
    }

    /// Print all diagnostic using provided renderer and destroy context.
    pub(crate) fn finish(self) -> Result<usize, InternalError> {
        for diag in self.diagnostics {
            self.renderer
                .show(diag, &self.sources)
                .map_err(|e| InternalError::AnyMsg(format!("Failed to show diagnostic: {e}")))?
        }
        Ok(self.err_count)
    }

    /// Add a diagnostic to this context.
    pub(crate) fn report(&mut self, diag: Diagnostic) {
        self.err_count += 1;
        self.diagnostics.push(diag);
    }

    /// Add source.
    pub(crate) fn add_source(&mut self, filename: Arc<str>, source: String) {
        self.sources.add(filename, source);
    }

    /// Get source associated with given filename.
    pub(crate) fn get_source(&self, filename: &Arc<str>) -> Option<&Source> {
        self.sources.get(filename)
    }
}
