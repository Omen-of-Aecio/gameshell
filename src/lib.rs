//! GameShell - A fast and lightweight shell for interactive work in Rust.
//!
//! GameShell is a little lisp-like command shell made to be embedded in rust programs. It has no
//! runtime and attempts to call given handlers as fast as possible. This means that GameShell is
//! made for one-time commands, where the heavy lifting is done in your specified handler
//! functions. It does not do any JIT/compilation or bytecode conversion, but goes straight to a
//! handler and calls it.
//!
//! # Language #
//!
//! The language is just
//! ```ignore
//! command argument (subcommand argument ...) (#literal string inside here) argument ...
//! ```
//!
//! If an opened parenthesis is not closed on a newline, the next line is also considered part of
//! the command:
//! ```ignore
//! command (
//!     subcommand
//!     argument
//!     ...
//! ) argument ...
//! ```
//!
//! # Example #
//!
//! This example sets up a basic interpreter and a single handler for a single command. More
//! commands and handlers can be added.
//!
//! ```
//! use gameshell::{
//!     predicates::*, types::Type, Evaluator, GameShell, IncConsumer,
//! };
//! use std::str::from_utf8;
//!
//! // This is the input stream, GameShell will handle anything that implements
//! // std::io::Read, and will efficiently acquire data from such a stream.
//! let read = b"Lorem ipsum 1.23\n";
//! // This is the output stream, GameShell will use this to write messages out.
//! let mut write = [0u8; 18];
//!
//! // A gameshell is created by supplying a context object (here 0u8), and an IO stream.
//! let mut eval = GameShell::new(0u8, &read[..], &mut write[..]);
//!
//! // We then register command handler functions to GameShell, such that it can run commands
//! // when reading from the input stream
//! //
//! // Each command handler takes a `&mut Context` (here u8), and a list of arguments.
//! // It returns a result, `Ok` indicating a successful computation,
//! // and an `Err` indicating an error, aborting any nested computation and writing out the
//! // error message to the writer.
//! fn handler(context: &mut u8, args: &[Type]) -> Result<String, String> {
//!     *context += 1;
//!     println!("Got types: {:?}", args[0]);
//!     Ok("Hello world!".into())
//! }
//!
//! // Register the handler and associate it with the command "Lorem ipsum <f32>".
//! // The first element in the pair is a string literal on which we match the command tree,
//! // and the second argument is an arbitrary `Decider` which parses our input data into a
//! // `Type`. Deciders can consume as many elements as they like.
//! eval.register((&[("Lorem", None), ("ipsum", ANY_F32)], handler)).unwrap();
//!
//! // Run the command loop, keeps reading from the tcp buffer until the buffer has no more
//! // elements. When reading from a `TcpStream` this call will block until the stream is
//! // closed. The `buffer` provided here is the buffer for parsing a single incoming whole
//! // command. If the command exceeds this size, the command will be discarded and the
//! // connection severed.
//! let buffer = &mut [0u8; 1024];
//! eval.run(buffer);
//!
//! // Ensure that we have run at least once, our starting context was 0, which should now be 1
//! assert_eq!(1, *eval.context());
//! // Our Ok message has been written to the writer
//! assert_eq!("Ok(\"Hello world!\")", from_utf8(&write[..]).unwrap());
//! ```
//!
//! # Why does a command handler return a string instead of a type? #
//!
//! Because nested commands may reinterpret the string in any way they like, according to an
//! arbitrary decider. Thus, we can't return types. This is an inefficiency to allow for proper
//! error checking in nested calls.
//!
//! As unfortunate as that is, this library is not meant to sacrifice usability for speed. If you
//! want speed, you can just collapse two commands into one and use a single handler, since pure
//! Rust will always beat this library at speed.
//!
//! # Builtin commands #
//!
//! GameShell has 2 builtin commands:
//! ```ignore
//! ?
//! ```
//! List all registered commands and their potential arguments. An argument to this command will
//! regex filter the output: `? lorem`.
//! and
//! ```ignore
//! autocomplete
//! ```
//! Autocomplete a query.
//!
//! These commands return strings that contain useful information to be displayed to the user. If
//! you do not wish to expose these commands then you overwrite these commands using a command
//! handler.
#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications
)]
pub use crate::{evaluator::Evaluator, incconsumer::IncConsumer};
use crate::{
    incconsumer::{Consumption, Process, Validation},
    types::Type,
};
use cmdmat::RegError;
pub use cmdmat::{self, Spec};
pub use metac::{Evaluate, PartialParse, PartialParseOp};
use std::{
    io::{Read, Write},
    str::from_utf8,
};

#[cfg(feature = "with-tokio")]
mod applicator;
pub mod evaluator;
mod incconsumer;
pub mod predicates;
pub mod types;

#[cfg(feature = "with-tokio")]
pub use applicator::tokio_apply;

/// Feedback provided by the interpreter. All results are either a success string or an error
/// string.
/// Errors will abort any nested expressions and return the error immediately.
pub type Feedback = Result<String, String>;

/// The main virtual machine wrapper for a game shell
///
/// This wrapper consumes an input and output stream through which it writes messages.
pub struct GameShell<'a, C, R: Read, W: Write> {
    evaluator: Evaluator<'a, C>,
    parser: PartialParse,
    reader: R,
    writer: W,
}

impl<'a, C, R: Read, W: Write> GameShell<'a, C, R, W> {
    /// Spawn a new gameshell with a given context and readers/writers.
    pub fn new(context: C, reader: R, writer: W) -> Self {
        Self {
            evaluator: Evaluator::new(context),
            parser: PartialParse::default(),
            reader,
            writer,
        }
    }

