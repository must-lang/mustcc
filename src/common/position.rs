use std::sync::Arc;

use ariadne::Span;

#[derive(Debug, Clone)]
pub struct Position {
    pub filename: Arc<str>,
    pub start: usize,
    pub end: usize,
}

impl Position {
    pub fn new(filename: Arc<str>, start: usize, end: usize) -> Self {
        Position {
            filename,
            start,
            end,
        }
    }
    pub fn nowhere() -> Self {
        Self {
            filename: Arc::from("<nowhere>"),
            start: 0,
            end: 0,
        }
    }
    pub fn generator(filename: Arc<str>) -> PositionGenerator {
        PositionGenerator { filename }
    }
}

impl Span for Position {
    type SourceId = Arc<str>;

    fn source(&self) -> &Self::SourceId {
        &self.filename
    }

    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.end
    }
}

#[derive(Debug, Clone)]
pub struct PositionGenerator {
    filename: Arc<str>,
}

impl PositionGenerator {
    pub fn make(&self, start: usize, end: usize) -> Position {
        Position {
            filename: self.filename.clone(),
            start,
            end,
        }
    }
}
