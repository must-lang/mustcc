use std::{hash::Hash, num::NonZeroUsize};

use crate::{mir, symtable::layout::Type, tp::BUILTIN_TYPES};

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
        self.id == 0
    }

    pub(crate) fn of_builtin(name: &str) -> TVar {
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
        TVar {
            id,
            kind: TVarKind::Type,
        }
    }

    pub(crate) fn is_builtin(&self) -> bool {
        self.id < 65
    }

    pub(crate) fn builtin_size(&self) -> Option<u32> {
        let size = if self.id < 13 {
            match BUILTIN_TYPES[self.id] {
                "never" => 42,
                "bool" => 1,
                "order" => 1,
                "u8" => 1,
                "u16" => 2,
                "u32" => 4,
                "u64" => 8,
                "usize" => 8,
                "i8" => 1,
                "i16" => 2,
                "i32" => 4,
                "i64" => 8,
                "isize" => 8,
                _ => return None,
            }
        } else {
            return None;
        };
        Some(size)
    }

    pub fn builtin_as_primitive(&self) -> Option<Type> {
        let tp = if self.id < 13 {
            match BUILTIN_TYPES[self.id] {
                "never" => todo!(),
                "bool" => todo!(),
                "order" => todo!(),
                "u8" => Type::Tu8,
                "u16" => Type::Tu16,
                "u32" => Type::Tu32,
                "u64" => Type::Tu64,
                "usize" => Type::Tusize,
                "i8" => Type::Ti8,
                "i16" => Type::Ti16,
                "i32" => Type::Ti32,
                "i64" => Type::Ti64,
                "isize" => Type::Tisize,
                _ => return None,
            }
        } else {
            return None;
        };
        Some(tp)
    }
}
