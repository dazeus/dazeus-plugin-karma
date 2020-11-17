use nom::{
    branch::alt,
    bytes::complete::{tag, take_till},
    character::complete::{anychar, one_of, satisfy},
    combinator::{map, opt, peek, value},
    multi::{fold_many0, fold_many1},
    sequence::tuple,
    IResult,
};

use crate::karma::{KarmaAmount, KarmaChange, KarmaStyle};

pub fn line(input: &str) -> IResult<&str, Vec<KarmaChange>> {
    fold_many0(element, Vec::new(), |mut acc, elem| {
        if let Some(elem) = elem {
            acc.push(elem);
        }
        acc
    })(input)
}

fn element(input: &str) -> IResult<&str, Option<KarmaChange>> {
    let karma = map(karma_change, Some);
    let anychar = discard(anychar);
    alt((karma, anychar))(input)
}

fn karma_change(input: &str) -> IResult<&str, KarmaChange> {
    alt((explicit_karma_change, implicit_karma_change))(input)
}

fn explicit_karma_change(input: &str) -> IResult<&str, KarmaChange> {
    alt((explicit_karma_notice_change, explicit_karma_silent_change))(input)
}

fn explicit_karma_notice_change(input: &str) -> IResult<&str, KarmaChange> {
    let (input, _) = tag("[")(input)?;
    let (input, term) = take_till(is_notice_char)(input)?;
    let (input, _) = tag("]")(input)?;
    let (input, votes) = modifier(input)?;
    Ok((input, KarmaChange::new(term, votes, KarmaStyle::Notify)))
}

fn explicit_karma_silent_change(input: &str) -> IResult<&str, KarmaChange> {
    let (input, _) = tag("(")(input)?;
    let (input, term) = take_till(is_silent_char)(input)?;
    let (input, _) = tag(")")(input)?;
    let (input, votes) = modifier(input)?;
    Ok((input, KarmaChange::new(term, votes, KarmaStyle::Silent)))
}

fn implicit_karma_change(input: &str) -> IResult<&str, KarmaChange> {
    let (input, term) = implicit_chars(input)?;
    let (input, votes) = modifier(input)?;
    Ok((input, KarmaChange::new(&term, votes, KarmaStyle::Implicit)))
}

fn implicit_chars(input: &str) -> IResult<&str, String> {
    fold_many1(
        tuple((opt(tag("-")), satisfy(is_implicit_char))),
        String::new(),
        |mut acc, (dash, c)| {
            if let Some(dash) = dash {
                acc.push_str(dash)
            }
            acc.push(c);
            acc
        },
    )(input)
}

fn is_implicit_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_notice_char(c: char) -> bool {
    matches!(c, '[' | ']')
}

fn is_silent_char(c: char) -> bool {
    matches!(c, '(' | ')')
}

fn modifier(input: &str) -> IResult<&str, KarmaAmount> {
    let increase = value(KarmaAmount { up: 1, down: 0 }, tag("++"));
    let decrease = value(KarmaAmount { up: 0, down: 1 }, tag("--"));

    let (input, change) = alt((increase, decrease))(input)?;
    let (input, _) = word_boundary(input)?;
    Ok((input, change))
}

fn word_boundary(input: &str) -> IResult<&str, ()> {
    let whitespace = discard(satisfy(char::is_whitespace));
    let punctuation = discard(one_of(",.;:)"));
    let eof = discard(nom::combinator::eof);
    peek(alt((whitespace, punctuation, eof)))(input)
}

fn discard<I, O1, O2: Clone + Default, E: nom::error::ParseError<I>, F>(
    parser: F,
) -> impl FnMut(I) -> IResult<I, O2, E>
where
    F: nom::Parser<I, O1, E>,
{
    value(Default::default(), parser)
}
