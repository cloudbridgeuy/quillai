//! # How does `nom` works
//!
//! Nom works by creating small, simple, `parser` functions that are then
//! combined together through `combinators` in order to handle more complex
//! tasks.
//!
//! A `parser` takes in an _input_ and returns a _result_, where:
//!
//! - `Ok` indicates the parser successfully found what it was looking for.
//! - `Err` indicates the parser could not find what it was looking for.
//!
//! If the `parser` is successful it'll return a tuple. The first field of
//! the tuple will contain everything the parser did not process. The second
//! will contain everything the parser processed. The idea is that a parser
//! can happily parse the `first` part of an input, without being able to
//! parse the whole thing.
//!
//! To simplify the return definition of a parser, `nom` exposes the `IResult`
//! type, which uses the `Ok` branch to return a tuple, and the `Err` branch
//! for the resulting parsing error.
//!
//! Here's the simplest parser you can create:
//!
//! ```rust
//! pub fn do_nothing(input: &str) -> IResult<&str, &str> {
//!     Ok((input, ""))
//! }
//!
//! fn main() -> anyhow::Result<()> {
//!     let (remaining_input, output) = do_nothing("my_input");
//!     assert_eq!(remaining_input, "my_input");
//!     assert_eq!(output, "");
//! }
//! ```
//!
//! In `nom` we call a collection of `bytes` a `tag`. For example, to parse the
//! string "abc" we could do it using the `nom::bytes::complete::tag` function.
//!
//! The type signature of the `tag()` function is:
//!
//! ```rust
//! fn tag(tag: &str) -> (impl Fn(&str) -> IResult<&str, Error>)
//! ```
//!
//! Since the signature returns a function, it's useful to wrap them in a higher
//! order function to expose its functionality.
//!
//! ```rust
//! fn parse_abc(input: &str) -> IResult<&str, &str> {
//!     nom::bytes::complete::tag("abc")(input)
//! }
//!
//! fn main() -> anyhow::Result<()> {
//!     let (remaining_input, output) = parse_abc("abc123");
//!     assert_eq!(remaining_input, "123");
//!     assert_eq!(output, "abc");
//!     assert!(parse_abc("defWorld").is_err());
//! }
//! ```
//!
//! > [!TIP] You can use the `nom::bytes::complete::tag_no_case` for case-insensitive
//! >        matches.
//!
//! Besides `tags`, `nom` exposes pre-defined `parsers` that we can use for more dynamic
//! situations. Here's a brief list:
//!
//! - `alpha0`: Recognizes zero or more lowercase and uppercase alphabetic characters.
//! - `alpha1`: Same as `alpha0` but retutns at least one character.
//! - `alphanumeric0`: Recognizes zero or more numberical alphabetic characters.
//! - `alphanumeric1`: Same as `alphanumeric0` but returns at least one character.
//! - `digit0`: Recognizes zero or more digits.
//! - `digit1`: Same as `digit0` but returns at least one character.
//! - `multispace0`: Recognizes zero or more spaces, tabs, carriage returns and line feeds.
//! - `multispace1`: Same as `multispace0` but returns at least one character.
//! - `space0`: Recognizes zero or more spaces and tabs.
//! - `space1`: Same as `space0` but returns at least one character.
//! - `line_ending`: Recognizes an end of line (both `\n` and `\r\n`.)
//! - `newline`: Matches a newline character (`\n`.)
//! - `tab`: Matches a tab character (`\t`.)
//!
//! > [!TIP] It is best to use these functions inside a function that returns an `IResult`.
//!
//! Sometimes we need to handle situations in which more than one parser could be valid.
//! To represent this, we can use `combinators` like `nom::branch::alt`. This `combinator`
//! will execute each parser in a tuple until it finds the one that does not error.
//!
//! > [!NOTE] If all `error`, then by default you are given the last one.
//!
//! ```rust
//! fn parse_abc_or_def(input: &str) -> IResult<&str, &str> {
//!     nom::branch::alt((
//!         nom::bytes::complete::tag("abc"),
//!         nom::bytes::complete::tag("def"),
//!     ))(input)
//! }
//!
//! fn main() -> anyhow::Result<()> {
//!     let (remaining_input, output) = parse_abc_or_def("def123");
//!     assert_eq!(remaining_input, "123");
//!     assert_eq!(output, "def");
//!     assert!(parse_abc_or_def("ghiWorld").is_err());
//! }
//! ```
//!
//! The power of `nom` comes into play when we start to compose these parsers. The simplest
//! way to do it is to evaluate them in sequence.
//!
//! ```rust
//! fn main() -> anyhow::Result<()> {
//!     let input = "abcghi";
//!     let (remaining_input, abc_output) = nom::bytes::complete::tag("abc")(input)?;
//!     let (remaining_input, ghi_output) = nom::bytes::complete::tag("ghi")(remaining_input)?;
//!     println!("abc_output = {}\nghi_output = {}", abc_output, ghi_output);
//! }
//! ```
//!
//! Composing parsers is so common that `nom` exposes `combinators` that simplify the
//! process of composing them together. For example, the `nom::sequence::tuple` combinator
//! takes a tuple of parsers, and either returns an `Ok` with a tuple of all of the successful
//! parsers, or it returns the `Err` of the first failed parser.
//!
//! You can use the `nom::combinator::value` function to convert from a successful parse to
//! a particular value.
//!
//! ```rust
//! fn parse_bool(input: &str) -> IResult<&str, bool> {
//!     nom::branch::alt((
//!         nom::combinator::value(true, tag("true")),
//!         nom::combinator::value(false, tag("false")),
//!     ))(input)
//! }
//!
//! fn main() -> anyhow::Result<()> {
//!     // Parses the `true` out.
//!     let (remaining_input, output) = parse_bool("true123")?;
//!     assert_eq!(remaining_input, "123");
//!     assert_eq!(output, true);
//!
//!     // Parses the `false` out.
//!     let (remaining_input, output) = parse_bool("false123")?;
//!     assert_eq!(remaining_input, "123");
//!     assert_eq!(output, false);
//! }
//! ```
//!
//! Some parses take a `predicate` function that returns a boolean value. For example:
//!
//! - `take_till`: will consume the input until its input meets the predicate.
//! - `take_while`: will consume the input until the input **does not**
//!                 meet the predicate.
//! - `take_until`: will consume the input until the first occurrence of a
//!                 pattern of bytes.
//!
//! There are combinators that repeat a parser. For example, `nom::multi::many0` will
//! apply a parser as many times as possible, returning a vector of the results of
//! those parsers.

