use crate::{
    common::Position,
    error::diagnostic::{Diagnostic, Label},
    tp::Type,
};

pub(crate) fn type_mismatch(pos: Position, exp: Type, got: Type) -> Diagnostic {
    Diagnostic::error(&pos).with_label(Label::new(&pos).with_msg(Box::new(move || {
        format!("Type mismatch. Expected: {}, Got: {}", exp, got)
    })))
}

pub(crate) fn expected_mutable(pos: Position) -> Diagnostic {
    Diagnostic::error(&pos).with_label(Label::new(&pos).with_msg(Box::new(move || {
        format!("cannot assign to immutable variable")
    })))
}

pub(crate) fn not_a_function(pos: Position) -> Diagnostic {
    Diagnostic::error(&pos).with_label(Label::new(&pos).with_msg(Box::new(move || {
        format!("it's not a function and cannot be called")
    })))
}

pub(crate) fn missing_argument(pos: Position, id: usize, tp: Type) -> Diagnostic {
    Diagnostic::error(&pos).with_label(Label::new(&pos).with_msg(Box::new(move || {
        format!("missing arg #{} of type {}", id, tp)
    })))
}

pub(crate) fn unexpected_argument(id: usize, pos: Position) -> Diagnostic {
    Diagnostic::error(&pos)
        .with_label(Label::new(&pos).with_msg(Box::new(move || format!("unexpected arg #{}", id))))
}

pub(crate) fn no_such_field(field_name: String, arg: Type, pos: &Position) -> Diagnostic {
    Diagnostic::error(&pos).with_label(Label::new(&pos).with_msg(Box::new(move || {
        format!("no field named {} on type {}", field_name, arg)
    })))
}

pub(crate) fn missing_field(pos: Position, f_name: String, f_type: Type) -> Diagnostic {
    Diagnostic::error(&pos).with_label(Label::new(&pos).with_msg(Box::new(move || {
        format!("missing field `{}` of type {}", f_name, f_type)
    })))
}

pub(crate) fn unbound_field(pos: Position, f_name: String) -> Diagnostic {
    Diagnostic::error(&pos).with_label(
        Label::new(&pos).with_msg(Box::new(move || format!("unbound field `{}`", f_name))),
    )
}

pub(crate) fn unbound_method(pos: Position, method_name: String) -> Diagnostic {
    Diagnostic::error(&pos).with_label(Label::new(&pos).with_msg(Box::new(move || {
        format!("unbound method `{}`", method_name)
    })))
}

pub(crate) fn unsolved_uvar(pos: Position, tp: Type) -> Diagnostic {
    Diagnostic::error(&pos).with_label(Label::new(&pos).with_msg(Box::new(move || {
        format!("unsolved unification variable `{}`", tp)
    })))
}

pub(crate) fn cannot_infer_type(pos: &Position) -> Diagnostic {
    Diagnostic::error(&pos).with_label(Label::new(&pos).with_msg(Box::new(move || {
        format!("cannot infer type, please annotate")
    })))
}

pub(crate) fn not_yet_supported(pos: &Position) -> Diagnostic {
    Diagnostic::error(&pos)
        .with_label(Label::new(&pos).with_msg(Box::new(move || format!("not yet supported"))))
}
