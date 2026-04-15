use crate::error::{ParseError, ParseResult, custom};
use stable_pattern::{Pattern, Searcher};

/// Borrowed token-stream input wrapper.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct TokenStream<'a, T> {
    tokens: &'a [T],
}

impl<T> Clone for TokenStream<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for TokenStream<'_, T> {}

impl<'a, T> TokenStream<'a, T> {
    #[must_use]
    pub fn new(tokens: &'a [T]) -> Self {
        Self { tokens }
    }

    #[must_use]
    pub fn as_slice(self) -> &'a [T] {
        self.tokens
    }
}

impl<'a, T> From<&'a [T]> for TokenStream<'a, T> {
    fn from(value: &'a [T]) -> Self {
        Self::new(value)
    }
}

/// Abstract parser input that supports consuming textual prefixes.
///
/// This trait is implemented for common borrowed input forms:
/// - `&str`
/// - `&[u8]` (UTF-8)
/// - [`TokenStream`] for token sequences where `T: AsRef<str>`
pub trait Input<'a>: Copy + Sized {
    /// Returns the number of remaining units in the input.
    fn input_len(self) -> usize;

    /// Trims leading whitespace and returns the remaining input.
    fn trim_start(self) -> ParseResult<Self>;

    /// Returns true when no input remains.
    fn is_empty(self) -> bool;

    /// Returns the remaining input as a debug-friendly string.
    fn display(self) -> String;

    /// Attempts to strip a literal prefix and returns remaining input on success.
    fn strip_prefix(self, prefix: &str) -> ParseResult<Option<Self>>;

    /// Finds the next occurrence of `needle` and returns input starting at that match.
    fn find(self, needle: &str) -> ParseResult<Option<Self>>;

    /// Returns the input segment from `self` up to (but excluding) `end`.
    ///
    /// Both values must be suffixes of the same original input.
    fn slice_to(self, end: Self) -> ParseResult<Self>;

    /// Takes the first character and returns it with the remaining input.
    fn take_char(self) -> ParseResult<Option<(char, Self)>>;

    /// Consumes a maximal prefix matching `predicate`.
    ///
    /// Returns the matched prefix and remaining input. The matched prefix may be
    /// empty when no leading characters satisfy the predicate.
    fn take_while<P>(self, predicate: P) -> ParseResult<(&'a str, Self)>
    where
        P: Pattern<'a> + Copy;

    /// Consumes a prefix until `predicate` matches.
    /// 
    /// Returns the consumed prefix and remaining input. The consumed prefix may be
    /// empty when the predicate matches at the start of the input.
    fn take_till<P>(self, predicate: P) -> ParseResult<(&'a str, Self)>
    where
        P: Pattern<'a> + Copy;

    /// Returns an empty input value of the same type.
    fn empty() -> Self;
}

fn utf8(bytes: &[u8]) -> ParseResult<&str> {
    std::str::from_utf8(bytes)
        .map_err(|err| ParseError::custom(format!("invalid UTF-8 input: {err}")))
}

impl<'a> Input<'a> for &'a str {
    fn input_len(self) -> usize {
        self.len()
    }

    fn trim_start(self) -> ParseResult<Self> {
        Ok(self.trim_start())
    }

    fn is_empty(self) -> bool {
        self.is_empty()
    }

    fn display(self) -> String {
        self.to_string()
    }

    fn strip_prefix(self, prefix: &str) -> ParseResult<Option<Self>> {
        Ok(self.strip_prefix(prefix))
    }

    fn find(self, needle: &str) -> ParseResult<Option<Self>> {
        Ok(self.find(needle).map(|idx| &self[idx..]))
    }

    fn slice_to(self, end: Self) -> ParseResult<Self> {
        if end.len() > self.len() {
            return Err(ParseError::custom(
                "invalid input bounds while slicing string input",
            ));
        }

        let consumed = self.len() - end.len();
        Ok(&self[..consumed])
    }

    fn take_char(self) -> ParseResult<Option<(char, Self)>> {
        let mut chars = self.chars();
        match chars.next() {
            Some(ch) => Ok(Some((ch, chars.as_str()))),
            None => Ok(None),
        }
    }

    fn take_while<P>(self, predicate: P) -> ParseResult<(&'a str, Self)>
    where
        P: Pattern<'a> + Copy,
    {
        let mut idx = 0;
        while let Some(rest) = predicate.strip_prefix_of(&self[idx..]) {
            idx += self[idx..].len() - rest.len();
        }
        Ok((&self[..idx], &self[idx..]))
    }

    fn take_till<P>(self, predicate: P) -> ParseResult<(&'a str, Self)>
    where
        P: Pattern<'a> + Copy,
    {
        let out = predicate
            .into_searcher(self)
            .next_match()
            .map(|(start, _)| {
                let (matched, rest) = self.split_at(start);
                (matched, rest)
            })
            .unwrap_or((self, ""));
        Ok(out)
    }

    fn empty() -> Self {
        ""
    }
}

impl<'a> Input<'a> for &'a [u8] {
    fn input_len(self) -> usize {
        self.len()
    }

    fn trim_start(self) -> ParseResult<Self> {
        let end = self
            .iter()
            .position(|b| !b.is_ascii_whitespace())
            .unwrap_or(self.len());
        Ok(&self[end..])
    }

    fn is_empty(self) -> bool {
        self.is_empty()
    }

    fn display(self) -> String {
        String::from_utf8_lossy(self).into_owned()
    }

    fn strip_prefix(self, prefix: &str) -> ParseResult<Option<Self>> {
        let s = utf8(self)?;
        if let Some(rest) = s.strip_prefix(prefix) {
            let consumed = s.len() - rest.len();
            Ok(Some(&self[consumed..]))
        } else {
            Ok(None)
        }
    }

    fn find(self, needle: &str) -> ParseResult<Option<Self>> {
        let s = utf8(self)?;
        Ok(s.find(needle).map(|idx| &self[idx..]))
    }

    fn slice_to(self, end: Self) -> ParseResult<Self> {
        if end.len() > self.len() {
            return Err(ParseError::custom(
                "invalid input bounds while slicing byte input",
            ));
        }

        let consumed = self.len() - end.len();
        Ok(&self[..consumed])
    }

    fn take_char(self) -> ParseResult<Option<(char, Self)>> {
        let s = utf8(self)?;
        let mut chars = s.chars();
        if let Some(ch) = chars.next() {
            let consumed = ch.len_utf8();
            Ok(Some((ch, &self[consumed..])))
        } else {
            Ok(None)
        }
    }

    fn take_while<P>(self, predicate: P) -> ParseResult<(&'a str, Self)>
    where
        P: Pattern<'a> + Copy,
    {
        let s = utf8(self)?;
        let mut idx = 0;
        while let Some(rest) = predicate.strip_prefix_of(&s[idx..]) {
            idx += s[idx..].len() - rest.len();
        }
        Ok((&s[..idx], &self[idx..]))
    }

    fn take_till<P>(self, predicate: P) -> ParseResult<(&'a str, Self)>
    where
        P: Pattern<'a> + Copy,
    {
        let s = utf8(self)?;
        let out = predicate
            .into_searcher(s)
            .next_match()
            .map(|(start, _)| {
                let (matched, rest) = s.split_at(start);
                (matched, rest.as_bytes())
            })
            .unwrap_or((s, &[]));
        Ok(out)
    }

    fn empty() -> Self {
        &[]
    }
}

impl<'a, T> Input<'a> for TokenStream<'a, T>
where
    T: AsRef<str>,
{
    fn input_len(self) -> usize {
        self.tokens.len()
    }

    fn trim_start(self) -> ParseResult<Self> {
        let mut idx = 0;
        while idx < self.tokens.len() && self.tokens[idx].as_ref().trim_start().is_empty() {
            idx += 1;
        }
        Ok(Self::new(&self.tokens[idx..]))
    }

    fn is_empty(self) -> bool {
        self.tokens.is_empty()
    }

    fn display(self) -> String {
        self.tokens
            .iter()
            .take(8)
            .map(AsRef::as_ref)
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn strip_prefix(self, prefix: &str) -> ParseResult<Option<Self>> {
        if let Some(first) = self.tokens.first()
            && first.as_ref() == prefix
        {
            return Ok(Some(Self::new(&self.tokens[1..])));
        }
        Ok(None)
    }

    fn find(self, needle: &str) -> ParseResult<Option<Self>> {
        Ok(self
            .tokens
            .iter()
            .position(|token| token.as_ref() == needle)
            .map(|idx| Self::new(&self.tokens[idx..])))
    }

    fn slice_to(self, end: Self) -> ParseResult<Self> {
        let start_slice = self.as_slice();
        let end_slice = end.as_slice();
        if end_slice.len() > start_slice.len() {
            return Err(ParseError::custom(
                "invalid input bounds while slicing token input",
            ));
        }

        let consumed = start_slice.len() - end_slice.len();
        Ok(Self::new(&start_slice[..consumed]))
    }

    fn take_char(self) -> ParseResult<Option<(char, Self)>> {
        let Some(first) = self.tokens.first() else {
            return Ok(None);
        };

        let token = first.as_ref();
        let mut chars = token.chars();
        let Some(ch) = chars.next() else {
            return Err(ParseError::custom(
                "encountered empty token in token stream",
            ));
        };
        if chars.next().is_some() {
            return Err(ParseError::custom(
                "cannot parse char from multi-character token stream element",
            ));
        }

        Ok(Some((ch, Self::new(&self.tokens[1..]))))
    }

    fn take_while<P>(self, predicate: P) -> ParseResult<(&'a str, Self)>
    where
        P: Pattern<'a> + Copy,
    {
        let Some(first) = self.tokens.first() else {
            return Ok(("", self));
        };

        let token = first.as_ref();
        let mut idx = 0;
        while let Some(rest) = predicate.strip_prefix_of(&token[idx..]) {
            idx += token[idx..].len() - rest.len();
        }
        if idx != token.len() {
            return Err(ParseError::custom(
                "cannot consume partial token from token stream input",
            ));
        }

        Ok((token, Self::new(&self.tokens[1..])))
    }

    fn take_till<P>(self, predicate: P) -> ParseResult<(&'a str, Self)>
    where
        P: Pattern<'a> + Copy,
    {
        let Some(first) = self.tokens.first() else {
            return Ok(("", self));
        };

        let token = first.as_ref();
        let out = predicate
            .into_searcher(token)
            .next_match()
            .map(|(start, _)| {
                let (matched, rest) = token.split_at(start);
                if !rest.is_empty() {
                    return Err(ParseError::custom(
                        "cannot consume partial token from token stream input",
                    ));
                }
                Ok(matched)
            })
            .unwrap_or(Ok(token))?;
        Ok((out, Self::new(&self.tokens[1..])))
    }

    fn empty() -> Self {
        Self::new(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::{Input, TokenStream};

    #[test]
    fn bytes_take_while() {
        let input: &[u8] = b"abc123";
        let (alpha, rest) = input.take_while(char::is_alphabetic).unwrap();
        assert_eq!(alpha, "abc");
        assert_eq!(rest, b"123");
    }

    #[test]
    fn str_take_while() {
        let input = "abc123";
        let (alpha, rest) = input.take_while(char::is_alphabetic).unwrap();
        assert_eq!(alpha, "abc");
        assert_eq!(rest, "123");
    }

    #[test]
    fn take_while_stream() {
        let input = ["abc", "def", "123"];
        let (alpha, rest) = TokenStream::new(&input)
            .take_while(char::is_alphabetic)
            .unwrap();
        assert_eq!(alpha, "abc");
        assert_eq!(rest.as_slice(), &["def", "123"]);
    }

    #[test]
    fn token_stream_take_while_full_token_only() {
        let input = ["hello", "world"];
        let (tok, rest) = TokenStream::new(&input)
            .take_while(char::is_alphabetic)
            .unwrap();
        assert_eq!(tok, "hello");
        assert_eq!(rest.as_slice(), &["world"]);
    }

    #[test]
    fn token_stream_rejects_partial_consumption() {
        let input = ["abc123"];
        let result = TokenStream::new(&input).take_while(char::is_alphabetic);
        assert!(result.is_err());
    }
}
