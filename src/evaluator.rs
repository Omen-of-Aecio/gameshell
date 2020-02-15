//! Core virtual machine.
use crate::{types::Type, Feedback};
use cmdmat::{self, Either, LookError, Mapping, RegError, Spec};
use metac::{Data, Evaluate, ParseError};
use regex::Regex;

/// The virtual machine that runs commands
///
/// The virtual machine interprets strings and provides an output. It operates on the strings
/// according to the specified mapping table, which can be manipulated via [Evaluator::register] and
/// [Evaluator::register_many].
///
/// Builting commands are `autocomplete`, which tries to look ahead by 1 query, and `?` which lists
/// all possible queries.
pub struct Evaluator<'a, C> {
    mapping: Mapping<'a, Type, String, C>,
    context: C,
    current_depth: usize,
    max_depth: usize,
}

impl<'a, C> Evaluator<'a, C> {
    /// Create a new VM which owns a `context`. The context is used in handler functions and can be
    /// mutated.
    pub fn new(context: C) -> Self {
        Self {
            mapping: Mapping::default(),
            context,
            current_depth: 0,
            max_depth: 100,
        }
    }

    /// Set the recursion limit of nested calls.
    pub fn set_recursion_limit(&mut self, limit: usize) {
        self.max_depth = limit;
    }

    /// Get a reference to this machine's context.
    pub fn context(&self) -> &C {
        &self.context
    }

    /// Get a mutable reference to this machine's context.
    pub fn context_mut(&mut self) -> &mut C {
        &mut self.context
    }

    /// Register a handler function for a command.
    pub fn register(&mut self, spec: Spec<'_, 'a, Type, String, C>) -> Result<(), RegError> {
        self.mapping.register(spec)
    }

    /// Register an array of handler functions for a command, see [Evaluator::register].
    pub fn register_many(
        &mut self,
        spec: &[Spec<'_, 'a, Type, String, C>],
    ) -> Result<(), RegError> {
        self.mapping.register_many(spec)
    }

    // Parse subcommands recursively into a vector of strings, fail with feedback otherwise
    fn parse_subcommands(&mut self, cmds: &[Data]) -> Result<Vec<String>, String> {
        let mut content: Vec<String> = Vec::new();
        for cmd in cmds {
            match cmd {
                Data::Atom(string) => {
                    content.push((*string).into());
                }
                Data::Command(string) => {
                    if let Some('#') = string.chars().next() {
                        content.push((string[1..]).into());
                    } else {
                        if self.current_depth == self.max_depth {
                            return Err(format!["Recursion limit reached: {}", self.max_depth]);
                        }
                        self.current_depth += 1;
                        let res = self.interpret_single(string);
                        self.current_depth -= 1;
                        match res {
                            Ok(Feedback::Ok(string)) => {
                                content.push(string);
                            }
                            Ok(Feedback::Err(res)) => {
                                return Err(res);
                            }
                            Err(ParseError::DanglingLeftParenthesis) => {
                                return Err("Dangling left parenthesis".into());
                            }
                            Err(ParseError::PrematureRightParenthesis) => {
                                return Err("Right parenthesis encountered with no matching left parenthesis".into());
                            }
                            Err(ParseError::NothingToParse) => {
                                return Err("No input to parse".into());
                            }
                        }
                    }
                }
            }
        }
        Ok(content)
    }

    fn handle_any_builtin_commands(&mut self, content: &[&str]) -> Option<Feedback> {
        fn mapping_to_list<C>(mapping: &'_ Mapping<'_, Type, String, C>) -> Vec<String> {
            let mut builder = vec![];
            for (key, entry) in mapping.iter() {
                let (parameter, spacer) = if let Some(decider) = entry.decider() {
                    (decider.description, " ")
                } else {
                    (" ", "")
                };

                if entry.finalizer().is_some() {
                    builder.push(
                        String::from(*key)
                            + if entry.decider().is_some() { " " } else { "" }
                            + parameter,
                    );
                }
                for command in mapping_to_list(entry) {
                    builder.push(String::from(*key) + spacer + parameter + spacer + &command);
                }
            }
            builder
        }
        if let Some(front) = content.first() {
            if *front == "autocomplete" {
                match self.mapping.partial_lookup(&content[1..]) {
                    Ok(Either::Left(mapping)) => {
                        let mut col = mapping
                            .get_direct_keys()
                            .map(|k| {
                                let mut s = String::new() + k.literal;
                                if k.decider.is_some() {
                                    s += " ";
                                }
                                s += if k.decider.is_some() {
                                    k.decider.unwrap().description
                                } else {
                                    ""
                                };
                                s += if k.finalizer.is_some() {
                                    " (final)"
                                } else {
                                    ""
                                };
                                s
                            })
                            .collect::<Vec<_>>();
                        if col.is_empty() {
                            return Some(Feedback::Ok("No more handlers".into()));
                        } else {
                            col.sort();
                            return Some(Feedback::Ok(col.join(", ")));
                        }
                    }
                    Ok(Either::Right(name)) => {
                        return Some(Feedback::Ok(name.into()));
                    }
                    Err(err) => {
                        return Some(lookerr_to_evalres(err));
                    }
                }
            }

            if *front == "?" {
                let mut list = mapping_to_list(&self.mapping);
                if let Some(regex) = content.get(1) {
                    if content.len() >= 3 {
                        return Some(Feedback::Err("Too many arguments to: ?".to_string()));
                    }
                    match Regex::new(&(".*".to_string() + &regex + ".*")) {
                        Ok(regex) => {
                            let joined = list.join("\n");
                            list.clear();
                            for capture in regex.captures_iter(&joined) {
                                list.push(capture[0].to_string());
                            }
                        }
                        Err(error) => {
                            return Some(Feedback::Err(format!(
                                "Regex could not be compiled: {}",
                                error
                            )));
                        }
                    }
                }
                list.sort();
                return Some(Feedback::Ok(list.join("\n")));
            }
        }
        None
    }