use nom::{IResult, Parser};

#[derive(Clone, Debug, PartialEq)]
pub enum SmtpCommand {
    Ehlo,
    MailFrom(String),
    RcptTo(String),
    Noop,
    Rset,
    Data,
    Quit,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Header {
    name: String,
    value: String,
}

fn not_email_end(s: &str) -> IResult<&str, &str> {
    nom::bytes::complete::is_not("> \t\r\n")(s)
}

fn email(input: &str) -> IResult<&str, &str> {
    nom::sequence::preceded(
        nom::character::multispace0(),
        nom::sequence::delimited(
            nom::bytes::complete::tag("<"),
            nom::sequence::delimited(
                nom::character::multispace0(),
                not_email_end,
                nom::character::multispace0(),
            ),
            nom::bytes::complete::tag(">"),
        ),
    )
    .parse(input)
}

pub fn mail_from(input: &str) -> IResult<&str, SmtpCommand> {
    let (input, _) = nom::bytes::complete::tag_no_case("Mail From")(input)?;
    let (input, _) = nom::bytes::complete::take_until("<")(input)?;
    let (input, email) = email(input)?;

    Ok((input, SmtpCommand::MailFrom(email.to_string())))
}

pub fn rcpt_to(input: &str) -> IResult<&str, SmtpCommand> {
    let (input, _) = nom::bytes::complete::tag_no_case("Rcpt To")(input)?;
    let (input, _) = nom::bytes::complete::take_until("<")(input)?;
    let (input, email) = email(input)?;

    Ok((input, SmtpCommand::RcptTo(email.to_string())))
}

pub fn parse_command(input: &str) -> IResult<&str, SmtpCommand> {
    nom::sequence::preceded(
        nom::character::multispace0(),
        nom::branch::alt((
            nom::combinator::value(SmtpCommand::Noop, nom::bytes::complete::tag_no_case("noop")),
            nom::combinator::value(SmtpCommand::Rset, nom::bytes::complete::tag_no_case("rset")),
            nom::combinator::value(SmtpCommand::Data, nom::bytes::complete::tag_no_case("data")),
            nom::combinator::value(SmtpCommand::Ehlo, nom::bytes::complete::tag_no_case("ehlo")),
            nom::combinator::value(SmtpCommand::Quit, nom::bytes::complete::tag_no_case("quit")),
            mail_from,
            rcpt_to,
        )),
    )
    .parse(input)
}

pub fn parse_header(input: &str) -> IResult<&str, Header> {
    let (input, (key, value)) = nom::sequence::separated_pair(
        nom::bytes::complete::take_until(":"),
        nom::character::char(':'),
        // Take until end of line
        nom::bytes::complete::take_until("\r\n"),
    )
    .parse(input)?;

    Ok((
        input,
        Header {
            name: key.trim().to_string(),
            value: value.trim().to_string(),
        },
    ))
}

/*
- Sender: EHLO
- Receiver: 250 OK
- Sender: MAIL FROM: \<sender@mysenderdomain.com>
- Receiver: 250 OK
- Sender: RCPT TO: \<receiver@myreceiverdomain.com>
- Receiver: 250 OK
- Sender: DATA
- Receiver: 354 End data with \<CR>\<LF>.\<CR>\<LF>
- Sender: Subject: Test email
- Sender: Test email content
- Sender: .
- Receiver: 250 OK
- Sender: QUIT
- Receiver: 221 Bye
*/
mod test {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use crate::prelude::Result;

    #[test]
    fn parse_smtp_message() -> Result<()> {
        assert_eq!(parse_command("EHLO")?, ("", SmtpCommand::Ehlo));
        assert_eq!(parse_command("    EHLO")?, ("", SmtpCommand::Ehlo));

        assert!(parse_command("    INVALID").is_err(),);

        Ok(())
    }
}
