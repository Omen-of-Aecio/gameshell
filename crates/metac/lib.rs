//! A zero-allocation, no_std, lisp-inspired command language parser for custom interpreters.
//!
//! Here's an example:
//! ```
//! use metac::{Data, Evaluate};
//! fn main() {
//!     struct Eval { }
//!     impl Evaluate<()> for Eval {
//!         fn evaluate(&mut self, statement: &[Data]) -> () {
//!             for part in statement {
//!                 match part {
//!                     Data::Atom(string) => {}
//!                     Data::Command(command_string) => {}
//!                 }
//!                 println!["{:?}", part];
//!             }
//!         }
//!     }
//!
//!     let mut eval = Eval { };
//!     eval.interpret_single("Hello (World 1 2) 3").unwrap();
//! }
//! ```
//!
//! All you need to do is implement trait `Evaluate` on a structure, then, you call `interpret`
//! on said struct.
//!
//! This allows you to put anything in your struct, you decide how the interpreter is going to
//! work. What this library does for you is parse the input into two things:
//!
//! 1. Atoms - Basically strings
//! 2. Commands - `()`-enclosed text.
//!
//! Note that nested expressions are not expanded by metac, you have to do this yourself.
//! A statement with nesting like `something (alpha (beta gamma))` will be parsed as `[Atom("something"),
//! Command("alpha (beta gamma)")]`.
//! Your evaluator decides whether it will parse the contents or use it for something different.
//!
//! # More interesting example #
//! ```
//! use metac::{Data, Evaluate};
//! use std::collections::HashMap;
//! fn main() {
//!     struct Eval {
//!         hashmap: HashMap<String, String>,
//!     }
//!     impl Eval {
//!         fn register(&mut self, key: &str, value: &str) {
//!             self.hashmap.insert(key.into(), value.into());
//!         }
//!     }
//!     impl Evaluate<String> for Eval {
//!         fn evaluate(&mut self, statement: &[Data]) -> String {
//!             if statement.len() == 2 {
//!                 if let Data::Atom("Get") = statement[0] {
//!                     if let Data::Atom(key) = statement[1] {
//!                         return self.hashmap.get(key).unwrap().clone();
//!                     }
//!                 }
//!             }
//!             "".into()
//!         }
//!     }
//!
//!     let mut eval = Eval { hashmap: HashMap::new() };
//!     eval.register("my-variable", "my-value");
//!     assert_eq!["my-value", eval.interpret_single("Get my-variable").unwrap()];
//! }
//! ```
//! From here we can set up a more complex environment, callbacks, etc. It's all up to the
//! implementer.
//!
//! # Multiline Support #
//! Because this is a sh-like language, it's quite line oriented by nature. Feeding "a b c\nd e f" into
//! the interpreter will separately interpret each line.
//!
//! However, it is sometimes very desirable to write code on multiple lines. The only way to do
//! this in metac is by using parentheses or `interpret_single`:
//! ```
//! use metac::{Data, Evaluate};
//! fn main() {
//!     struct Eval { }
//!     impl Evaluate<usize> for Eval {
//!         fn evaluate(&mut self, statement: &[Data]) -> usize {
//!             statement.len()
//!         }
//!     }
//!
//!     let mut eval = Eval { };
//!
//!     assert_eq![5, eval.interpret_single("This is\na single statement").unwrap()];
//!
//!     // Note: The return value is the result of interpreting the last statement, which is why
//!     // it returns 3 instead of 2 (the first statement) or 5 (the sum).
//!     assert_eq![3, eval.interpret_multiple("Here are\ntwo unrelated statements").unwrap()];
//!     assert_eq![5, eval.interpret_single("Here are\ntwo related statements").unwrap()];
//!
//!     // Because the "\n" was present during an opening parenthesis, both lines are considered
//!     // part of the same statement, hence 5 elements in this statement.
//!     assert_eq![5, eval.interpret_multiple("This is (\na) single statement").unwrap()];
//! }
//! ```
#![feature(test)]
#![no_std]
extern crate test;

use smallvec::SmallVec;

/// A specific-sized small vector
type SVec<A> = SmallVec<[A; 8]>;

