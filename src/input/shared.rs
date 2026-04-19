use crate::error::{ParseError, ParseResult};
use stable_pattern::{Pattern, Searcher};

pub(super) fn utf8(bytes: &[u8]) -> ParseResult<&str> {
    std::str::from_utf8(bytes)
        .map_err(|err| ParseError::fatal(format!("invalid UTF-8 input: {err}")))
}

pub(super) fn checked_consumed_len(
    input_len: usize,
    end_len: usize,
    input_name: &str,
) -> ParseResult<usize> {
    if end_len > input_len {
        return Err(ParseError::fatal(format!(
            "invalid input bounds while slicing {input_name} input"
        )));
    }

    Ok(input_len - end_len)
}

pub(super) fn take_while_prefix_len<P>(input: &str, predicate: P) -> usize
where
    P: for<'a> Pattern<'a> + Copy,
{
    let mut idx = 0;
    while let Some(rest) = predicate.strip_prefix_of(&input[idx..]) {
        idx += input[idx..].len() - rest.len();
    }
    idx
}

pub(super) fn split_take_till<'a, P>(input: &'a str, predicate: P) -> (&'a str, &'a str)
where
    P: for<'b> Pattern<'b> + Copy,
{
    predicate
        .into_searcher(input)
        .next_match()
        .map_or((input, ""), |(start, _)| input.split_at(start))
}
