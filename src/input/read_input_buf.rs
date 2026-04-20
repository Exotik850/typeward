use std::borrow::Cow;
use std::io::{self, Read};

use super::{BorrowInput, Input, shared};
use crate::error::ParseResult;
use stable_pattern::Pattern;

/// Owned bytes read from any [`Read`] source.
///
/// Keep this value alive for as long as parsed values borrow from the input.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ReadInputBuf {
    bytes: Vec<u8>,
}

impl ReadInputBuf {
    /// Read all bytes from `reader` into an owned buffer.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced while reading from the source.
    pub fn from_read<R>(reader: R) -> io::Result<Self>
    where
        R: Read,
    {
        Self::with_capacity(reader, 8 * 1024)
    }

    /// Read all bytes from `reader` into an owned buffer, pre-allocating with the given capacity.
    ///
    /// # Errors
    ///
    /// Returns any I/O error produced while reading from the source.
    pub fn with_capacity<R>(mut reader: R, capacity: usize) -> io::Result<Self>
    where
        R: Read,
    {
        let mut bytes = Vec::with_capacity(capacity);
        let mut chunk = [0_u8; 8 * 1024];

        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => bytes.extend_from_slice(&chunk[..read]),
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                Err(err) => return Err(err),
            }
        }

        Ok(Self { bytes })
    }

    /// Returns this buffer as a parseable [`ReadInput`] view.
    #[must_use]
    pub fn as_input(&self) -> ReadInput<'_> {
        ReadInput::new(&self.bytes)
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl From<Vec<u8>> for ReadInputBuf {
    fn from(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

impl AsRef<[u8]> for ReadInputBuf {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

/// Borrowed, copyable parse input backed by bytes read from a [`Read`] source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReadInput<'a> {
    bytes: &'a [u8],
}

impl<'a> ReadInput<'a> {
    #[must_use]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    #[must_use]
    pub const fn as_bytes(self) -> &'a [u8] {
        self.bytes
    }
}

impl<'a> From<&'a [u8]> for ReadInput<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        Self::new(bytes)
    }
}

impl<'a> From<&'a ReadInputBuf> for ReadInput<'a> {
    fn from(input: &'a ReadInputBuf) -> Self {
        input.as_input()
    }
}

impl AsRef<[u8]> for ReadInput<'_> {
    fn as_ref(&self) -> &[u8] {
        self.bytes
    }
}

