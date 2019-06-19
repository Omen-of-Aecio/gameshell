//! Basic types used by the gameshell for input to handlers
/// Basic types used by the gameshell for input to handlers
#[derive(Clone, Debug)]
pub enum Type {
    /// A string that contains no whitespace
    Atom(String),
    /// A `true` or `false` value
    Bool(bool),
    /// A string which was enclosed by parentheses, may contain parentheses itself
    Command(String),
    /// A 32-bit floating point value
    F32(f32),
    /// A 32-bit signed integer value
    I32(i32),
    /// A string, can be created using (#)
    String(String),
    /// An unsigned 8-bit value
    U8(u8),
}
