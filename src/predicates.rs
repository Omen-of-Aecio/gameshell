//! Contains pre-built predicates for use outside this library for creating command
//! specifications.
//!
//! ## Writing a custom predicate ##
//!
//! ```
//! use gameshell::{types::Type, Evaluate, Evaluator};
//! use gameshell::cmdmat::{Decider, Decision, SVec};
//!
//! // This is the general decider type to use
//! pub type SomeDec = Option<&'static Decider<Type, String>>;
//!
//! // This is your decider that you will use when registering a handler
//! pub const I32_NUMBER_ABOVE_123: SomeDec = Some(&Decider {
//!     description: "<i32-over-123>",
//!     decider: i32_number_above_123,
//! });
//!
//! fn i32_number_above_123(input: &[&str], out: &mut SVec<Type>) -> Decision<String> {
//!     // Check if input is non-empty
//!     if input.is_empty() {
//!         return Decision::Deny("No input received".into());
//!     }
//!
//!     // Ensure string contains no whitespaces
//!     for i in input[0].chars() {
//!         if i.is_whitespace() {
//!             return Decision::Deny(input[0].into());
//!         }
//!     }
//!
//!     // Parse string to i32
//!     let number = input[0].parse::<i32>();
//!     let number = match number {
//!         Ok(number) => number,
//!         Err(err) => return Decision::Deny(format!["{:?}", err]),
//!     };
//!
//!     // Ensure number is >123
//!     if !(number > 123) {
//!         return Decision::Deny("Number is not >123".into());
//!     }
//!
//!     // All checks passed, push the value to the output
//!     out.push(Type::I32(number));
//!
//!     // Tell the GameShell machinery that we have consumed 1 argument
//!     Decision::Accept(1)
//! }
//!
//! fn handler(_: &mut (), args: &[Type]) -> Result<String, String> {
//!     if let [Type::I32(number)] = args {
//!         println!["The number {} is definitely greater than 123", number];
//!         Ok("We can return whatever we want to here".into())
//!     } else {
//!         panic!["Wrong arguments"];
//!     }
//! }
//!
//! let mut eval = Evaluator::new(());
//! eval.register((&[("my-command", I32_NUMBER_ABOVE_123)], handler));
//! eval.interpret_single("my-command 124").unwrap().unwrap();
//! assert_eq![Err("Expected <i32-over-123>. Decider: Number is not >123".into()), eval.interpret_single("my-command -9").unwrap()];
//! ```
use crate::types::Type;
use cmdmat::{Decider, Decision, SVec};

// ---

/// Decider type alias
pub type SomeDec = Option<&'static Decider<Type, String>>;

// Please keep this list sorted

/// Accepts a single string which does not contain whitespace
pub const ANY_ATOM: SomeDec = Some(&Decider {
    description: "<atom>",
    decider: any_atom_function,
});
/// Accepts any base64 string
pub const ANY_BASE64: SomeDec = Some(&Decider {
    description: "<base64>",
    decider: any_base64_function,
});
/// Accepts a single boolean
pub const ANY_BOOL: SomeDec = Some(&Decider {
    description: "<true/false>",
    decider: any_bool_function,
});
/// Accepts a single f32
pub const ANY_F32: SomeDec = Some(&Decider {
    description: "<f32>",
    decider: any_f32_function,
});
/// Accepts a single i32
pub const ANY_I32: SomeDec = Some(&Decider {
    description: "<i32>",
    decider: any_i32_function,
});
/// Accepts a single string
pub const ANY_STRING: SomeDec = Some(&Decider {
    description: "<string>",
    decider: any_string_function,
});
/// Accepts a single u8
pub const ANY_U8: SomeDec = Some(&Decider {
    description: "<u8>",
    decider: any_u8_function,
});
/// Accepts a single usize
pub const ANY_USIZE: SomeDec = Some(&Decider {
    description: "<usize>",
    decider: any_usize_function,
});
/// Ignores all arguments
pub const IGNORE_ALL: SomeDec = Some(&Decider {
    description: "<anything> ...",
    decider: ignore_all_function,
});
/// Accepts 1 or more i32s
pub const MANY_I32: SomeDec = Some(&Decider {
    description: "<i32> ...",
    decider: many_i32_function,
});
/// Accepts 1 or more strings
pub const MANY_STRING: SomeDec = Some(&Decider {
    description: "<string> ...",
    decider: many_string_function,
});
/// Accepts a positive f32
pub const POSITIVE_F32: SomeDec = Some(&Decider {
    description: "<f32>=0>",
    decider: positive_f32_function,
});
/// Accepts two strings
pub const TWO_STRINGS: SomeDec = Some(&Decider {
    description: "<string> <string>",
    decider: two_string_function,
});

// ---

fn any_atom_function(input: &[&str], out: &mut SVec<Type>) -> Decision<String> {
    aslen(input, 1)?;
    for i in input[0].chars() {
        if i.is_whitespace() {
            return Decision::Deny(input[0].into());
        }
    }
    out.push(Type::Atom(input[0].to_string()));
    Decision::Accept(1)
}

