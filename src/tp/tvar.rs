static mut COUNTER: usize = 64;

/// A type variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TVar {
    id: usize,
}

impl TVar {
    /// Create a fresh type variable.
    pub(crate) fn new() -> Self {
        unsafe {
            COUNTER += 1;
            Self { id: COUNTER }
        }
    }

    /// Returns the underlying type id.
    pub fn id(&self) -> usize {
        self.id
    }

    /// Check if type variable represents numeric type.
    pub(crate) fn is_numeric(&self) -> bool {
        self.id > 2 && self.id < 32
    }

    /// Check if type variable represents the never type.
    pub(crate) fn is_never(&self) -> bool {
        self.id == 1
    }

    pub(crate) fn of_builtin(name: String) -> TVar {
        let id = match name.as_str() {
            "never" => 1,
            "bool" => 2,
            "order" => 3,
            "u8" => 4,
            "u16" => 5,
            "u32" => 6,
            "u64" => 7,
            "usize" => 8,
            "i8" => 9,
            "i16" => 10,
            "i32" => 11,
            "i64" => 12,
            "isize" => 13,
            _ => panic!("not a builtin name: {}", name),
        };
        TVar { id }
    }

    pub(crate) fn is_builtin(&self) -> bool {
        todo!()
    }

    pub(crate) fn builtin_size(&self) -> Option<usize> {
        todo!()
    }
}
