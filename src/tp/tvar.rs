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

    /// Check if type variable represents numeric type.
    pub(crate) fn is_numeric(&self) -> bool {
        todo!()
    }

    /// Check if type variable represents the never type.
    pub(crate) fn is_never(&self) -> bool {
        todo!()
    }

    pub(crate) fn of_builtin(name: String) -> TVar {
        let id = match name.as_str() {
            "never" => 1,
            "bool" => 2,
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