/// Distinguishes atoms from commands
#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub enum Data<'a> {
    /// An atom is a single, non-whitespace non-(), connected string of characters
    Atom(&'a str),
    /// A command represents the contents (including whitespace) inside (), excluding the outer
    /// parentheses. It may contain inner ()-characters.
    Command(&'a str),
}

impl<'a> Data<'a> {
    /// Get the raw string data
    pub fn content(&self) -> &'a str {
        match *self {
            Data::Atom(string) => string,
            Data::Command(string) => string,
        }
    }
}

/// Parsing error struct
///
/// The errors represented here are single-line oriented. For instance, a
/// `DanglingLeftParenthesis` refers to an unclosed `(` in the line.
#[derive(Debug, PartialEq)]
pub enum ParseError {
    /// A left parenthesis has been left open
    DanglingLeftParenthesis,
    /// A right parenthesis has been found without having read a corresponding left parenthesis
    PrematureRightParenthesis,
}

/// Interpreter trait
///
/// Central trait to add the interpreter to your custom evaluator
pub trait Evaluate<T: Default> {
    /// Evaluate a single statement
    ///
    /// Statements are line-separated pieces of code turned into fixed data
    /// segments. See `interpret_single` and `interpret_multiple` on how to
    /// parse statements.
    fn evaluate<'a>(&mut self, statement: &[Data<'a>]) -> T;
    /// Set up the parser and call evaluate on the result
    ///
    /// This method expects 1 single statement, that is, it doesn't take in a bunch of
    /// separate statements, but rather one single whole statement, even if it contains
    /// newlines, which are considered whitespace and skipped.
    fn interpret_single(&mut self, statement: &str) -> Result<T, ParseError> {
        let mut data = SVec::<_>::new();
        parse(statement, &mut data)?;
        Ok(self.evaluate(&data[..]))
    }
    /// Interpret several statements one-by-one
    ///
    /// When calling this function, it will `interpret` each individual statement,
    /// Normally, this happens when a newline is found. If however that same line contains
    /// an unclosed opening parenthesis, we will need to include some lines coming after this one
    /// in order to complete the statement.
    fn interpret_multiple(&mut self, code: &str) -> Result<T, ParseError> {
        let mut old_idx = 0;
        let mut lparen_stack = 0;
        let mut result = T::default();
        let mut idx = 0;
        let mut seen_non_ws = false;
        for ch in code.chars() {
            if ch == '\n' && lparen_stack == 0 && seen_non_ws {
                seen_non_ws = false;
                result = self.interpret_single(&code[old_idx..idx])?;
                old_idx = idx + 1;
            } else if ch == '(' {
                lparen_stack += 1;
            } else if ch == ')' {
                if lparen_stack == 0 {
                    return Err(ParseError::PrematureRightParenthesis);
                }
                lparen_stack -= 1;
            } else if !ch.is_whitespace() {
                seen_non_ws = true;
            }
            idx += ch.len_utf8();
        }
        if idx != old_idx && seen_non_ws {
            result = self.interpret_single(&code[old_idx..idx])?;
        }
        Ok(result)
    }
}

/// A partial parse is a parse where we send single bytes into the parser and get back a complete
/// parsing state. This is useful when reading TCP streams or other streams that may yield at any
/// point in time.
///
/// This structure does not in any way do any interpreting or evaluate, it's just used for stream
/// parsing. This structure is quite low-level. See the tests in this file to see how it operates,
/// based on that, you need to add surrounding facilities where you use this struct to handle the
/// outputs it gives you.
#[derive(Debug, Default, PartialEq)]
pub struct PartialParse {
    lparen_stack: usize,
    has_encountered_rparen: bool,
}

/// Description of the parsing state
#[derive(Debug, PartialEq)]
pub enum PartialParseOp {
    Ready,
    Unready,
    Discard,
}

