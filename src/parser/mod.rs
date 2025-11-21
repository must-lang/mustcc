use std::{
    collections::BTreeMap,
    fs::read_to_string,
    hint::unreachable_unchecked,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    common::Position,
    error::{InternalError, ParsingError, context::Context},
};

pub mod ast;

lalrpop_util::lalrpop_mod!(pub parser, "/parser/parser.rs");

/// Parses the entire `src` directory ignoring files without „mst” extension.
///
/// Expects that CWD is set to project root.
pub fn parse_project(root: &Path, ctx: &mut Context) -> Result<ast::Program, InternalError> {
    let mut path = PathBuf::from(root);
    path.push("src");
    let files = get_files(&mut path)?;
    let mut file_map = BTreeMap::new();
    for file in files {
        let path = file.strip_prefix(root).unwrap();
        let module_path = get_module_path(path)?;
        let parsed_file = parse_file(ctx, file)?;
        if let Some(module) = parsed_file {
            if let Some(_) = file_map.insert(module_path, module) {
                panic!("same module defined twice")
            }
        }
    }
    let prog = ast::Program { file_map };
    Ok(prog)
}

/// Return the module path of file path.
///
/// Both `bar.mst` and `bar/mod.mst` result in path `[bar]`.
fn get_module_path(path: &Path) -> Result<Vec<String>, InternalError> {
    let mut module_path = vec![];
    let iter: Vec<&str> = path
        .components()
        .rev()
        .map(|c| {
            Ok(c.as_os_str()
                .to_str()
                .ok_or(InternalError::AnyMsg("couldn't parse filename".into()))?)
        })
        .collect::<Result<_, InternalError>>()?;
    let mut iter = iter.into_iter();
    match iter.next() {
        Some("mod.mst") => (),
        Some(v) => {
            let v = v.strip_suffix(".mst").ok_or(InternalError::AnyMsg(
                "invalid file extension, should be skipped".into(),
            ))?;
            module_path.push(v)
        }
        None => return Err(InternalError::AnyMsg("empty filepath".into())),
    }
    module_path.extend(iter);
    module_path.reverse();
    let module_path = module_path.into_iter().map(|s| s.to_string()).collect();
    Ok(module_path)
}

/// Collects all source files recursively in given path.
///
/// Skips files without proper extension.
fn get_files(arg: &mut PathBuf) -> Result<Vec<PathBuf>, InternalError> {
    let mut paths = vec![];

    for file in arg.read_dir().expect("should be a directory") {
        if let Ok(file) = file {
            let ft = file
                .file_type()
                .map_err(|_| InternalError::AnyMsg("couldn't get file extension".into()))?;
            if ft.is_dir() {
                arg.push(&file.file_name());
                let new_paths = get_files(arg)?;
                paths.extend(new_paths);
                arg.pop();
            }
            if let Some("mst") = file.path().extension().map(|s| s.to_str().unwrap())
                && ft.is_file()
            {
                arg.push(&file.file_name());
                paths.push(arg.clone());
                arg.pop();
            }
        }
    }
    Ok(paths)
}

