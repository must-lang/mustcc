use crate::{
    common::Position,
    error::diagnostic::{Diagnostic, Label},
};

pub fn already_bound(pos: &Position, name: String) -> Diagnostic {
    Diagnostic::error(pos).with_label(
        Label::new(pos).with_msg(Box::new(move || format!("{} is already bound", name))),
    )
}

pub(crate) fn self_on_free_function(pos: &Position) -> Diagnostic {
    Diagnostic::error(pos).with_label(Label::new(pos).with_msg(Box::new(move || {
        format!("invalid self parameter on a free function")
    })))
}

pub(crate) fn function_with_no_body(pos: &Position) -> Diagnostic {
    Diagnostic::error(pos)
        .with_label(Label::new(pos).with_msg(Box::new(|| format!("local function without a body"))))
}

pub(crate) fn field_duplicate(pos: &Position, name: String) -> Diagnostic {
    Diagnostic::error(pos).with_label(Label::new(pos).with_msg(Box::new(move || {
        format!("field `{}` initialized more than once", name)
    })))
}

pub(crate) fn expected_type_got_var(pos: &Position) -> Diagnostic {
    Diagnostic::error(pos)
        .with_label(Label::new(pos).with_msg(Box::new(|| format!("expected type, found variable"))))
}

pub(crate) fn local_type(pos: &Position) -> Diagnostic {
    Diagnostic::error(&pos)
        .with_label(Label::new(&pos).with_msg(Box::new(|| format!("this is a local type"))))
}
