use crate::{
    decision::Decision,
    incconsumer::{Consumption, Process, Validation},
    types::Type,
};
pub use crate::{evaluator::Evaluator, feedback::Feedback, incconsumer::IncConsumer};
use cmdmat::{RegError, Spec};
use metac::{Evaluate, PartialParse, PartialParseOp};
use std::{
    io::{Read, Write},
    str::from_utf8,
};

pub mod decision;
pub mod evaluator;
mod feedback;
mod incconsumer;
pub mod predicates;
pub mod types;

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
    /// Spawn a new gameshell with a given context and readers/writers
    pub fn new(context: C, reader: R, writer: W) -> Self {
        Self {
            evaluator: Evaluator::new(context),
            parser: PartialParse::default(),
            reader,
            writer,
        }
    }

    /// Get a reference to the current evaluator
    pub fn evaluator(&mut self) -> &mut Evaluator<'a, C> {
        &mut self.evaluator
    }

    /// Get a reference to the current context
    pub fn context(&self) -> &C {
        self.evaluator.context()
    }

    /// Register a command specificator to this gameshell instance
    pub fn register(&mut self, spec: Spec<'_, 'a, Type, Decision, C>) -> Result<(), RegError> {
        self.evaluator.register(spec)
    }

    /// Register multiple command specifications to this gameshell instance
    pub fn register_many(
        &mut self,
        spec: &[Spec<'_, 'a, Type, Decision, C>],
    ) -> Result<(), RegError> {
        self.evaluator.register_many(spec)
    }
}

impl<'a, C, R: Read, W: Write> IncConsumer for GameShell<'a, C, R, W> {
    fn consume(&mut self, output: &mut [u8]) -> Consumption {
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
            if let Ok(result) = result {
                match result {
                    Feedback::Ok(res) => {
                        if !res.is_empty() {
                            if self.writer.write_all(res.as_bytes()).is_err() {
                                return Process::Stop;
                            }
                        } else if self.writer.write_all(b"Ok").is_err() {
                            return Process::Stop;
                        }
                    }
                    Feedback::Err(res) => {
                        if self
                            .writer
                            .write_all(format!["Err: {}", res].as_bytes())
                            .is_err()
                        {
                            return Process::Stop;
                        }
                    }
                    Feedback::Help(res) => {
                        if !res.is_empty() {
                            if self.writer.write_all(res.as_bytes()).is_err() {
                                return Process::Stop;
                            }
                        } else if self.writer.write_all(b"Empty help message").is_err() {
                            return Process::Stop;
                        }
                    }
                }
                if self.writer.flush().is_err() {
                    return Process::Stop;
                }
            } else {
                if self
                    .writer
                    .write_all(b"Unable to complete query (parse error)")
                    .is_err()
                {
                    return Process::Stop;
                }
                if self.writer.flush().is_err() {
                    return Process::Stop;
                }
            }
            Process::Continue
        } else {
            Process::Stop
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicates::*;

    #[test]
    fn rampage() {
        let read = b"Lorem ipsum";
        let mut write = [0u8; 10];
        GameShell::new(0u8, &read[..], &mut write[..]);
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

        eval.run(1024);

        assert_eq![1, *eval.context()];
    }
}
