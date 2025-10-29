use crate::{
    common::Position,
    error::diagnostic::{Diagnostic, Label},
};

pub fn resursive_types(pos: &Position) -> Diagnostic {
    Diagnostic::error(pos)
        .with_label(Label::new(pos).with_msg(Box::new(move || format!("recursive type"))))
}

pub(crate) fn unsized_type(pos: &Position) -> Diagnostic {
    Diagnostic::error(pos).with_label(
        Label::new(pos).with_msg(Box::new(move || format!("this type has infinite size"))),
    )
}
