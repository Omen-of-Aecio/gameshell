//! Incremental consumer (parser) of bytes
//!
//! The purpose of this module is to facilitate parsing of streams of unknown size and timing, basically:
//!
//! `&[u8] -> &[u8]`
//!
//! Where the latter `&[u8]` is something we believe to be valid.
//!
//! The incoming bytes are buffered into an array and each byte is passed through a user-defined
//! parser. If the parser decides that the input is invalid, all previous bytes will be
//! discarded. If the input is not yet finished, we wait for the next byte(s) to come.
//! If the input has been approved, we call `process` with the slice of the approved bytes.

/// Describe how many bytes are consumed. Stop will stop the consumption of bytes.
#[derive(Debug)]
pub enum Consumption {
    /// Indicate that we have written `usize` bytes to the buffer
    Consumed(usize),
    /// Stop the system
    Stop,
}

/// Indication of the validity of the current accumulation buffer
#[derive(Debug)]
pub enum Validation {
    /// Returning this value causes `process` to run with the accumulated buffer. The buffer is
    /// reset after `process` has run
    Ready,
    /// Continue consuming bytes
    Unready,
    /// Discard the current accumulation buffer
    Discard,
    /// Stop the system
    Stop,
}

/// Indicate whether to continue or stop the system
#[derive(Debug)]
pub enum Process {
    /// Continue processing bytes
    Continue,
    /// Stop the system
    Stop,
}

/// Incremental consumer of bytes
///
/// Consume bytes until a complete set of bytes has been found, then, run a handler
/// function on just that set of bytes.
///
/// This is used for accepting bytes from some external stream, note that we set a maximum
/// size on the buffer, so no external input can cause excessive memory usage.
pub trait IncConsumer {
    /// Consume bytes and place them on an output stack
    fn consume(&mut self, output: &mut [u8]) -> Consumption;

    /// Validate part of the bytestream, as soon as we return `Validation::Ready`, `process`
    /// will be run on the current accumulated bytes, after which these bytes will be deleted.
    fn validate(&mut self, output: u8) -> Validation;

    /// Process do actual stuff with the bytes to affect the system.
    ///
    /// The sequence of bytes input here will have been verified by the `validate`
    /// function.
    fn process(&mut self, input: &[u8]) -> Process;

    /// Runs the incremental consumer until it is signalled to quit
    fn run(&mut self, buf: &mut [u8]) {
        let mut begin = 0;
        let mut shift = 0;
        loop {
            for idx in shift..begin {
                buf[idx - shift] = buf[idx];
            }
            begin -= shift;
            shift = 0;
            match self.consume(&mut buf[begin..]) {
                Consumption::Consumed(amount) => {
                    for ch in buf[begin..(begin + amount)].iter() {
                        begin += 1;
                        match self.validate(*ch) {
                            Validation::Discard => shift = begin,
                            Validation::Ready => {
                                match self.process(&buf[shift..begin]) {
                                    Process::Continue => {}
                                    Process::Stop => return,
                                }
                                shift = begin;
                            }
                            Validation::Stop => return,
                            Validation::Unready => {}
                        }
                    }
                }
                Consumption::Stop => return,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        #[derive(Default)]
        struct Consumer {
            pub consumption_count: usize,
            pub validation_count: usize,
            pub process_count: usize,
        }

        impl IncConsumer for Consumer {
            fn consume(&mut self, _: &mut [u8]) -> Consumption {
                self.consumption_count += 1;
                Consumption::Stop
            }
            fn validate(&mut self, _: u8) -> Validation {
                self.validation_count += 1;
                Validation::Ready
            }
            fn process(&mut self, _: &[u8]) -> Process {
                self.process_count += 1;
                Process::Continue
            }
        }

        let mut consumed = Consumer::default();
        let buffer = &mut [0u8; 0];
        consumed.run(buffer);

        assert_eq![1, consumed.consumption_count];
        assert_eq![0, consumed.validation_count];
        assert_eq![0, consumed.process_count];
    }

    #[test]
    fn stop_at_validation() {
        #[derive(Default)]
        struct Consumer {
            pub consumption_count: usize,
            pub validation_count: usize,
            pub process_count: usize,
        }

        impl IncConsumer for Consumer {
            fn consume(&mut self, _: &mut [u8]) -> Consumption {
                self.consumption_count += 1;
                Consumption::Consumed(1)
            }
            fn validate(&mut self, _: u8) -> Validation {
                self.validation_count += 1;
                Validation::Stop
            }
            fn process(&mut self, _: &[u8]) -> Process {
                self.process_count += 1;
                Process::Continue
            }
        }

        let mut consumed = Consumer::default();
        let buffer = &mut [0u8; 1];
        consumed.run(buffer);

        assert_eq![1, consumed.consumption_count];
        assert_eq![1, consumed.validation_count];
        assert_eq![0, consumed.process_count];
    }

    #[quickcheck_macros::quickcheck]
    fn consuming_bytes_within_bounds_never_fails(bytes: u16) {
        #[derive(Default)]
        struct Consumer {
            bytes: usize,
            seen: usize,
        }

        impl IncConsumer for Consumer {
            fn consume(&mut self, _: &mut [u8]) -> Consumption {
                if self.bytes == 0 {
                    Consumption::Stop
                } else {
                    Consumption::Consumed(self.bytes)
                }
            }
            fn validate(&mut self, _: u8) -> Validation {
                self.seen += 1;
                if self.seen == self.bytes {
                    Validation::Ready
                } else {
                    Validation::Unready
                }
            }
            fn process(&mut self, input: &[u8]) -> Process {
                assert_eq![self.bytes, input.len()];
                Process::Stop
            }
        }

        let mut consumed = Consumer {
            bytes: bytes as usize,
            seen: 0,
        };
        let mut buffer = vec![0u8; bytes as usize];
        consumed.run(&mut buffer);
    }
}
