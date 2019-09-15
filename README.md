# GameShell - Practical shell for Rust integration #
[![Latest Version][s1]][l1] [![docs.rs][s4]][l4]

[s1]: https://img.shields.io/crates/v/gameshell.svg
[l1]: https://crates.io/crates/gameshell
[s4]: https://docs.rs/gameshell/badge.svg
[l4]: https://docs.rs/gameshell/

GameShell is a fast interpreter for integration into Rust applications. It combines the features of lisp and bash so it can be practical.

## Language ##

GameShell is a language that is very simple, its basic form is the following:

    command argument-1 argument-2 ...

It splits all commands by line and all arguments by whitespace.

We can nest commands by using parentheses:

    command-1 (command-2 argument-1 argument-2) argument-3

If we want, we can also run these on multiple lines

    command-1 (
        command-2 argument-1 argument-2
    ) argument-3

When there are no more ()-nestings, the next newline will complete a command.

### Strings ###

Literal strings are made by using the built-in `#` command:

    print (#This is a literal string)

### What is a command? ###

A command is a function that looks like this:

     fn handler(context: &mut Context, args: &[Type]) -> Result<String, String> {
        ...
     }

You provide the context and GameShell will provide the `args` list. You can then do whatever you want inside the handler. If the handler returns `Ok`, that result can be used as an argument to another command, if it returns `Err`, any nested computation is cancelled.

Commands are tied to a Command Matcher (`cmdmat` for short). You register a command matcher to a handler:

    eval.register((&[("command", None)], handler)).unwrap();

This matches the command `command` with 0 arguments. The `None` is a so-called `Decider`, it will parse any arguments given to `command`, and decide whether the invocation is valid or not.

Please see the predicates module documentation for more details and examples.

## Features ##

 * Regex command search - Using `?` as a command, returns a list of registered commands that match a regex
 * Literal strings - Using the `#` pseudo-command.
 * Command nesting - `a (b (c d))`
 * Command handlers - Commands call into Rust code.
 * Stack overflow protection for nested calls - Aborts a command if the nesting has exceeded a certain treshold (can be customized).
 * Custom command validators/classifiers (deciders)
 * Input limiting - Limit the amount of characters a command can consist of.