    /// Get a reference to the current evaluator.
    pub fn evaluator(&mut self) -> &mut Evaluator<'a, C> {
        &mut self.evaluator
    }

    /// Get a reference to the current context.
    pub fn context(&self) -> &C {
        self.evaluator.context()
    }

    /// Get a mutable reference to the current context.
    pub fn context_mut(&mut self) -> &mut C {
        self.evaluator.context_mut()
    }

    /// Register a command specificator to this gameshell instance.
    pub fn register(&mut self, spec: Spec<'_, 'a, Type, String, C>) -> Result<(), RegError> {
        self.evaluator.register(spec)
    }

    /// Register multiple command specifications to this gameshell instance.
    pub fn register_many(
        &mut self,
        spec: &[Spec<'_, 'a, Type, String, C>],
    ) -> Result<(), RegError> {
        self.evaluator.register_many(spec)
    }
}

impl<'a, C, R: Read, W: Write> IncConsumer for GameShell<'a, C, R, W> {
    fn consume(&mut self, output: &mut [u8]) -> Consumption {
        if output.is_empty() {
            let _ = self
                .writer
                .write(b"DecodeError(\"Internal buffer is full, disconnecting\")");
            return Consumption::Stop;
        }
        match self.reader.read(output) {
            Ok(0) => Consumption::Stop,
            Ok(count) => Consumption::Consumed(count),
            Err(_) => Consumption::Stop,
        }
    }
    fn validate(&mut self, input: u8) -> Validation {
        match self.parser.parse_increment(input) {
            PartialParseOp::Ready => Validation::Ready,
            PartialParseOp::Unready => Validation::Unready,
            PartialParseOp::Discard => Validation::Discard,
        }
    }
    fn process(&mut self, input: &[u8]) -> Process {
        let string = from_utf8(input);
        if let Ok(string) = string {
            let result = self.evaluator.interpret_single(string);
            match result {
                Ok(result) => {
                    match result {
                        Feedback::Ok(res) => {
                            if self
                                .writer
                                .write_all(format!("Ok({:?})", res).as_bytes())
                                .is_err()
                            {
                                return Process::Stop;
                            }
                        }
                        Feedback::Err(res) => {
                            if self
                                .writer
                                .write_all(format!("Err({:?})", res).as_bytes())
                                .is_err()
                            {
                                return Process::Stop;
                            }
                        }
                    }
                    if self.writer.flush().is_err() {
                        return Process::Stop;
                    }
                }
                Err(parse_error) => {
                    if self
                        .writer
                        .write_all(
                            format!("ParseError(\"Unable to parse input: {:?}\")", parse_error)
                                .as_bytes(),
                        )
                        .is_err()
                    {
                        return Process::Stop;
                    }
                    if self.writer.flush().is_err() {
                        return Process::Stop;
                    }
                }
            }
            Process::Continue
        } else {
            if self
                .writer
                .write_all(b"DecodeError(\"Received invalid UTF-8 input, disconnecting\")")
                .is_err()
            {
                return Process::Stop;
            }
            if self.writer.flush().is_err() {
                return Process::Stop;
            }
            Process::Stop
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicates::*;

    #[test]
    fn basic_case() {
        let read = b"Lorem ipsum";
        let mut write = [0u8; 10];
        let mut shell = GameShell::new(0u8, &read[..], &mut write[..]);

        assert_eq!(&mut 0u8, shell.context_mut());
    }

    #[test]
    fn basic_byte_stream() {
        let read = b"call 1.23\n";
        let mut write = [0u8; 10];

        let mut eval = GameShell::new(0u8, &read[..], &mut write[..]);

        fn handler(context: &mut u8, _args: &[Type]) -> Result<String, String> {
            *context += 1;
            Ok("".into())
        }

        eval.register((&[("call", ANY_F32)], handler)).unwrap();

        let buffer = &mut [0u8; 1024];
        eval.run(buffer);

        assert_eq!(1, *eval.context());
    }

    #[test]
    fn discard_stream_first_command_too_big() {
        let read = b"call 1.2\ncall 3.1\ncall 99.9999\ncall 1.0";
        let mut write = [0u8; 1024];

        let mut eval = GameShell::new(0u8, &read[..], &mut write[..]);

        fn handler(context: &mut u8, _args: &[Type]) -> Result<String, String> {
            *context += 1;
            Ok("".into())
        }

        eval.register((&[("call", ANY_F32)], handler)).unwrap();

        let buffer = &mut [0u8; 10];
        eval.run(buffer);

        assert_eq!(2, *eval.context());

        let index = write
            .iter()
            .enumerate()
            .find(|(_, &byte)| byte == b'\0')
            .map(|(idx, _)| idx)
            .unwrap();
        assert_eq!(
            "Ok(\"\")Ok(\"\")DecodeError(\"Internal buffer is full, disconnecting\")",
            from_utf8(&write[0..index]).unwrap()
        );
    }

    #[test]
    fn partial_read_succeeds() {
        let read = b"call 1.2\nrock 3.1\n";
        let mut write = [0u8; 1024];

        let mut eval = GameShell::new(0f32, &read[..], &mut write[..]);

        fn handler(context: &mut f32, args: &[Type]) -> Result<String, String> {
            match args[0] {
                Type::F32(number) => *context += number,
                _ => panic!(),
            }
            Ok("".into())
        }

        eval.register((&[("call", ANY_F32)], handler)).unwrap();
        eval.register((&[("rock", ANY_F32)], handler)).unwrap();

        let buffer = &mut [0u8; 12];
        eval.run(buffer);

        assert_eq!(4.3, *eval.context());
    }
}
