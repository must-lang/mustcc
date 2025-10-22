use crate::{common::Position, error::diagnostic::Diagnostic, tp::Type};

pub(crate) fn type_mismatch(pos: Position, exp_tp: &Type, tp: &Type) -> Diagnostic {
    todo!()
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
