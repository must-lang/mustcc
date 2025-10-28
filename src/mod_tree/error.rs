use crate::{
    common::Position,
    error::diagnostic::{Diagnostic, Label},
};

pub fn missing_module(pos: &Position, name: String) -> Diagnostic {
    Diagnostic::error(pos)
        .with_label(Label::new(pos).with_msg(Box::new(move || format!("missing module: {}", name))))
}

pub fn unbound_variable(pos: &Position, name: String) -> Diagnostic {
    Diagnostic::error(pos).with_label(
        Label::new(pos).with_msg(Box::new(move || format!("unbound variable: {}", name))),
    )
}

pub fn ambiguous_symbol(pos: &Position, name: String) -> Diagnostic {
    Diagnostic::error(pos)
        .with_label(Label::new(pos).with_msg(Box::new(move || format!("{} is ambiguous", name))))
}

pub fn cannot_import_from(pos: &Position, name: String) -> Diagnostic {
    Diagnostic::error(pos).with_label(
        Label::new(pos).with_msg(Box::new(move || format!("cannot import from {}", name))),
    )
}

pub fn private_item(pos: &Position, name: String) -> Diagnostic {
    Diagnostic::error(pos)
        .with_label(Label::new(pos).with_msg(Box::new(move || format!("{} is private", name))))
}

pub fn already_bound(pos: &Position, name: String) -> Diagnostic {
    Diagnostic::error(pos).with_label(
        Label::new(pos).with_msg(Box::new(move || format!("{} is already bound", name))),
    )
}