    #[cfg(test)]
    fn get_current_depth(&self) -> usize {
        self.current_depth
    }
}

impl<'a, C> Evaluate<Feedback> for Evaluator<'a, C> {
    fn evaluate(&mut self, commands: &[Data]) -> Feedback {
        let content = match self.parse_subcommands(commands) {
            Ok(content) => content,
            Err(err) => return Err(err),
        };
        let content_ref = content.iter().map(|s| &s[..]).collect::<Vec<_>>();

        let res = self.mapping.lookup(&content_ref[..]);
        match res {
            Ok(fin) => {
                let res = fin.0(&mut self.context, &fin.1);
                match res {
                    Ok(res) => Feedback::Ok(res),
                    Err(res) => Feedback::Err(res),
                }
            }
            Err(err) => {
                if let Some(result) = self.handle_any_builtin_commands(&content_ref[..]) {
                    return result;
                }
                lookerr_to_evalres(err)
            }
        }
    }
}

fn lookerr_to_evalres(err: LookError<String>) -> Feedback {
    match err {
        LookError::DeciderAdvancedTooFar => Feedback::Err("Decider advanced too far".into()),
        LookError::DeciderDenied(desc, decider) => {
            Feedback::Err(format!["Expected {}. Decider: {}", desc, decider])
        }
        LookError::FinalizerDoesNotExist => Feedback::Err("Finalizer does not exist".into()),
        LookError::UnknownMapping(token) => {
            Feedback::Err(format!["Unrecognized mapping: {}", token])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicates::*;
    use crossbeam_channel::{bounded, Sender};

    #[test]
    fn evaluate_simple() {
        let mut eval = Evaluator::new(0usize);

        fn handler(_context: &mut usize, _args: &[Type]) -> Result<String, String> {
            Ok("".into())
        }

        eval.register((&[("call", None)], handler)).unwrap();
        assert_eq!(
            Feedback::Ok("".into()),
            eval.interpret_single("call").unwrap()
        );
    }

    #[test]
    fn list_available() {
        let mut eval = Evaluator::new(0usize);

        fn handler(_context: &mut usize, _args: &[Type]) -> Result<String, String> {
            Ok("fafa".into())
        }

        eval.register((&[("call", ANY_F32), ("something", None)], handler))
            .unwrap();
        eval.register((&[("call", None), ("abc", MANY_I32)], handler))
            .unwrap();
        eval.register((
            &[("log", None), ("context", MANY_I32), ("level", ANY_ATOM)],
            handler,
        ))
        .unwrap();
        assert_eq!(
            Feedback::Ok("call <f32> abc <i32> ...\ncall <f32> something \nlog context <i32> ... level <atom>".into()),
            eval.interpret_single("?").unwrap()
        );
    }

    #[test]
    fn list_available_using_regex() {
        let mut eval = Evaluator::new(0usize);

        fn handler(_context: &mut usize, _args: &[Type]) -> Result<String, String> {
            Ok("fafa".into())
        }

        eval.register((&[("call", ANY_F32), ("something", None)], handler))
            .unwrap();
        eval.register((&[("call", None), ("abc", MANY_I32)], handler))
            .unwrap();
        eval.register((
            &[("log", None), ("context", MANY_I32), ("level", ANY_ATOM)],
            handler,
        ))
        .unwrap();
        assert_eq!(
            Feedback::Ok("call <f32> abc <i32> ...\ncall <f32> something ".into()),
            eval.interpret_single("? call").unwrap()
        );
        assert_eq!(
            Feedback::Ok("call <f32> abc <i32> ...".into()),
            eval.interpret_single("? abc").unwrap()
        );
        assert_eq!(
            Feedback::Err("Regex could not be compiled: regex parse error:\n    .*\\x.*\n        ^\nerror: invalid hexadecimal digit".into()),
            eval.interpret_single("? \\x").unwrap()
        );
    }

    #[test]
    fn evaluate_any_f32() {
        let mut eval = Evaluator::new(0usize);

        fn handler(_context: &mut usize, _args: &[Type]) -> Result<String, String> {
            Ok("".into())
        }

        eval.register((&[("call", ANY_F32)], handler)).unwrap();
        assert_eq!(
            Feedback::Err(
                "Expected <f32>. Decider: Too few elements: [], length: 0, expected: 1".into()
            ),
            eval.interpret_single("call").unwrap()
        );
        assert_eq!(
            Feedback::Ok("".into()),
            eval.interpret_single("call 3.14159").unwrap()
        );
        assert_eq!(
            Feedback::Ok("".into()),
            eval.interpret_single("call 3").unwrap()
        );
        assert_eq!(
            Feedback::Err("Expected <f32>. Decider: got string: alpha".into()),
            eval.interpret_single("call alpha").unwrap()
        );
    }

    #[test]
    fn a_command_triggers_a_channel() {
        let (tx, rx) = bounded(1);

        let mut eval = Evaluator::new(tx);

        fn handler(context: &mut Sender<f32>, args: &[Type]) -> Result<String, String> {
            match args[0] {
                Type::F32(number) => context.send(number).unwrap(),
                _ => panic!("Input was not an f32"),
            }
            Ok("".into())
        }

        eval.register((&[("call", ANY_F32)], handler)).unwrap();

        eval.interpret_single("call 3.14159").unwrap().unwrap();

        assert_eq!(3.14159, rx.recv().unwrap());
    }

    #[test]
    fn base64_decoding_into_raw() {
        let (tx, rx) = bounded(1);

        let mut eval = Evaluator::new(tx);

        fn handler(context: &mut Sender<Vec<u8>>, args: &[Type]) -> Result<String, String> {
            match args[0] {
                Type::Raw(ref bytes) => context.send(bytes.clone()).unwrap(),
                _ => panic!("Input not raw bytes"),
            }
            Ok("".into())
        }

        eval.register((&[("call", ANY_BASE64)], handler)).unwrap();

        eval.interpret_single("call iVBORw0KGgo").unwrap().unwrap();

        // PNG magic number
        static PNG_MAGIC_NUMBER: &[u8] = &[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
        assert_eq!(PNG_MAGIC_NUMBER, &rx.recv().unwrap()[..]);
    }

    #[test]
    fn touching_subcommand() {
        let mut eval = Evaluator::new(0u32);

        fn handler(context: &mut u32, _: &[Type]) -> Result<String, String> {
            *context += 1;
            Ok("string".into())
        }

        eval.register((&[("call", ANY_STRING)], handler)).unwrap();
        eval.interpret_single("call(call _)").unwrap().unwrap();
        assert_eq!(2, *eval.context());
        eval.interpret_single("call(call(call _))")
            .unwrap()
            .unwrap();
        assert_eq!(2 + 3, *eval.context());
        eval.interpret_multiple("call(call(call _))")
            .unwrap()
            .unwrap();
        assert_eq!(2 + 3 + 3, *eval.context());
    }

    #[test]
    fn recursive_call_stack() {
        let mut eval = Evaluator::new(());

        fn handler(_: &mut (), _: &[Type]) -> Result<String, String> {
            Ok("string".into())
        }

        eval.register((&[("call", ANY_STRING)], handler)).unwrap();

        let mut call = "call".to_string();
        let count = 1000;
        for _ in 0..count {
            call += "(call";
        }
        call += " _";
        for _ in 0..count {
            call += ")";
        }

        assert_eq!(
            Err("Recursion limit reached: 100".into()),
            eval.interpret_single(&call).unwrap()
        );
        assert_eq!(0, eval.get_current_depth());
    }

    #[test]
    fn question_only_handles_one() {
        let mut eval = Evaluator::new(());
        assert_eq!("", eval.interpret_single("? _").unwrap().unwrap());
        assert_eq!(
            "Too many arguments to: ?",
            eval.interpret_single("? _ _").unwrap().unwrap_err()
        );
        assert_eq!(
            "Too many arguments to: ?",
            eval.interpret_single("? _ _ _ _ _").unwrap().unwrap_err()
        );
        assert_eq!(
            "Too many arguments to: ?",
            eval.interpret_multiple("? _ _ _ _ _").unwrap().unwrap_err()
        );
    }

    #[test]
    fn override_builtins() {
        let mut eval = Evaluator::new(());

        assert_eq!(
            Ok("".into()),
            eval.interpret_single("?").unwrap()
        );

        fn handler(_: &mut (), _: &[Type]) -> Result<String, String> {
            Err("? is not available".into())
        }
        eval.register((&[("?", None)], handler)).unwrap();

        assert_eq!(
            Err("? is not available".into()),
            eval.interpret_single("?").unwrap()
        );

        assert_eq!(
            Ok("? (final)".into()),
            eval.interpret_single("autocomplete").unwrap()
        );

        fn autocomplete_handler(_: &mut (), _: &[Type]) -> Result<String, String> {
            Err("autocomplete is not available".into())
        }
        eval.register((&[("autocomplete", None)], autocomplete_handler)).unwrap();

        assert_eq!(
            Err("autocomplete is not available".into()),
            eval.interpret_single("autocomplete").unwrap()
        );
    }
}
