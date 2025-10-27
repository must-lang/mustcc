use std::{hash::Hash, num::NonZeroUsize};

static mut COUNTER: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TVarKind {
    Parameter,
    Type,
    TypeCons(NonZeroUsize),
}

/// A type variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord)]
pub struct TVar {
    id: usize,
    kind: TVarKind,
}

impl PartialOrd for TVar {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl Hash for TVar {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl TVar {
    /// Create a fresh type variable.
    pub(crate) fn new(kind: TVarKind) -> Self {
        unsafe {
            COUNTER += 1;
            Self { id: COUNTER, kind }
        }
    }

    /// Returns the kind of type variable.
    pub fn kind(&self) -> TVarKind {
        self.kind
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

    pub(crate) fn of_builtin(name: &str) -> TVar {
        let id = match name {
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
        TVar {
            id,
            kind: TVarKind::Type,
        }
    }

    pub(crate) fn is_builtin(&self) -> bool {
        todo!()
    }

    pub(crate) fn builtin_size(&self) -> Option<usize> {
        todo!()
    }
}