impl<'a> Input<'a> for ReadInput<'a> {
    fn input_len(self) -> usize {
        self.bytes.len()
    }

    fn trim_start(self) -> Self {
        let start = self
            .bytes
            .iter()
            .position(|byte| !byte.is_ascii_whitespace())
            .unwrap_or(self.bytes.len());
        Self::new(&self.bytes[start..])
    }

    fn is_empty(self) -> bool {
        self.bytes.is_empty()
    }

    fn display(self) -> Cow<'a, str> {
        String::from_utf8_lossy(self.bytes)
    }

    fn strip_prefix(self, prefix: &str) -> ParseResult<Option<Self>> {
        Ok(self.bytes.strip_prefix(prefix.as_bytes()).map(Self::new))
    }

    fn advance(self, bytes: usize) -> ParseResult<Self> {
        if bytes > self.bytes.len() {
            return Err(crate::error::ParseError::fatal(
                "invalid input bounds while advancing read input",
            ));
        }

        Ok(Self::new(&self.bytes[bytes..]))
    }

    fn find(self, needle: &str) -> ParseResult<Option<Self>> {
        let s = shared::utf8(self.bytes)?;
        Ok(s.find(needle).map(|idx| Self::new(&self.bytes[idx..])))
    }

    fn slice_to(self, end: Self) -> ParseResult<Self> {
        let consumed = shared::checked_consumed_len(self.bytes.len(), end.bytes.len(), "read")?;
        Ok(Self::new(&self.bytes[..consumed]))
    }

    fn take_char(self) -> ParseResult<Option<(char, Self)>> {
        let s = shared::utf8(self.bytes)?;
        let mut chars = s.chars();
        Ok(chars
            .next()
            .map(|ch| (ch, Self::new(&self.bytes[ch.len_utf8()..]))))
    }

    fn take_while<P>(self, predicate: P) -> ParseResult<(Cow<'a, str>, Self)>
    where
        P: for<'b> Pattern<'b> + Copy,
    {
        let (matched, rest) = self.take_while_borrowed(predicate)?;
        Ok((Cow::Borrowed(matched), rest))
    }

    fn take_till<P>(self, predicate: P) -> ParseResult<(Cow<'a, str>, Self)>
    where
        P: for<'b> Pattern<'b> + Copy,
    {
        let (matched, rest) = self.take_till_borrowed(predicate)?;
        Ok((Cow::Borrowed(matched), rest))
    }
}

impl<'a> BorrowInput<'a> for ReadInput<'a> {
    fn take_while_borrowed<P>(self, predicate: P) -> ParseResult<(&'a str, Self)>
    where
        P: for<'b> Pattern<'b> + Copy,
    {
        let s = shared::utf8(self.bytes)?;
        let idx = shared::take_while_prefix_len(s, predicate);
        Ok((&s[..idx], Self::new(&self.bytes[idx..])))
    }

    fn take_till_borrowed<P>(self, predicate: P) -> ParseResult<(&'a str, Self)>
    where
        P: for<'b> Pattern<'b> + Copy,
    {
        let s = shared::utf8(self.bytes)?;
        let (matched, rest) = shared::split_take_till(s, predicate);
        let split = self.bytes.len() - rest.len();
        Ok((matched, Self::new(&self.bytes[split..])))
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, BufReader, Cursor, Read};

    use super::*;
    use crate::{
        input::Input,
        lit_token,
        parse::{parse_complete_input, parse_complete_input_spanned},
        prelude::Parenthesized,
    };

    lit_token!(HelloToken, "hello");

    struct InterruptedOnceReader<R> {
        inner: R,
        interrupted: bool,
    }

    impl<R> InterruptedOnceReader<R> {
        fn new(inner: R) -> Self {
            Self {
                inner,
                interrupted: false,
            }
        }
    }

    impl<R> Read for InterruptedOnceReader<R>
    where
        R: Read,
    {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if !self.interrupted {
                self.interrupted = true;
                return Err(io::Error::new(io::ErrorKind::Interrupted, "interrupted"));
            }
            self.inner.read(buf)
        }
    }

    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("boom"))
        }
    }

    #[test]
    fn read_input_buf_reads_all_bytes() {
        let input = ReadInputBuf::from_read(Cursor::new(b"hello world")).unwrap();
        assert_eq!(input.as_bytes(), b"hello world");
        assert_eq!(input.len(), 11);
        assert!(!input.is_empty());
    }

    #[test]
    fn read_input_buf_retries_interrupted_reads() {
        let reader = InterruptedOnceReader::new(Cursor::new(b"abc123"));
        let input = ReadInputBuf::from_read(reader).unwrap();
        assert_eq!(input.as_bytes(), b"abc123");
    }

    #[test]
    fn read_input_buf_propagates_hard_read_errors() {
        let err = ReadInputBuf::from_read(FailingReader).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Other);
    }

    #[test]
    fn read_input_take_while_matches_prefix() {
        let input = ReadInput::new(b"abc123");
        let (matched, remaining) = input.take_while(char::is_alphabetic).unwrap();
        assert_eq!(matched, "abc");
        assert_eq!(remaining.as_bytes(), b"123");
    }

    #[test]
    fn read_input_take_char_rejects_invalid_utf8() {
        let bytes = [0xFFu8];
        let err = ReadInput::new(&bytes).take_char().unwrap_err();
        assert!(err.to_string().contains("invalid UTF-8 input"));
    }

    #[test]
    fn parse_complete_input_supports_unbuffered_reader_data() {
        let input = ReadInputBuf::from_read(Cursor::new(b"hello")).unwrap();
        let _ = parse_complete_input::<_, HelloToken>(input.as_input()).unwrap();
    }

    #[test]
    fn parse_complete_input_supports_buffered_reader_data() {
        let buffered = BufReader::new(Cursor::new(b"hello"));
        let input = ReadInputBuf::from_read(buffered).unwrap();
        let _ = parse_complete_input::<_, HelloToken>(input.as_input()).unwrap();
    }

    #[test]
    fn parse_complete_input_reads_and_parses_data_larger_than_default_capacity() {
        let payload = format!("({})", "a".repeat((8 * 1024) + 257));
        let input = ReadInputBuf::from_read(Cursor::new(payload.as_bytes())).unwrap();
        assert_eq!(input.as_bytes(), payload.as_bytes());

        let parsed = parse_complete_input::<_, Parenthesized<String>>(input.as_input()).unwrap();
        assert_eq!(parsed.inner, &payload[1..payload.len() - 1]);
    }

    #[test]
    fn parse_complete_input_spanned_uses_byte_offsets_for_read_input() {
        let input = ReadInputBuf::from_read(Cursor::new("\u{00E9}".as_bytes())).unwrap();
        let result = parse_complete_input_spanned::<_, char>(input.as_input()).unwrap();
        assert_eq!(result.inner, '\u{00E9}');
        assert_eq!(result.span.start, 0);
        assert_eq!(result.span.end, 2);
    }
}
