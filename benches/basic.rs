use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gameshell::{predicates as pred, types::Type, Evaluate, Evaluator, GameShell, IncConsumer};

// ---

criterion_main!(benches);
criterion_group!(
    benches,
    initialize,
    interpret,
    interpret_complex,
    interpret_nested
);

// ---

fn initialize_gameshell() {
    let read = b"call 1.23\n";
    let mut write = [0u8; 10];

    let mut eval = GameShell::new(0u8, &read[..], &mut write[..]);

    fn handler(context: &mut u8, _args: &[Type]) -> Result<String, String> {
        *context += 1;
        Ok("".into())
    }

    eval.register((&[("call", pred::ANY_F32)], handler))
        .unwrap();

    let buffer = &mut [0u8; 1024];
    eval.run(buffer);

    assert_eq![1, *eval.context()];
}

fn initialize(c: &mut Criterion) {
    c.bench_function("initialize", |b| b.iter(|| initialize_gameshell()));
}

fn interpret(c: &mut Criterion) {
    let mut eval = Evaluator::new(0u8);

    fn handler(_context: &mut u8, _args: &[Type]) -> Result<String, String> {
        Ok("".into())
    }

    eval.register((&[("call", pred::ANY_F32)], handler))
        .unwrap();

    c.bench_function("simple call", move |b| {
        b.iter(|| {
            eval.interpret_single(black_box("call 1.23"))
                .unwrap()
                .unwrap();
        })
    });
}

fn interpret_complex(c: &mut Criterion) {
    let mut eval = Evaluator::new(0u8);

    fn handler(_context: &mut u8, _args: &[Type]) -> Result<String, String> {
        Ok("".into())
    }

    eval.register((&[("call", pred::ANY_F32), ("x", pred::ANY_I32)], handler))
        .unwrap();

    c.bench_function("complex call", move |b| {
        b.iter(|| {
            eval.interpret_single(black_box("call 1.23 x 0"))
                .unwrap()
                .unwrap();
        })
    });
}

fn interpret_nested(c: &mut Criterion) {
    let mut eval = Evaluator::new(0u8);

    fn handler(_context: &mut u8, _args: &[Type]) -> Result<String, String> {
        Ok("123".into())
    }

    eval.register((&[("call", pred::ANY_I32)], handler))
        .unwrap();
    eval.register((&[("call-2", None)], handler)).unwrap();

    c.bench_function("nested call", move |b| {
        b.iter(|| {
            eval.interpret_single(black_box("call (call-2)"))
                .unwrap()
                .unwrap();
        })
    });
}
