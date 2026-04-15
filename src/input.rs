use crate::{error::ParseError, error::ParseResult};

/// Borrowed token-stream input wrapper.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct TokenStream<'a, T> {
    tokens: &'a [T],
}

impl<'a, T> Clone for TokenStream<'a, T> {
    fn clone(&self) -> Self {
        Self {
            tokens: self.tokens,
        }
    }
}

impl<'a, T> Copy for TokenStream<'a, T> {}

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
    fn is_empty(self) -> ParseResult<bool>;

    /// Returns the remaining input as a debug-friendly string.
    fn display(self) -> String;

    /// Attempts to strip a literal prefix and returns remaining input on success.
    fn strip_prefix(self, prefix: &str) -> ParseResult<Option<Self>>;

    /// Takes the first character and returns it with the remaining input.
    fn take_char(self) -> ParseResult<Option<(char, Self)>>;

    /// Consumes a maximal prefix matching `predicate`.
    ///
    /// Returns the matched prefix and remaining input. The matched prefix may be
    /// empty when no leading characters satisfy the predicate.
    fn take_while<F>(self, predicate: F) -> ParseResult<(&'a str, Self)>
    where
        F: FnMut(char) -> bool;
}

fn utf8<'a>(bytes: &'a [u8]) -> ParseResult<&'a str> {
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

    fn is_empty(self) -> ParseResult<bool> {
        Ok(self.is_empty())
    }

    fn display(self) -> String {
        self.to_string()
    }

    fn strip_prefix(self, prefix: &str) -> ParseResult<Option<Self>> {
        Ok(self.strip_prefix(prefix))
    }

    fn take_char(self) -> ParseResult<Option<(char, Self)>> {
        let mut chars = self.chars();
        match chars.next() {
            Some(ch) => Ok(Some((ch, chars.as_str()))),
            None => Ok(None),
        }
    }

    fn take_while<F>(self, mut predicate: F) -> ParseResult<(&'a str, Self)>
    where
        F: FnMut(char) -> bool,
    {
        let idx = self.find(|c| !predicate(c)).unwrap_or(self.len());
        Ok((&self[..idx], &self[idx..]))
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

    fn is_empty(self) -> ParseResult<bool> {
        Ok(self.is_empty())
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

    fn take_while<F>(self, mut predicate: F) -> ParseResult<(&'a str, Self)>
    where
        F: FnMut(char) -> bool,
    {
        let s = utf8(self)?;
        let idx = s.find(|c| !predicate(c)).unwrap_or(s.len());
        let consumed = s[..idx].len();
        Ok((&s[..idx], &self[consumed..]))
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

    fn is_empty(self) -> ParseResult<bool> {
        Ok(self.tokens.is_empty())
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
        if let Some(first) = self.tokens.first() {
            if first.as_ref() == prefix {
                return Ok(Some(Self::new(&self.tokens[1..])));
            }
        }
        Ok(None)
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

    fn take_while<F>(self, mut predicate: F) -> ParseResult<(&'a str, Self)>
    where
        F: FnMut(char) -> bool,
    {
        let Some(first) = self.tokens.first() else {
            return Ok(("", self));
        };

        let token = first.as_ref();
        let idx = token.find(|c| !predicate(c)).unwrap_or(token.len());
        if idx == 0 {
            return Ok(("", self));
        }
        if idx != token.len() {
            return Err(ParseError::custom(
                "cannot consume partial token from token stream input",
            ));
        }

        Ok((token, Self::new(&self.tokens[1..])))
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
