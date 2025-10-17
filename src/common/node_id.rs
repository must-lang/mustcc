static mut COUNTER: usize = 64;

/// Id representing a top-level declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeID {
    id: usize,
}

impl NodeID {
    /// Create a fresh node id.
    pub(crate) fn new_global() -> NodeID {
        unsafe {
            COUNTER += 1;
            NodeID { id: COUNTER }
        }
    }

    /// Get the id of root node.
    pub(crate) fn of_root() -> NodeID {
        NodeID { id: 0 }
    }

    /// Get the actual numeric id.
    pub fn get(&self) -> usize {
        self.id
    }
}