/// Parse a single source file.
fn parse_file(ctx: &mut Context, filename: PathBuf) -> Result<Option<ast::Module>, InternalError> {
    let source = read_to_string(&filename).map_err(|_| {
        InternalError::AnyMsg(format!("should be able to open file: {:#?}", filename))
    })?;
    let filename: Arc<str> = filename
        .to_str()
        .ok_or(InternalError::AnyMsg(
            "can't parse filename into str".into(),
        ))?
        .into();
    let mut errors = vec![];

    let pg = Position::generator(filename.clone());
    ctx.add_source(filename.clone(), source);
    let source = ctx
        .get_source(&filename)
        .expect("the source was added in previous line")
        .text();
    let res = match parser::FileParser::new().parse(&mut errors, &pg, &source) {
        Ok(r) => Some(r),
        Err(e) => {
            errors.push(e);
            None
        }
    };

    let errors: Vec<ParsingError> = errors
        .into_iter()
        .map(|err| match err {
            lalrpop_util::ParseError::InvalidToken { location } => {
                let pos = pg.make(location, location);
                ParsingError::InvalidToken { pos }
            }
            lalrpop_util::ParseError::UnrecognizedEof { location, expected } => {
                let pos = pg.make(location, location);
                ParsingError::UnrecognizedEof { pos, expected }
            }
            lalrpop_util::ParseError::UnrecognizedToken { token, expected } => {
                let pos = pg.make(token.0, token.2);
                let token = token.1.to_string();
                ParsingError::UnrecognizedToken {
                    pos,
                    token,
                    expected,
                }
            }
            lalrpop_util::ParseError::ExtraToken { token } => {
                let pos = pg.make(token.0, token.2);
                let token = token.1.to_string();
                ParsingError::ExtraToken { pos, token }
            }
            // There are no user-defined errors in the parser
            lalrpop_util::ParseError::User { .. } => unsafe { unreachable_unchecked() },
        })
        .collect();

    for err in errors {
        ctx.report(err.into());
    }

    Ok(res)
}

/// TODO: enable reporting string errors through the parser.
/// Now it will panic if this function doesn't succeed
pub fn unescape_json_string(s: &str) -> Result<String, String> {
    // Strip surrounding quotes
    let raw = &s[1..s.len() - 1];

    println!("{:#?}", raw);

    let mut result = String::new();
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => result.push('"'),
                Some('\\') => result.push('\\'),
                Some('/') => result.push('/'),
                Some('b') => result.push('\u{0008}'),
                Some('f') => result.push('\u{000C}'),
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('u') => {
                    // Expect 4 hex digits
                    let code: String = chars.by_ref().take(4).collect();
                    if code.len() == 4 {
                        if let Ok(num) = u16::from_str_radix(&code, 16) {
                            if let Some(ch) = char::from_u32(num as u32) {
                                result.push(ch);
                            } else {
                                return Err(format!("Invalid unicode escape: {}", code));
                            }
                        } else {
                            return Err(format!("Bad hex in unicode escape: {}", code));
                        }
                    } else {
                        return Err("Incomplete unicode escape".into());
                    }
                }
                Some(other) => return Err(format!("Invalid escape: \\{}", other)),
                None => return Err("Incomplete escape".into()),
            }
        } else {
            result.push(c);
        }
    }

    Ok(result)
}

pub fn parse_char_literal(s: &str) -> Result<u8, String> {
    // Expect format: `'x'` or `'\xNN'` or `'\n'`
    if !s.starts_with('\'') || !s.ends_with('\'') {
        return Err("invalid char literal".into());
    }

    let inner = &s[1..s.len() - 1];
    let bytes = inner.as_bytes();

    // Case 1: normal one-character literal: `'a'`
    if bytes.len() == 1 {
        return Ok(bytes[0]);
    }

    // Case 2: escaped literal: starts with '\'
    if bytes.len() >= 2 && bytes[0] == b'\\' {
        match bytes[1] {
            b'a' => return Ok(0x07),
            b'b' => return Ok(0x08),
            b'f' => return Ok(0x0C),
            b'n' => return Ok(0x0A),
            b'r' => return Ok(0x0D),
            b't' => return Ok(0x09),
            b'v' => return Ok(0x0B),
            b'\\' => return Ok(b'\\'),
            b'\'' => return Ok(b'\''),
            b'"' => return Ok(b'"'),
            b'?' => return Ok(b'?'),
            b'x' => {
                // \xNN (1–2 hex digits)
                let hex = &inner[2..];
                if hex.is_empty() || hex.len() > 2 {
                    return Err("invalid hex escape".into());
                }
                return u8::from_str_radix(hex, 16).map_err(|_| "invalid hex digits".to_string());
            }
            _ => return Err("unknown escape".into()),
        }
    }

    Err("invalid char literal format".into())
}
