//! This ast is a result of parsing a project directory.
//!
//! Some of the files might be undeclared and this will be reported later on
//! when building module tree.

// ==== Imports ================================================================

use crate::common::{Ident, Path, Position, RAttribute, Visibility};
use std::collections::BTreeMap;

// ==== Program ================================================================

/// Program is a list of files implementing modules.
///
/// Each file has a „module path” in which it is declared.
/// `foo/bar/mod.mst` and `foo/bar.mst` are implementing the same module
/// with a path `[foo, bar]`.
#[derive(Debug)]
pub struct Program {
    pub file_map: BTreeMap<Vec<String>, Module>,
}

// ==== Top level ==============================================================

/// A module.
/// ```mst
/// @attributes
/// (pub) mod name {
///     <items>
/// }
/// ```
#[derive(Debug)]
pub struct Module {
    pub attributes: Vec<RAttribute>,
    pub visibility: Visibility,
    pub name: Ident,
    pub items: Vec<ModuleItem>,
    pub pos: Position,
}

/// Any item that can be declared inside of a module.
#[derive(Debug)]
pub enum ModuleItem {
    Module(Module),
    ModuleDecl(ModuleDecl),
    Import(Import),
    Func(Func),
    Struct(Struct),
    Enum(Enum),
    Error,
}

// ==== Module items ===========================================================

/// A declaration of a module implemented in a separate file.
///
/// ```mst
/// @attributes
/// (pub) mod name;
/// ```
#[derive(Debug)]
pub struct ModuleDecl {
    pub attributes: Vec<RAttribute>,
    pub visibility: Visibility,
    pub name: Ident,
    pub pos: Position,
}

/// Import of an item/items from another module/namespace.
///
/// ```mst
/// (pub) import foo::{bar::{test, baz as bas}, qux::*};
/// ```
#[derive(Debug)]
pub struct Import {
    pub visibility: Visibility,
    pub path: ImportPathNode,
    pub pos: Position,
}

/// Function declaration.
///
/// If return type is not specified, it defaults to `unit`.
/// Function body can be ommited for external and builtin functions.
///
/// ```mst
/// @attributes
/// (pub) fn name(arg: type, mut arg: type) -> type {
///     <expr>
/// }
///
/// @attributes
/// (pub) fn builtin_fn(arg: type);
/// ```
#[derive(Debug)]
pub struct Func {
    pub attributes: Vec<RAttribute>,
    pub visibility: Visibility,
    pub name: Ident,
    pub args: Vec<FnArg>,
    pub ret_type: Option<RTypeNode>,
    pub body: Option<ExprNode>,
    pub pos: Position,
}

/// Declaration of structure type.
///
/// ```mst
/// @attributes
/// (pub) struct Name {
///     field_name: type
/// } with {
///     fn new() -> Name { ... }
/// }
/// ```
#[derive(Debug)]
pub struct Struct {
    pub attributes: Vec<RAttribute>,
    pub visibility: Visibility,
    pub name: Ident,
    pub fields: Vec<(Ident, RTypeNode)>,
    pub pos: Position,
    pub methods: Vec<Func>,
}

/// Declaration of enum type.
///
/// ```mst
/// @attributes
/// (pub) enum Name {
///     Cons1(arg1, arg2, arg3),
///     Cons2(arg4, arg5),
/// } with {
///     fn new() -> name { ... }
/// }
/// ```
#[derive(Debug)]
pub struct Enum {
    pub attributes: Vec<RAttribute>,
    pub visibility: Visibility,
    pub name: Ident,
    pub constructors: Vec<Constructor>,
    pub pos: Position,
    pub methods: Vec<Func>,
}

// ==== Others =================================================================

/// Convienience wrapper for import path.
#[derive(Debug)]
pub struct ImportPathNode {
    pub pos: Position,
    pub data: ImportPathData,
}

/// A path used in import statement.
#[derive(Debug)]
pub enum ImportPathData {
    /// Glob import all items from a namespace.
    ///
    /// ```mst
    /// import std::*;
    /// ```
    All,

    /// Import one item with an optional alias.
    ///
    /// ```mst
    /// import std::io::println as pln;
    /// ```
    Exact(Ident, Option<Ident>),

    /// Import many items from a namespace.
    ///
    /// ```mst
    /// import std::{io, mem, fs};
    /// ```
    Many(Vec<ImportPathNode>),

    /// A path node.
    ///
    /// ```mst
    /// import std::foo::bar::...;
    /// ```
    Path(Ident, Box<ImportPathNode>),
}

/// An argument to a function.
#[derive(Debug)]
pub enum FnArg {
    Named {
        is_mut: bool,
        name: Ident,
        tp: RTypeNode,
        pos: Position,
    },
    NSelf {
        is_mut: bool,
        pos: Position,
    },
    PtrSelf(Position),
    MutPtrSelf(Position),
}

/// Constructor of an enum.
#[derive(Debug)]
pub enum Constructor {
    /// A tuple variant.
    ///
    /// ```mst
    /// enum Message {
    ///     Empty,
    ///     Draft(String),
    ///     Sent(MsgID, String),
    /// }
    /// ```
    Tuple {
        attributes: Vec<RAttribute>,
        name: Ident,
        pos: Position,
        params: Vec<RTypeNode>,
    },
    /// A struct variant.
    ///
    /// ```mst
    /// enum Expr {
    ///     Let {
    ///         name: str,
    ///         value: Val,
    ///     },
    ///     Fn {
    ///         name: str,
    ///         args: Vec<Expr>,
    ///     },
    /// }
    /// ```
    Struct {
        attributes: Vec<RAttribute>,
        name: Ident,
        pos: Position,
        params: Vec<(Ident, RTypeNode)>,
    },
}

