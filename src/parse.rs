use nom::{
    branch::alt,
    bytes::complete::{tag, take_till},
    character::complete::{anychar, satisfy},
    combinator::{map, opt, peek, value},
    multi::{fold_many0, fold_many1},
    sequence::tuple,
    IResult,
};

use crate::karma::{Karma, KarmaChange, KarmaStyle};

pub fn line(input: &str) -> IResult<&str, Vec<Karma>> {
    fold_many0(element, Vec::new(), |mut acc, elem| {
        if let Some(elem) = elem {
            acc.push(elem);
        }
        acc
    })(input)
}

fn element(input: &str) -> IResult<&str, Option<Karma>> {
    let karma = map(karma_change, Some);
    let anychar = value(None, anychar);
    alt((karma, anychar))(input)
}

fn karma_change(input: &str) -> IResult<&str, Karma> {
    alt((explicit_karma_change, implicit_karma_change))(input)
}

fn explicit_karma_change(input: &str) -> IResult<&str, Karma> {
    alt((explicit_karma_notice_change, explicit_karma_silent_change))(input)
}

fn explicit_karma_notice_change(input: &str) -> IResult<&str, Karma> {
    let (input, _) = tag("[")(input)?;
    let (input, term) = take_till(is_notice_char)(input)?;
    let (input, _) = tag("]")(input)?;
    let (input, change) = modifier(input)?;
    Ok((
        input,
        Karma {
            term: term.to_owned(),
            change,
            style: KarmaStyle::Notify,
        },
    ))
}

fn explicit_karma_silent_change(input: &str) -> IResult<&str, Karma> {
    let (input, _) = tag("(")(input)?;
    let (input, term) = take_till(is_silent_char)(input)?;
    let (input, _) = tag(")")(input)?;
    let (input, change) = modifier(input)?;
    Ok((
        input,
        Karma {
            term: term.to_owned(),
            change,
            style: KarmaStyle::Notify,
        },
    ))
}

fn implicit_karma_change(input: &str) -> IResult<&str, Karma> {
    let (input, term) = implicit_chars(input)?;
    let (input, change) = modifier(input)?;
    Ok((
        input,
        Karma {
            term,
            change,
            style: KarmaStyle::Notify,
        },
    ))
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

fn modifier(input: &str) -> IResult<&str, KarmaChange> {
    let (input, modifier) = alt((tag("++"), tag("--")))(input)?;
    let (input, _) = word_boundary(input)?;

    let change = match modifier {
        "++" => KarmaChange { up: 1, down: 0 },
        "--" => KarmaChange { up: 0, down: 1 },
        _ => unreachable!(),
    };
    Ok((input, change))
}

fn word_boundary(input: &str) -> IResult<&str, ()> {
    let (input, _) = peek(satisfy(|c| c.is_whitespace() || ",.;:)".contains(c)))(input)?;
    Ok((input, ()))
}
