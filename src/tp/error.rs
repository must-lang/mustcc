use crate::{
    common::Position,
    error::diagnostic::{Diagnostic, Label},
};

pub fn type_params_mismatch(pos: &Position, exp: usize, got: usize) -> Diagnostic {
    Diagnostic::error(pos).with_label(Label::new(pos).with_msg(Box::new(move || {
        format!("expected {} type parameters, but got {}", exp, got)
    })))
}
