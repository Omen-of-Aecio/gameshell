//! Contains pre-built predicates for use outside this library for creating command
//! specifications.
use crate::{decision::Decision, types::Type};
use cmdmat::{Decider, Decision as Cecision, SVec};

// ---

/// Decider type alias
pub type SomeDec = Option<&'static Decider<Type, Decision>>;

// Please keep this list sorted

/// Accepts a single string which does not contain whitespace
pub const ANY_ATOM: SomeDec = Some(&Decider {
    description: "<atom>",
    decider: any_atom_function,
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
/// Accepts two strings
pub const TWO_STRINGS: SomeDec = Some(&Decider {
    description: "<string> <string>",
    decider: two_string_function,
});

// ---

// TODO: Replace usage of this macro with `?'
macro_rules! ret_if_err {
    ($e:expr) => {{
        let res = $e;
        match res {
            Ok(x) => x,
            Err(res) => {
                return res;
            }
        }
    }};
}

// ---

fn any_atom_function(input: &[&str], out: &mut SVec<Type>) -> Cecision<Decision> {
    ret_if_err![aslen(input, 1)];
    for i in input[0].chars() {
        if i.is_whitespace() {
            return Cecision::Deny(Decision::Err(input[0].into()));
        }
    }
    out.push(Type::Atom(input[0].to_string()));
    Cecision::Accept(1)
}

fn any_bool_function(input: &[&str], out: &mut SVec<Type>) -> Cecision<Decision> {
    ret_if_err![aslen(input, 1)];
    match input[0].parse::<bool>().ok().map(Type::Bool) {
        Some(num) => {
            out.push(num);
        }
        None => {
            return Cecision::Deny(Decision::Err(input[0].into()));
        }
    }
    Cecision::Accept(1)
}

fn any_f32_function(input: &[&str], out: &mut SVec<Type>) -> Cecision<Decision> {
    ret_if_err![aslen(input, 1)];
    match input[0].parse::<f32>().ok().map(Type::F32) {
        Some(num) => {
            out.push(num);
        }
        None => {
            return Cecision::Deny(Decision::Err(input[0].into()));
        }
    }
    Cecision::Accept(1)
}

fn any_i32_function(input: &[&str], out: &mut SVec<Type>) -> Cecision<Decision> {
    ret_if_err![aslen(input, 1)];
    match input[0].parse::<i32>().ok().map(Type::I32) {
        Some(num) => {
            out.push(num);
        }
        None => {
            return Cecision::Deny(Decision::Err(input[0].into()));
        }
    }
    Cecision::Accept(1)
}

fn any_string_function(input: &[&str], out: &mut SVec<Type>) -> Cecision<Decision> {
    ret_if_err![aslen(input, 1)];
    out.push(Type::String(input[0].to_string()));
    Cecision::Accept(1)
}

fn any_u8_function(input: &[&str], out: &mut SVec<Type>) -> Cecision<Decision> {
    ret_if_err![aslen(input, 1)];
    match input[0].parse::<u8>().ok().map(Type::U8) {
        Some(num) => {
            out.push(num);
        }
        None => {
            return Cecision::Deny(Decision::Err(input[0].into()));
        }
    }
    Cecision::Accept(1)
}

fn ignore_all_function(input: &[&str], _: &mut SVec<Type>) -> Cecision<Decision> {
    Cecision::Accept(input.len())
}

fn many_i32_function(input: &[&str], out: &mut SVec<Type>) -> Cecision<Decision> {
    let mut cnt = 0;
    for i in input.iter() {
        if let Some(num) = i.parse::<i32>().ok().map(Type::I32) {
            ret_if_err![aslen(input, cnt + 1)];
            out.push(num);
            cnt += 1;
        } else {
            break;
        }
    }
    Cecision::Accept(cnt)
}

fn many_string_function(input: &[&str], out: &mut SVec<Type>) -> Cecision<Decision> {
    ret_if_err![aslen(input, input.len())];
    let mut cnt = 0;
    for (idx, i) in input.iter().enumerate() {
        out.push(Type::String((*i).into()));
        cnt = idx + 1;
    }
    Cecision::Accept(cnt)
}

fn two_string_function(input: &[&str], out: &mut SVec<Type>) -> Cecision<Decision> {
    if input.len() == 1 {
        return Cecision::Deny(Decision::Help("<string>".into()));
    }
    ret_if_err![aslen(input, 2)];
    out.push(Type::String(input[0].to_string()));
    out.push(Type::String(input[1].to_string()));
    Cecision::Accept(2)
}

// ---

fn aslen(input: &[&str], input_l: usize) -> Result<(), Cecision<Decision>> {
    if input.len() < input_l {
        Err(Cecision::Deny(Decision::Err(format![
            "Too few elements: {:?}",
            input
        ])))
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

        two_string_function(input, out);
        many_string_function(input, out);
        ignore_all_function(input, out);
        any_u8_function(input, out);
        any_string_function(input, out);
        any_f32_function(input, out);
        any_bool_function(input, out);
        any_atom_function(input, out);
    }
}
