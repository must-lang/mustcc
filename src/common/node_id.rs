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

    pub(crate) fn of_builtin_type(name: &str) -> NodeID {
        let id = match name {
            "never" => 0,
            "bool" => 1,
            "order" => 2,
            "u8" => 3,
            "u16" => 4,
            "u32" => 5,
            "u64" => 6,
            "usize" => 7,
            "i8" => 8,
            "i16" => 9,
            "i32" => 10,
            "i64" => 11,
            "isize" => 12,
            _ => panic!("not a builtin name: {}", name),
        };
        NodeID { id }
    }
}