// ==== Expressions ============================================================

/// Convienience wrapper for expr.
#[derive(Debug)]
pub struct ExprNode {
    pub data: ExprData,
    pub pos: Position,
}

/// Expressions.
#[derive(Debug)]
pub enum ExprData {
    /// Parser error.
    Error,
    /// Access of a variable.
    Var(Path),
    /// Numeric literal.
    Number(usize),
    /// Character literal.
    Char(char),
    /// String literal.
    String(String),
    /// Tuple.
    Tuple(Vec<ExprNode>),
    /// Function call. LHS is callable expression, RHS are arguments in order.
    ///
    /// It's important to note that all tuple variant constructors will be
    /// treated as a function call.
    FunCall(Box<ExprNode>, Vec<ExprNode>),
    /// Method call.
    ///
    /// LHS is object on which method will be called,
    /// Ident is the name of the method,
    /// RHS are arguments.
    MethodCall(Box<ExprNode>, Ident, Vec<ExprNode>),
    /// Field access to a struct type/variant.
    FieldAccess(Box<ExprNode>, Ident),
    /// Block of semicolon-separated expressions.
    ClosedBlock(Vec<ExprNode>),
    /// Block of semicolon-separated expressions and a last expression
    /// being the „return value” of the block.
    OpenBlock(Vec<ExprNode>, Box<ExprNode>),
    /// Return from the function early.
    ///
    /// If no value is specified, it defaults to `unit`.
    Return(Option<Box<ExprNode>>),
    /// Create a new variable, possible mutable.
    ///
    /// Type can be ommited if it's unambiguous.
    Let {
        name: Ident,
        is_mut: bool,
        tp: Option<RTypeNode>,
        expr: Box<ExprNode>,
    },
    /// Pattern matching on values.
    Match(Box<ExprNode>, Vec<MatchClause>),
    /// Get a pointer to value.
    Ref(Box<ExprNode>),
    /// Get a pointer to mutable value.
    RefMut(Box<ExprNode>),
    /// Dereference the pointer.
    Deref(Box<ExprNode>),
    /// If-then-else control statement.
    ///
    /// If else block is ommited, it defaults to `unit`.
    If(Box<ExprNode>, Box<ExprNode>, Option<Box<ExprNode>>),
    /// While control statement.
    While(Box<ExprNode>, Box<ExprNode>),
    /// Struct type/variant constructor.
    StructCons(Path, Vec<(Ident, ExprNode)>),
    /// Assignment, (mut) LHS = RHS.
    Assign(Box<ExprNode>, Box<ExprNode>),
}

// ==== Pattern matching =======================================================

/// A pattern matching clause.
///
/// ```mst
/// <pattern> => expr
/// ```
#[derive(Debug)]
pub struct MatchClause {
    pub pattern: PatternNode,
    pub expr: ExprNode,
    pub pos: Position,
}

/// Convienience wrapper for patterns.
#[derive(Debug)]
pub struct PatternNode {
    pub data: PatternData,
    pub pos: Position,
}

/// Pattern that can be matched against values.
#[derive(Debug)]
pub enum PatternData {
    /// `_` matches everything and discards the value.
    Wildcard,
    /// Match numeric literal.
    Number(usize),
    /// Match anything and bind it to a variable.
    Var(Ident),
    /// Match tuple.
    Tuple(Vec<PatternNode>),
    /// Match tuple variant constructor.
    TupleCons(Path, Vec<PatternNode>),
}

// ==== Types ==================================================================

/// Convienience wrapper for raw types.
#[derive(Debug)]
pub struct RTypeNode {
    pub data: RTypeData,
    pub pos: Position,
}

/// Raw type annotation inserted by the user.
#[derive(Debug)]
pub enum RTypeData {
    /// Type represented by a specific name.
    /// ```mst
    /// x : usize
    /// ```
    Var(Path),

    /// A tuple.
    /// ```mst
    /// x : (i32, char)
    /// ```
    Tuple(Vec<RTypeNode>),

    /// An array with a compile-time known length.
    /// ```mst
    /// x : [5]i32
    /// ```
    Array(usize, Box<RTypeNode>),

    /// An immutable pointer.
    /// ```mst
    /// x : *unit
    /// ```
    Ptr(Box<RTypeNode>),

    /// A mutable pointer.
    /// ```mst
    /// x : *mut char
    /// ```
    MutPtr(Box<RTypeNode>),

    /// An immutable slice.
    ///
    /// Slice is a fat pointer representing the data location
    /// and its' length in terms of element count.
    ///
    /// ```mst
    /// x : []u8
    /// ```
    Slice(Box<RTypeNode>),

    /// A mutable slice.
    ///
    /// Slice is a fat pointer representing the data location
    /// and its' length in terms of element count.
    ///
    /// ```mst
    /// x : []mut bool
    /// ```
    MutSlice(Box<RTypeNode>),

    /// A function (pointer).
    /// ```mst
    /// x : fn(i32, i32) -> bool
    /// ```
    Fun(Vec<RTypeNode>, Box<RTypeNode>),
}