impl PartialParse {
    /// Parses. 1 byte at a time
    ///
    /// This function assumes that a linear stream of bytes is fed into it.
    ///
    /// It will return `PartialParseOp::Discard` when there has been an error. In such cases, _all_ previous bytes ought
    /// to be discarded and not interpreted.
    ///
    /// When it returns `PartialParseOp::Ready`, it means that the previous bytes (except for those that were
    /// marked `PartialParseOp::Discard`) can be sent into `interpret_single` safely.
    ///
    /// When it returns `PartialParseOp::Unready`, it means that the parser simply noticed the character and
    /// advanced its internal state.
    pub fn parse_increment(&mut self, input: u8) -> PartialParseOp {
        if input == b'\n' && self.lparen_stack == 0 {
            self.has_encountered_rparen = false;
            return PartialParseOp::Ready;
        } else if input == b'(' {
            self.lparen_stack += 1;
        } else if input == b')' {
            if self.lparen_stack == 0 {
                self.has_encountered_rparen = true;
                return PartialParseOp::Discard;
            }
            self.lparen_stack -= 1;
        }
        if self.has_encountered_rparen {
            PartialParseOp::Discard
        } else {
            PartialParseOp::Unready
        }
    }
}

// ---

/// Parse an input line into a classified output buffer
fn parse<'a>(line: &'a str, output: &mut SVec<Data<'a>>) -> Result<(), ParseError> {
    let mut lparen_stack = 0;
    let (mut start, mut stop) = (0, 0);
    for ch in line.chars() {
        if lparen_stack > 0 {
            if ch == '(' {
                lparen_stack += 1;
                stop += ch.len_utf8();
            } else if ch == ')' {
                lparen_stack -= 1;
                if lparen_stack == 0 {
                    output.push(Data::Command(&line[start..stop]));
                    stop += ch.len_utf8();
                    start = stop;
                } else {
                    stop += ch.len_utf8();
                }
            } else {
                stop += ch.len_utf8();
            }
        } else if ch.is_whitespace() {
            if start != stop {
                output.push(Data::Atom(&line[start..stop]));
            }
            stop += ch.len_utf8();
            start = stop;
        } else if ch == '(' {
            lparen_stack += 1;
            stop += ch.len_utf8();
            start = stop;
        } else if ch == ')' {
            return Err(ParseError::PrematureRightParenthesis);
        } else {
            stop += ch.len_utf8();
        }
    }
    if lparen_stack > 0 {
        return Err(ParseError::DanglingLeftParenthesis);
    }
    if start != stop {
        output.push(Data::Atom(&line[start..stop]));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::{black_box, Bencher};

    #[test]
    fn empty_parse() {
        let line = "";
        let mut data = SVec::<Data>::new();
        parse(line, &mut data).unwrap();

        assert_eq![true, data.is_empty()];
    }

    #[test]
    fn basic_parse() {
        let line = "Set Log Level 0";
        let mut data = SVec::<Data>::new();
        parse(line, &mut data).unwrap();

        assert_eq![4, data.len()];
        assert_eq![Data::Atom("Set"), data[0]];
        assert_eq![Data::Atom("Log"), data[1]];
        assert_eq![Data::Atom("Level"), data[2]];
        assert_eq![Data::Atom("0"), data[3]];
    }

    #[test]
    fn parse_weird_whitespace() {
        let line = "Set Log\n\n\n Level  ( 0)";
        let mut data = SVec::<Data>::new();
        parse(line, &mut data).unwrap();

        assert_eq![4, data.len()];
        assert_eq![Data::Atom("Set"), data[0]];
        assert_eq![Data::Atom("Log"), data[1]];
        assert_eq![Data::Atom("Level"), data[2]];
        assert_eq![Data::Command(" 0"), data[3]];
    }

    #[test]
    fn parse_unicode() {
        let line = "2";
        let mut data = SVec::<Data>::new();
        parse(line, &mut data).unwrap();

        assert_eq![1, data.len()];
    }

    #[test]
    fn empty_subcommand_parse() {
        let line = "()";
        let mut data = SVec::<Data>::new();
        parse(line, &mut data).unwrap();

        assert_eq![1, data.len()];
        assert_eq![Data::Command(""), data[0]];
    }

    #[test]
    fn empty_nested_subcommand_parse() {
        let line = "(())";
        let mut data = SVec::<Data>::new();
        parse(line, &mut data).unwrap();

        assert_eq![1, data.len()];
        assert_eq![Data::Command("()"), data[0]];
    }

    #[test]
    fn empty_nested_subcommand_with_more_empty_parse() {
        let line = "(())()";
        let mut data = SVec::<Data>::new();
        parse(line, &mut data).unwrap();

        assert_eq![2, data.len()];
        assert_eq![Data::Command("()"), data[0]];
        assert_eq![Data::Command(""), data[1]];
    }

    #[test]
    fn subcommand_parse() {
        let line = "Set Log Level (Get Log Level)";
        let mut data = SVec::<Data>::new();
        parse(line, &mut data).unwrap();

        assert_eq![4, data.len()];
        assert_eq![Data::Atom("Set"), data[0]];
        assert_eq![Data::Atom("Log"), data[1]];
        assert_eq![Data::Atom("Level"), data[2]];
        assert_eq![Data::Command("Get Log Level"), data[3]];
    }

    #[test]
    fn subcommand_parse_multiline() {
        let line = "Set Log Level (\n\tGet Logger Levels\n)";
        let mut data = SVec::<Data>::new();
        parse(line, &mut data).unwrap();

        assert_eq![4, data.len()];
        assert_eq![Data::Atom("Set"), data[0]];
        assert_eq![Data::Atom("Log"), data[1]];
        assert_eq![Data::Atom("Level"), data[2]];
        assert_eq![Data::Command("\n\tGet Logger Levels\n"), data[3]];

        let mut new_data = SVec::<Data>::new();
        parse(data[3].content(), &mut new_data).unwrap();
        assert_eq![3, new_data.len()];
        assert_eq![Data::Atom("Get"), new_data[0]];
        assert_eq![Data::Atom("Logger"), new_data[1]];
        assert_eq![Data::Atom("Levels"), new_data[2]];
    }

    // ---

    #[test]
    fn fail_parse_too_long() {
        let line = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Suspendisse viverra porta lacus, quis pretium nibh lacinia at. Mauris convallis sed lectus nec dapibus. Interdum et malesuada fames ac ante ipsum primis in faucibus. Nulla vulputate sapien dui. Aliquam finibus ante ut purus facilisis, in sagittis tortor varius. Nunc interdum fermentum libero, et egestas arcu convallis sed. Maecenas nec diam a libero vulputate suscipit. Phasellus ac dolor ut nunc ultricies fringilla. Maecenas sed feugiat nunc. Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia Curae. Quisque tincidunt metus ut ante dapibus, et molestie massa varius. Sed ultrices sapien sed mauris congue pretium. Pellentesque bibendum hendrerit sagittis. Vestibulum dignissim egestas feugiat. Ut porttitor et massa a posuere. Ut euismod metus a sem facilisis ullamcorper. Proin pharetra placerat enim";
        let mut data = SVec::<_>::new();
        parse(line, &mut data).unwrap();
    }

    #[test]
    fn fail_parse_closing_parenthesis() {
        let line = "command ) will not work";
        let mut data = SVec::<_>::new();
        assert_eq![
            ParseError::PrematureRightParenthesis,
            parse(line, &mut data).unwrap_err()
        ];
    }

    #[test]
    fn fail_parse_dangling_open_parenthesis() {
        let line = "command ( will not work";
        let mut data = SVec::<_>::new();
        assert_eq![
            ParseError::DanglingLeftParenthesis,
            parse(line, &mut data).unwrap_err()
        ];
    }

    // ---

    #[test]
    fn interpret_empty() {
        struct Eval {
            pub invoked: usize,
        }
        impl Evaluate<()> for Eval {
            fn evaluate<'a>(&mut self, data: &[Data<'a>]) {
                assert_eq![0, data.len()];
                self.invoked += 1;
            }
        }
        let mut eval = Eval { invoked: 0 };

        let line = "";
        eval.interpret_single(line).unwrap();
        assert_eq![1, eval.invoked];
        eval.interpret_multiple(line).unwrap();
        assert_eq![1, eval.invoked];
    }

    #[test]
    fn interpret_whitespace() {
        struct Eval {
            pub invoked: usize,
        }
        impl Evaluate<()> for Eval {
            fn evaluate<'a>(&mut self, data: &[Data<'a>]) {
                assert_eq![0, data.len()];
                self.invoked += 1;
            }
        }
        let mut eval = Eval { invoked: 0 };

        let line = " ";
        eval.interpret_single(line).unwrap();
        assert_eq![1, eval.invoked];
        eval.interpret_multiple(line).unwrap();
        assert_eq![1, eval.invoked];
        eval.interpret_multiple(" \n").unwrap();
        assert_eq![1, eval.invoked];
    }

    #[test]
    fn interpret_unicode() {
        struct Eval {
            pub invoked: usize,
        }
        impl Evaluate<()> for Eval {
            fn evaluate<'a>(&mut self, _: &[Data<'a>]) {
                self.invoked += 1;
            }
        }
        let mut eval = Eval { invoked: 0 };

        let line = "2";
        eval.interpret_single(line).unwrap();
        eval.interpret_multiple(line).unwrap();
        assert_eq![2, eval.invoked];
    }

    #[test]
    fn interpret_multiple_simple() {
        struct Eval {
            pub invoked: usize,
        }
        impl Evaluate<()> for Eval {
            fn evaluate<'a>(&mut self, _: &[Data<'a>]) {
                self.invoked += 1;
            }
        }
        let mut eval = Eval { invoked: 0 };

        eval.interpret_multiple("X\nY\nZ\nW (\n\t1 2 3\n) W-1\nQ")
            .unwrap();
        assert_eq![5, eval.invoked];
    }

    #[test]
    fn interpret_multiple() {
        struct Eval {
            pub invoked: usize,
        }
        impl Evaluate<()> for Eval {
            fn evaluate<'a>(&mut self, commands: &[Data<'a>]) {
                self.invoked += 1;
                match self.invoked {
                    1 => {
                        assert_eq![
                            &[
                                Data::Atom("Lorem"),
                                Data::Atom("ipsum"),
                                Data::Command("\n\tdolor sit amet\n\tX\n")
                            ],
                            commands
                        ];
                    }
                    2 => {
                        assert_eq![
                            &[
                                Data::Atom("dolor"),
                                Data::Atom("sit"),
                                Data::Atom("amet"),
                                Data::Atom("X")
                            ],
                            commands
                        ];
                    }
                    3 => {
                        assert_eq![&[Data::Atom("Singular")], commands];
                    }
                    _ => assert![false],
                }
                for command in commands {
                    match command {
                        Data::Atom(_) => {}
                        Data::Command(string) => {
                            self.interpret_single(string).unwrap();
                        }
                    }
                }
            }
        }
        let mut eval = Eval { invoked: 0 };

        eval.interpret_multiple("Lorem ipsum (\n\tdolor sit amet\n\tX\n)\nSingular")
            .unwrap();
        assert_eq![3, eval.invoked];
    }

    #[test]
    fn evaluator() {
        struct Eval {
            pub invoked: usize,
        }
        impl Evaluate<()> for Eval {
            fn evaluate<'a>(&mut self, _: &[Data<'a>]) {
                self.invoked += 1;
            }
        }
        let mut eval = Eval { invoked: 0 };
        eval.interpret_single("Hello World").unwrap();
        assert_eq![1, eval.invoked];
        eval.interpret_single("This is an example (command)")
            .unwrap();
        assert_eq![2, eval.invoked];
    }

    #[test]
    fn recursive_evaluator() {
        struct Eval {
            pub invoked: usize,
        }
        impl Evaluate<()> for Eval {
            fn evaluate<'a>(&mut self, commands: &[Data<'a>]) {
                self.invoked += 1;
                for command in commands {
                    match command {
                        Data::Atom(_) => {}
                        Data::Command(string) => {
                            self.interpret_single(string).unwrap();
                        }
                    }
                }
            }
        }
        let mut eval = Eval { invoked: 0 };
        eval.interpret_single("Hello World").unwrap();
        assert_eq![1, eval.invoked];
        eval.interpret_single("This is an example of substitution: (command)")
            .unwrap();
        assert_eq![3, eval.invoked];
        eval.interpret_single(
            "We can substitute more than once: (my command), anywhere: (another command here)",
        )
        .unwrap();
        assert_eq![6, eval.invoked];
        eval.interpret_single("We can also nest substitutions: (my (recursive (command) here))")
            .unwrap();
        assert_eq![10, eval.invoked];
        eval.interpret_single("a (\n\tb c\n)").unwrap();
        assert_eq![12, eval.invoked];
    }

    // ---

    #[test]
    fn partial_parse_single_line() {
        let mut part = PartialParse::default();
        for ch in "hello world".bytes() {
            assert_eq![PartialParseOp::Unready, part.parse_increment(ch)];
        }
        assert_eq![PartialParseOp::Ready, part.parse_increment(b'\n')];
    }

    #[test]
    fn partial_parse_multi_line() {
        let mut part = PartialParse::default();
        for ch in "hello world (".bytes() {
            assert_eq![PartialParseOp::Unready, part.parse_increment(ch)];
        }
        assert_eq![PartialParseOp::Unready, part.parse_increment(b'\n')];

        for ch in "this is a message".bytes() {
            assert_eq![PartialParseOp::Unready, part.parse_increment(ch)];
        }
        assert_eq![PartialParseOp::Unready, part.parse_increment(b'\n')];
        assert_eq![PartialParseOp::Unready, part.parse_increment(b')')];

        for ch in "last few words".bytes() {
            assert_eq![PartialParseOp::Unready, part.parse_increment(ch)];
        }
        assert_eq![PartialParseOp::Ready, part.parse_increment(b'\n')];
    }

    #[test]
    fn partial_parse_multi_line_nested() {
        let mut part = PartialParse::default();
        for ch in "hello world (".bytes() {
            assert_eq![PartialParseOp::Unready, part.parse_increment(ch)];
        }
        assert_eq![PartialParseOp::Unready, part.parse_increment(b'\n')];

        for ch in "this) (is (a message".bytes() {
            assert_eq![PartialParseOp::Unready, part.parse_increment(ch)];
        }
        assert_eq![PartialParseOp::Unready, part.parse_increment(b'\n')];
        assert_eq![PartialParseOp::Unready, part.parse_increment(b')')];

        for ch in "last few words".bytes() {
            assert_eq![PartialParseOp::Unready, part.parse_increment(ch)];
        }
        assert_eq![PartialParseOp::Unready, part.parse_increment(b'\n')];
        assert_eq![PartialParseOp::Unready, part.parse_increment(b')')];
        assert_eq![PartialParseOp::Ready, part.parse_increment(b'\n')];
    }

    #[test]
    fn partial_parse_error() {
        let mut part = PartialParse::default();
        for ch in "hello world".bytes() {
            assert_eq![PartialParseOp::Unready, part.parse_increment(ch)];
        }
        assert_eq![PartialParseOp::Discard, part.parse_increment(b')')];
        assert_eq![PartialParseOp::Ready, part.parse_increment(b'\n')];
    }

    #[test]
    fn partial_parse_error_complex() {
        let mut part = PartialParse::default();
        for ch in "hello world (\na b c) d ".bytes() {
            assert_eq![PartialParseOp::Unready, part.parse_increment(ch)];
        }
        assert_eq![PartialParseOp::Discard, part.parse_increment(b')')];
        for ch in "opener (\na b c d\ne f".bytes() {
            assert_eq![PartialParseOp::Discard, part.parse_increment(ch)];
        }
        assert_eq![PartialParseOp::Discard, part.parse_increment(b')')];
        assert_eq![PartialParseOp::Ready, part.parse_increment(b'\n')];
    }

    #[test]
    fn premature_right_parentheses_discards_entire_line() {
        let mut part = PartialParse::default();
        for ch in "hello world (\na b c) d ".bytes() {
            assert_eq![PartialParseOp::Unready, part.parse_increment(ch)];
        }
        assert_eq![PartialParseOp::Discard, part.parse_increment(b')')];
        for ch in "opener (\na b c d\ne f".bytes() {
            assert_eq![PartialParseOp::Discard, part.parse_increment(ch)];
        }
        assert_eq![PartialParseOp::Discard, part.parse_increment(b'\n')];
        assert_eq![PartialParseOp::Discard, part.parse_increment(b'a')];
        assert_eq![PartialParseOp::Discard, part.parse_increment(b'(')];
        assert_eq![PartialParseOp::Discard, part.parse_increment(b'\n')];
        assert_eq![PartialParseOp::Discard, part.parse_increment(b'x')];
        assert_eq![PartialParseOp::Discard, part.parse_increment(b'\n')];
        assert_eq![PartialParseOp::Discard, part.parse_increment(b'd')];
        assert_eq![PartialParseOp::Discard, part.parse_increment(b')')];
        assert_eq![PartialParseOp::Discard, part.parse_increment(b')')];
        assert_eq![PartialParseOp::Ready, part.parse_increment(b'\n')];
        assert_eq![PartialParseOp::Unready, part.parse_increment(b'x')];
    }

    // ---

    #[bench]
    fn empty_evaluate(b: &mut Bencher) {
        struct Eval {}
        impl Evaluate<()> for Eval {
            fn evaluate<'a>(&mut self, _: &[Data<'a>]) {}
        }
        let mut eval = Eval {};
        b.iter(|| {
            eval.interpret_single(black_box("unknown reasonably long command"))
                .unwrap();
        });
    }

    #[bench]
    fn empty_evaluate_very_short(b: &mut Bencher) {
        struct Eval {}
        impl Evaluate<()> for Eval {
            fn evaluate<'a>(&mut self, _: &[Data<'a>]) {}
        }
        let mut eval = Eval {};
        b.iter(|| {
            eval.interpret_single(black_box("x")).unwrap();
        });
    }

    #[bench]
    fn empty_evaluate_very_long(b: &mut Bencher) {
        struct Eval {}
        impl Evaluate<()> for Eval {
            fn evaluate<'a>(&mut self, _: &[Data<'a>]) {}
        }
        let mut eval = Eval {};
        b.iter(|| {
            eval.interpret_single(black_box("Lorem ipsum dolor sit amet, consectetur adipiscing elit. Mauris tristique massa magna, eget consectetur dui posuere congue. Etiam rhoncus porttitor enim, eget malesuada ante dapibus eget. Duis neque dui, tincidunt ut varius")).unwrap();
        });
    }

    #[bench]
    fn empty_evaluate_with_subsistution(b: &mut Bencher) {
        struct Eval {}
        impl Evaluate<()> for Eval {
            fn evaluate<'a>(&mut self, _: &[Data<'a>]) {}
        }
        let mut eval = Eval {};
        b.iter(|| {
            eval.interpret_single(black_box("unknown (some) (long command 1)"))
                .unwrap();
        });
    }

    #[bench]
    fn increment_evaluate(b: &mut Bencher) {
        struct Eval {
            pub invoke: usize,
        }
        impl Evaluate<()> for Eval {
            fn evaluate<'a>(&mut self, _: &[Data<'a>]) {
                self.invoke += 1;
            }
        }
        let mut eval = Eval { invoke: 0 };
        b.iter(|| {
            eval.interpret_single(black_box("unknown reasonably long command"))
                .unwrap();
        });
    }

    #[bench]
    fn parse_very_long(b: &mut Bencher) {
        let line = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Mauris tristique massa magna, eget consectetur dui posuere congue. Etiam rhoncus porttitor enim, eget malesuada ante dapibus eget. Duis neque dui, tincidunt ut varius";
        b.iter(|| {
            let mut data = SVec::<_>::new();
            parse(black_box(line), &mut data).unwrap();
        });
    }

    #[bench]
    fn iterate_very_long(b: &mut Bencher) {
        let line = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Mauris tristique massa magna, eget consectetur dui posuere congue. Etiam rhoncus porttitor enim, eget malesuada ante dapibus eget. Duis neque dui, tincidunt ut varius";
        b.iter(|| {
            let mut count = 0;
            for _ in black_box(line).chars() {
                count += 1;
            }
            assert_eq![223, count];
        });
    }
}
