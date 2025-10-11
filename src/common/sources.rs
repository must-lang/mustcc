use std::{collections::BTreeMap, sync::Arc};

use ariadne::Source;

#[derive(Debug, Clone)]
pub struct SourceMap {
    map: BTreeMap<Arc<str>, Source>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, filename: Arc<str>, source: String) {
        let source = Source::from(source);
        if let Some(_) = self.map.insert(filename, source) {
            panic!("adding the same filepath twice to source map")
        }
    }

    pub fn get(&self, filename: &Arc<str>) -> Option<&Source> {
        self.map.get(filename)
    }
}