fn any_base64_function(input: &[&str], out: &mut SVec<Type>) -> Decision<String> {
    aslen(input, 1)?;
    match base64::decode(input[0]) {
        Ok(base64) => {
            out.push(Type::Raw(base64));
            Decision::Accept(1)
        }
        Err(err) => Decision::Deny(format!["{}", err]),
    }
}

fn any_bool_function(input: &[&str], out: &mut SVec<Type>) -> Decision<String> {
    aslen(input, 1)?;
    match input[0].parse::<bool>().ok().map(Type::Bool) {
        Some(num) => {
            out.push(num);
        }
        None => {
            return Decision::Deny("got string: ".to_string() + input[0]);
        }
    }
    Decision::Accept(1)
}

fn any_f32_function(input: &[&str], out: &mut SVec<Type>) -> Decision<String> {
    aslen(input, 1)?;
    match input[0].parse::<f32>().ok().map(Type::F32) {
        Some(num) => {
            out.push(num);
        }
        None => {
            return Decision::Deny("got string: ".to_string() + input[0]);
        }
    }
    Decision::Accept(1)
}

fn any_i32_function(input: &[&str], out: &mut SVec<Type>) -> Decision<String> {
    aslen(input, 1)?;
    match input[0].parse::<i32>().ok().map(Type::I32) {
        Some(num) => {
            out.push(num);
        }
        None => {
            return Decision::Deny("got string: ".to_string() + input[0]);
        }
    }
    Decision::Accept(1)
}

fn any_string_function(input: &[&str], out: &mut SVec<Type>) -> Decision<String> {
    aslen(input, 1)?;
    out.push(Type::String(input[0].to_string()));
    Decision::Accept(1)
}

fn any_u8_function(input: &[&str], out: &mut SVec<Type>) -> Decision<String> {
    aslen(input, 1)?;
    match input[0].parse::<u8>().ok().map(Type::U8) {
        Some(num) => {
            out.push(num);
        }
        None => {
            return Decision::Deny("got string: ".to_string() + input[0]);
        }
    }
    Decision::Accept(1)
}

fn any_usize_function(input: &[&str], out: &mut SVec<Type>) -> Decision<String> {
    aslen(input, 1)?;
    match input[0].parse::<usize>().ok().map(Type::Usize) {
        Some(num) => {
            out.push(num);
        }
        None => {
            return Decision::Deny("got string: ".to_string() + input[0]);
        }
    }
    Decision::Accept(1)
}

fn ignore_all_function(input: &[&str], _: &mut SVec<Type>) -> Decision<String> {
    Decision::Accept(input.len())
}

fn many_i32_function(input: &[&str], out: &mut SVec<Type>) -> Decision<String> {
    let mut cnt = 0;
    for i in input.iter() {
        if let Some(num) = i.parse::<i32>().ok().map(Type::I32) {
            aslen(input, cnt + 1)?;
            out.push(num);
            cnt += 1;
        } else {
            break;
        }
    }
    Decision::Accept(cnt)
}

fn many_string_function(input: &[&str], out: &mut SVec<Type>) -> Decision<String> {
    aslen(input, input.len())?;
    let mut cnt = 0;
    for (idx, i) in input.iter().enumerate() {
        out.push(Type::String((*i).into()));
        cnt = idx + 1;
    }
    Decision::Accept(cnt)
}

fn positive_f32_function(input: &[&str], out: &mut SVec<Type>) -> Decision<String> {
    aslen(input, 1)?;
    match input[0].parse::<f32>().ok().map(Type::F32) {
        Some(Type::F32(val)) if val >= 0.0f32 => {
            out.push(Type::F32(val));
        }
        _ => {
            return Decision::Deny("got string: ".to_string() + input[0]);
        }
    }
    Decision::Accept(1)
}

fn two_string_function(input: &[&str], out: &mut SVec<Type>) -> Decision<String> {
    if input.len() == 1 {
        return Decision::Deny("expected 1 more string".into());
    }
    aslen(input, 2)?;
    out.push(Type::String(input[0].to_string()));
    out.push(Type::String(input[1].to_string()));
    Decision::Accept(2)
}

// ---

fn aslen(input: &[&str], input_l: usize) -> Result<(), String> {
    if input.len() < input_l {
        Err(format![
            "Too few elements: {:?}, length: {}, expected: {}",
            input,
            input.len(),
            input_l
        ])
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[quickcheck_macros::quickcheck]
    fn basic_quickcheck(input: Vec<String>) {
        let input = &input.iter().map(|string| &string[..]).collect::<Vec<_>>()[..];
        let out = &mut SVec::new();

        any_atom_function(input, out);
        any_base64_function(input, out);
        any_bool_function(input, out);
        any_f32_function(input, out);
        any_string_function(input, out);
        any_u8_function(input, out);
        ignore_all_function(input, out);
        many_string_function(input, out);
        positive_f32_function(input, out);
        two_string_function(input, out);
    }
}
