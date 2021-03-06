use crate::{Evaluator, Feedback, PartialParse, PartialParseOp};
use metac::Evaluate;
use slog::{error, info, warn, Logger};
use std::str::from_utf8;
use tokio::{net::TcpStream, prelude::*};

/// Apply an evaluator to a tokio TcpStream. Uses an internal buffer efficiently (of 1024 bytes) to
/// store incoming data using the gsh protocol.
pub async fn tokio_apply<C>(mut evaluator: Evaluator<'_, C>, mut stream: TcpStream, log: Logger) {
    let mut buf = [0u8; 1024];
    let mut begin = 0;
    let mut shift = 0;
    let mut parser = PartialParse::default();

    loop {
        for idx in shift..begin {
            buf[idx - shift] = buf[idx];
        }
        begin -= shift;
        shift = 0;

        if begin == buf.len() {
            warn!(log, "Internal buffer is full, disconnecting");
            let _ = stream
                .write_all(b"DecodeError(\"Internal buffer is full, disconnecting\")")
                .await;
            return;
        }

        let amount = match stream.read(&mut buf[begin..]).await {
            Ok(n) if n == 0 => {
                info!(log, "Remote gameshell has disconnected");
                return;
            }
            Ok(n) => n,
            Err(err) => {
                error!(log, "An error occurred while reading from the stream"; "error" => ?err);
                return;
            }
        };

        for ch in buf[begin..(begin + amount)].iter() {
            begin += 1;
            match parser.parse_increment(*ch) {
                PartialParseOp::Discard => {
                    shift = begin;
                }
                PartialParseOp::Ready => {
                    let string = from_utf8(&buf[shift..begin]);
                    if let Ok(string) = string {
                        info!(log, "Got input"; "string" => string);
                        let result = evaluator.interpret_single(string);
                        match result {
                            Ok(result) => {
                                match result {
                                    Feedback::Ok(res) => {
                                        let result = format!("Ok({:?})", res);
                                        if stream.write_all(result.as_bytes()).await.is_err() {
                                            return;
                                        }
                                    }
                                    Feedback::Err(res) => {
                                        let string = format!("Err({:?})", res);
                                        if stream.write_all(string.as_bytes()).await.is_err() {
                                            return;
                                        }
                                    }
                                }
                                if stream.flush().await.is_err() {
                                    return;
                                }
                            }
                            Err(parse_error) => {
                                if stream
                                    .write_all(
                                        format!(
                                            "ParseError(\"Unable to parse input: {:?}\")",
                                            parse_error
                                        )
                                        .as_bytes(),
                                    )
                                    .await
                                    .is_err()
                                {
                                    return;
                                }
                                if stream.flush().await.is_err() {
                                    return;
                                }
                            }
                        }
                    } else {
                        if stream
                            .write_all(
                                b"DecodeError(\"Received invalid UTF-8 input, disconnecting\")",
                            )
                            .await
                            .is_err()
                        {
                            return;
                        }
                        if stream.flush().await.is_err() {
                            return;
                        }
                        return;
                    }
                    shift = begin;
                }
                PartialParseOp::Unready => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::tokio_apply;
    use crate::{
        cmdmat::{Decider, Decision, SVec},
        types::Type,
        Evaluator,
    };
    use slog::{o, Discard, Logger};
    use std::{
        net::{Ipv4Addr, SocketAddrV4},
        str::from_utf8,
    };
    use tokio::{
        net::{TcpListener, TcpStream},
        prelude::*,
        runtime::Builder,
    };

    async fn io_assert(stream: &mut TcpStream, input: &str, output: &str) {
        io_assert_raw(stream, input.as_bytes(), output).await;
    }

    async fn io_assert_raw(stream: &mut TcpStream, input: &[u8], output: &str) {
        stream.write_all(input).await.unwrap();
        for byte in output.bytes() {
            let mut buffer = [0u8; 1];
            let read = stream.read(&mut buffer[..]).await.unwrap();
            assert_eq!(1, read);
            assert_eq!(byte, buffer[0]);
        }
    }

    #[test]
    fn basic_io() {
        let mut sched = Builder::new()
            .basic_scheduler()
            .enable_io()
            .build()
            .unwrap();

        sched.block_on(async {
            let loopback = Ipv4Addr::new(127, 0, 0, 1);
            let socket = SocketAddrV4::new(loopback, 0);

            let mut listen = TcpListener::bind(socket).await.unwrap();
            let address = listen.local_addr().unwrap();

            tokio::spawn(async move {
                let (stream, _) = listen.accept().await.unwrap();
                tokio_apply(Evaluator::new(()), stream, Logger::root(Discard, o!())).await;
            });

            let mut input = TcpStream::connect(address).await.unwrap();

            io_assert(&mut input, "+\n", "Err(\"Unrecognized mapping: +\")").await;
            io_assert(&mut input, "x\n", "Err(\"Unrecognized mapping: x\")").await;
            io_assert(&mut input, "-\n", "Err(\"Unrecognized mapping: -\")").await;
            io_assert(
                &mut input,
                "lorem-ipsum\n",
                "Err(\"Unrecognized mapping: lorem-ipsum\")",
            )
            .await;
            io_assert(
                &mut input,
                "lorem\n",
                "Err(\"Unrecognized mapping: lorem\")",
            )
            .await;
            io_assert(&mut input, "(\n)\n", "Err(\"No input to parse\")").await;

            io_assert(&mut input, "?\n", "Ok(\"\")").await;
            io_assert(
                &mut input,
                ")\n",
                "ParseError(\"Unable to parse input: NothingToParse\")",
            )
            .await;

            io_assert_raw(
                &mut input,
                &[0xA0, 0xA1, b'\n'],
                "DecodeError(\"Received invalid UTF-8 input, disconnecting\")",
            )
            .await;
        });
    }

    #[test]
    fn with_decider_advancing_too_far() {
        pub type SomeDec = Option<&'static Decider<Type, String>>;

        pub const DECIDER_LOREM: SomeDec = Some(&Decider {
            description: "<ipsum>",
            decider: decider_lorem,
        });

        fn decider_lorem(_: &[&str], _: &mut SVec<Type>) -> Decision<String> {
            Decision::Accept(1)
        }

        fn handler(_: &mut (), _: &[Type]) -> Result<String, String> {
            Ok("dolor sit amet".to_string())
        }

        let mut sched = Builder::new()
            .basic_scheduler()
            .enable_io()
            .build()
            .unwrap();

        sched.block_on(async {
            let loopback = Ipv4Addr::new(127, 0, 0, 1);
            let socket = SocketAddrV4::new(loopback, 0);

            let mut listen = TcpListener::bind(socket).await.unwrap();
            let address = listen.local_addr().unwrap();

            tokio::spawn(async move {
                let (stream, _) = listen.accept().await.unwrap();
                let mut evaluator = Evaluator::new(());
                evaluator
                    .register((&[("lorem", DECIDER_LOREM)], handler))
                    .unwrap();
                tokio_apply(evaluator, stream, Logger::root(Discard, o!())).await;
            });

            let mut input = TcpStream::connect(address).await.unwrap();

            io_assert(&mut input, "lorem\n", "Err(\"Decider advanced too far\")").await;
            io_assert(&mut input, "?\n", "Ok(\"lorem <ipsum>\")").await;
            io_assert(&mut input, "? lorem\n", "Ok(\"lorem <ipsum>\")").await;
            io_assert(&mut input, "? ipsum\n", "Ok(\"lorem <ipsum>\")").await;
            io_assert(&mut input, "lorem ipsum\n", "Ok(\"dolor sit amet\")").await;

            let mut long = [b'l'; 1025];
            long[1024] = b'\n';
            io_assert(
                &mut input,
                from_utf8(&long).unwrap(),
                "DecodeError(\"Internal buffer is full, disconnecting\")",
            )
            .await;
        });
    }
}
