/// Basic types used internally by the gameshell
#[derive(Clone, Debug)]
pub enum Type {
    Atom(String),
    Bool(bool),
    Command(String),
    F32(f32),
    I32(i32),
    String(String),
    U8(u8),
}
