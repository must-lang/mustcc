use crate::{
    common::Position,
    error::diagnostic::{Diagnostic, Label},
    tp::Type,
};

pub(crate) fn type_mismatch(pos: Position, exp: &Type, got: &Type) -> Diagnostic {
    Diagnostic::error(&pos).with_label(
        Label::new(&pos).with_msg(format!("Type mismatch. Expected: {}, Got: {}", exp, got)),
    )
}

pub(crate) fn expected_mutable(pos: Position) -> Diagnostic {
    todo!()
}

pub(crate) fn not_a_function(pos: Position) -> Diagnostic {
    todo!()
}

pub(crate) fn missing_argument(id: usize, arg: &Type) -> Diagnostic {
    todo!()
}

pub(crate) fn unexpected_argument(id: usize, pos: Position) -> Diagnostic {
    todo!()
}

pub(crate) fn no_such_field(field_name: String, arg: &Type, pos: &Position) -> Diagnostic {
    Diagnostic::error(&pos).with_label(
        Label::new(&pos).with_msg(format!("no field named {} on type {}", field_name, arg)),
    )
}
