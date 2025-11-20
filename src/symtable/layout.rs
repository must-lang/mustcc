#[derive(Debug, Clone)]
pub struct Layout {
    pub size: u32,
    pub align: u32,
    pub kind: LayoutKind,
}

#[derive(Debug, Clone)]
pub enum Type {
    Tu8,
    Tu16,
    Tu32,
    Tu64,
    Tusize,
    Ti8,
    Ti16,
    Ti32,
    Ti64,
    Tisize,
}

#[derive(Debug, Clone)]
pub enum LayoutKind {
    Primitive(Type),
    Struct(Vec<(Layout, i32)>),
    Union(Vec<Layout>),
}

impl Layout {
    pub(crate) fn require_stack(&self) -> bool {
        match &self.kind {
            LayoutKind::Primitive(_) => false,
            LayoutKind::Struct(items) => true,
            LayoutKind::Union(layouts) => true,
        }
    }
}
