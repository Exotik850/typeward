use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::io::{IoSliceMut, Read};

use stable_pattern::Pattern;

use super::{Input, shared};
use crate::error::{ParseError, ParseResult};

/// A buffered streaming source that can produce copyable parse cursors.
///
/// This type owns the underlying reader and a fixed-size replay window of `N`
/// bytes. Call [`Self::as_input`]
/// to obtain a lightweight, copyable input cursor suitable for parser execution.
///
/// Matches produced from this input are always owned (`Cow::Owned`) so they do
/// not outlive mutable stream buffering.
#[derive(Debug)]
pub struct ReadInputStream<R, const N: usize> {
    reader: RefCell<R>,
    buf: RefCell<[u8; N]>,
    window_start: Cell<usize>,
    window_len: Cell<usize>,
    eof: Cell<bool>,
}

impl<R, const N: usize> ReadInputStream<R, N>
where
    R: Read,
{
    #[must_use]
    pub fn new(reader: R) -> Self {
        Self {
            reader: RefCell::new(reader),
            buf: RefCell::new([0_u8; N]),
            window_start: Cell::new(0),
            window_len: Cell::new(0),
            eof: Cell::new(false),
        }
    }

    #[must_use]
    pub fn as_input(&self) -> ReadInputStreamInput<'_, R, N> {
        ReadInputStreamInput {
            stream: self,
            start: 0,
            end: None,
        }
    }

    fn replay_error() -> ParseError {
        ParseError::fatal(format!(
            "stream backtracking exceeded the {N}-byte replay window"
        ))
    }

    fn ensure_non_zero_window() -> ParseResult<()> {
        if N == 0 {
            return Err(ParseError::fatal(
                "ReadInputStream requires a non-zero buffer size",
            ));
        }

        Ok(())
    }

    fn loaded_end(&self) -> usize {
        self.window_start
            .get()
            .saturating_add(self.window_len.get())
    }

    fn read_into_window(&self) -> ParseResult<usize> {
        Self::ensure_non_zero_window()?;

        let start = self.window_start.get();
        let len = self.window_len.get();
        let tail = (start + len) % N;
        let writable = if len < N { N - len } else { N };

        let mut reader = self.reader.borrow_mut();
        let mut buf = self.buf.borrow_mut();

        let first_len = writable.min(N - tail);
        let second_len = writable - first_len;

        let bytes_read = loop {
            let result = if second_len == 0 {
                reader.read(&mut buf[tail..tail + first_len])
            } else {
                let (left, right) = buf.split_at_mut(tail);
                let mut slices = [
                    IoSliceMut::new(&mut right[..first_len]),
                    IoSliceMut::new(&mut left[..second_len]),
                ];
                reader.read_vectored(&mut slices)
            };

            match result {
                Ok(read) => break read,
                Err(err) if err.kind() == std::io::ErrorKind::Interrupted => {}
                Err(err) => {
                    return Err(ParseError::fatal(format!("stream read error: {err}")));
                }
            }
        };

        if bytes_read == 0 {
            self.eof.set(true);
            return Ok(0);
        }

        if len < N {
            self.window_len.set(len + bytes_read);
        } else {
            self.window_start.set(start + bytes_read);
        }

        Ok(bytes_read)
    }

    fn ensure_loaded(&self, absolute: usize) -> ParseResult<bool> {
        if absolute < self.window_start.get() {
            return Err(Self::replay_error());
        }

        while absolute >= self.loaded_end() {
            if self.eof.get() {
                return Ok(false);
            }

            if self.read_into_window()? == 0 {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn byte_at(&self, absolute: usize) -> ParseResult<Option<u8>> {
        if !self.ensure_loaded(absolute)? {
            return Ok(None);
        }

        let start = self.window_start.get();
        let len = self.window_len.get();
        let rel = absolute.saturating_sub(start);
        if rel >= len {
            return Ok(None);
        }

        Ok(Some(self.buf.borrow()[absolute % N]))
    }

    fn drain_to_end(&self, from: usize) -> ParseResult<Vec<u8>> {
        if from < self.window_start.get() {
            return Err(Self::replay_error());
        }

        let mut out = Vec::new();
        let mut pos = from;
        while let Some(byte) = self.byte_at(pos)? {
            out.push(byte);
            pos = pos.saturating_add(1);
        }

        Ok(out)
    }

    fn collect_bounded(&self, start: usize, end: usize) -> ParseResult<Vec<u8>> {
        if start > end {
            return Err(ParseError::fatal(
                "invalid stream bounds while resolving input segment",
            ));
        }

        if start < self.window_start.get() {
            return Err(Self::replay_error());
        }

        let mut out = Vec::with_capacity(end - start);
        for pos in start..end {
            match self.byte_at(pos)? {
                Some(byte) => out.push(byte),
                None => {
                    return Err(ParseError::fatal(
                        "invalid stream bounds while resolving input segment",
                    ));
                }
            }
        }

        Ok(out)
    }
}

/// Copyable parse cursor over a [`ReadInputStream`].
///
/// The cursor tracks an absolute start offset and an optional exclusive end
/// bound. Bounds are used to represent `slice_to` results without borrowing.
#[derive(Debug)]
pub struct ReadInputStreamInput<'a, R, const N: usize> {
    stream: &'a ReadInputStream<R, N>,
    start: usize,
    end: Option<usize>,
}

impl<R, const N: usize> Clone for ReadInputStreamInput<'_, R, N> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R, const N: usize> Copy for ReadInputStreamInput<'_, R, N> {}

impl<R, const N: usize> ReadInputStreamInput<'_, R, N>
where
    R: Read,
{
    #[must_use]
    pub(crate) const fn absolute_start(self) -> usize {
        self.start
    }

    fn with_start(self, start: usize) -> Self {
        Self {
            stream: self.stream,
            start,
            end: self.end,
        }
    }

    fn bounded(self, end: usize) -> Self {
        Self {
            stream: self.stream,
            start: self.start,
            end: Some(end),
        }
    }

    fn utf8_char_width(first: u8) -> usize {
        if first < 0x80 {
            1
        } else if (first & 0b1110_0000) == 0b1100_0000 {
            2
        } else if (first & 0b1111_0000) == 0b1110_0000 {
            3
        } else if (first & 0b1111_1000) == 0b1111_0000 {
            4
        } else {
            0
        }
    }

    fn collect_segment_bytes(self) -> ParseResult<Vec<u8>> {
        match self.end {
            Some(end) => self.stream.collect_bounded(self.start, end),
            None => self.stream.drain_to_end(self.start),
        }
    }

    fn collect_segment_text(self) -> ParseResult<String> {
        let bytes = self.collect_segment_bytes()?;
        std::str::from_utf8(&bytes)
            .map(str::to_owned)
            .map_err(|err| ParseError::fatal(format!("invalid UTF-8 input: {err}")))
    }

    fn next_char_at(self, position: usize) -> ParseResult<Option<(char, usize)>> {
        if let Some(end) = self.end
            && position >= end
        {
            return Ok(None);
        }

        let Some(first) = self.stream.byte_at(position)? else {
            return Ok(None);
        };

        let width = Self::utf8_char_width(first);
        if width == 0 {
            return Err(ParseError::fatal(
                "invalid UTF-8 input: invalid leading byte",
            ));
        }

        if let Some(end) = self.end
            && position.saturating_add(width) > end
        {
            return Err(ParseError::fatal(
                "invalid UTF-8 input: unexpected end of bounded stream input",
            ));
        }

        let mut bytes = [0_u8; 4];
        bytes[0] = first;
        for (index, slot) in bytes.iter_mut().enumerate().take(width).skip(1) {
            let absolute = position.saturating_add(index);
            *slot = self.stream.byte_at(absolute)?.ok_or_else(|| {
                ParseError::fatal("invalid UTF-8 input: unexpected end of stream")
            })?;
        }

        let s = std::str::from_utf8(&bytes[..width])
            .map_err(|err| ParseError::fatal(format!("invalid UTF-8 input: {err}")))?;
        let ch = s
            .chars()
            .next()
            .expect("validated UTF-8 character should exist");
        Ok(Some((ch, width)))
    }
}

impl<'a, R, const N: usize> Input<'a> for ReadInputStreamInput<'a, R, N>
where
    R: Read,
{
    fn input_len(self) -> usize {
        if let Some(end) = self.end {
            return end.saturating_sub(self.start);
        }

        if self.stream.eof.get() {
            return self.stream.loaded_end().saturating_sub(self.start);
        }

        usize::MAX.saturating_sub(self.start)
    }

    fn trim_start(self) -> Self {
        if self.start < self.stream.window_start.get() {
            return self;
        }

        let mut cursor = self.start;

        loop {
            if let Some(end) = self.end
                && cursor >= end
            {
                break;
            }

            match self.stream.byte_at(cursor) {
                Ok(Some(byte)) if byte.is_ascii_whitespace() => cursor = cursor.saturating_add(1),
                _ => break,
            }
        }

        self.with_start(cursor)
    }

    fn is_empty(self) -> bool {
        if let Some(end) = self.end {
            return self.start >= end;
        }

        match self.stream.byte_at(self.start) {
            Ok(None) => true,
            Ok(Some(_)) | Err(_) => false,
        }
    }

    fn display(self) -> Cow<'a, str> {
        match self.collect_segment_bytes() {
            Ok(bytes) => Cow::Owned(String::from_utf8_lossy(&bytes).into_owned()),
            Err(err) => Cow::Owned(format!("<stream display unavailable: {err}>")),
        }
    }

    fn strip_prefix(self, prefix: &str) -> ParseResult<Option<Self>> {
        if self.start < self.stream.window_start.get() {
            return Err(ReadInputStream::<R, N>::replay_error());
        }

        let prefix_len = prefix.len();
        let end = self.start.saturating_add(prefix_len);

        if let Some(bound) = self.end
            && end > bound
        {
            return Ok(None);
        }

        for (index, expected) in prefix.as_bytes().iter().copied().enumerate() {
            let absolute = self.start.saturating_add(index);
            match self.stream.byte_at(absolute)? {
                Some(found) if found == expected => {}
                Some(_) | None => return Ok(None),
            }
        }

        Ok(Some(self.with_start(end)))
    }

    fn find(self, needle: &str) -> ParseResult<Option<Self>> {
        if self.start < self.stream.window_start.get() {
            return Err(ReadInputStream::<R, N>::replay_error());
        }

        if needle.is_empty() {
            return Ok(Some(self));
        }

        let needle = needle.as_bytes();
        let mut pos = self.start;

        loop {
            if let Some(bound) = self.end
                && pos.saturating_add(needle.len()) > bound
            {
                return Ok(None);
            }

            let mut matched = true;
            for (index, expected) in needle.iter().copied().enumerate() {
                let absolute = pos.saturating_add(index);
                match self.stream.byte_at(absolute)? {
                    Some(found) if found == expected => {}
                    Some(_) => {
                        matched = false;
                        break;
                    }
                    None => return Ok(None),
                }
            }

            if matched {
                return Ok(Some(self.with_start(pos)));
            }

            let Some((_, width)) = self.next_char_at(pos)? else {
                return Ok(None);
            };
            pos = pos.saturating_add(width);
        }
    }

    fn slice_to(self, end: Self) -> ParseResult<Self> {
        if !std::ptr::eq(self.stream, end.stream) || self.end != end.end || end.start < self.start {
            return Err(ParseError::fatal(
                "invalid stream bounds while slicing input",
            ));
        }

        if let Some(bound) = self.end
            && end.start > bound
        {
            return Err(ParseError::fatal(
                "invalid stream bounds while slicing input",
            ));
        }

        if self.start < self.stream.window_start.get() {
            return Err(ReadInputStream::<R, N>::replay_error());
        }

        if end.start < self.stream.window_start.get() {
            return Err(ReadInputStream::<R, N>::replay_error());
        }

        Ok(self.bounded(end.start))
    }

    fn take_char(self) -> ParseResult<Option<(char, Self)>> {
        if self.start < self.stream.window_start.get() {
            return Err(ReadInputStream::<R, N>::replay_error());
        }

        let Some((ch, width)) = self.next_char_at(self.start)? else {
            return Ok(None);
        };

        Ok(Some((
            ch,
            self.with_start(self.start.saturating_add(width)),
        )))
    }

    fn take_while<P>(self, predicate: P) -> ParseResult<(Cow<'a, str>, Self)>
    where
        P: for<'b> Pattern<'b> + Copy,
    {
        if self.start < self.stream.window_start.get() {
            return Err(ReadInputStream::<R, N>::replay_error());
        }

        let mut matched = String::new();
        let mut cursor = self.start;

        loop {
            let Some((ch, width)) = self.next_char_at(cursor)? else {
                break;
            };

            let mut encoded = [0_u8; 4];
            let ch_str = ch.encode_utf8(&mut encoded);
            if predicate.strip_prefix_of(ch_str).is_some() {
                matched.push(ch);
                cursor = cursor.saturating_add(width);
            } else {
                break;
            }
        }

        Ok((Cow::Owned(matched), self.with_start(cursor)))
    }

    fn take_till<P>(self, predicate: P) -> ParseResult<(Cow<'a, str>, Self)>
    where
        P: for<'b> Pattern<'b> + Copy,
    {
        if self.start < self.stream.window_start.get() {
            return Err(ReadInputStream::<R, N>::replay_error());
        }

        let text = self.collect_segment_text()?;
        let (matched, rest) = shared::split_take_till(&text, predicate);
        let split = text.len() - rest.len();
        Ok((
            Cow::Owned(matched.to_owned()),
            self.with_start(self.start.saturating_add(split)),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::cell::Cell;
    use std::io::{Cursor, Read};
    use std::rc::Rc;

    use super::*;
    use crate::{
        input::Input,
        lit_token,
        parse::{parse_complete_input, parse_complete_input_spanned},
        primitives::prelude::AlphaString,
    };

    lit_token!(HelloToken, "hello");

    #[test]
    fn stream_take_while_returns_owned_text() {
        let stream = ReadInputStream::<_, 2>::new(Cursor::new(b"abc123"));
        let input = stream.as_input();

        let (matched, rest) = input.take_while(char::is_alphabetic).unwrap();
        assert_eq!(matched, Cow::<str>::Owned("abc".to_string()));
        assert_eq!(rest.display(), Cow::<str>::Owned("123".to_string()));
    }

    #[test]
    fn stream_slice_to_supports_bounded_segments() {
        let stream = ReadInputStream::<_, 8>::new(Cursor::new(b"123abc"));
        let input = stream.as_input();

        let (_, rest) = input.take_while(|c: char| c.is_ascii_digit()).unwrap();
        let number_segment = input.slice_to(rest).unwrap();
        let (value, empty) = number_segment.take_while(|_: char| true).unwrap();

        assert_eq!(value, Cow::<str>::Owned("123".to_string()));
        assert!(empty.is_empty());
    }

    #[test]
    fn stream_parse_complete_supports_owned_parsers() {
        let stream = ReadInputStream::<_, 2>::new(Cursor::new(b"hello"));
        let parsed = parse_complete_input::<_, HelloToken>(stream.as_input()).unwrap();
        let _ = parsed;
    }

    #[test]
    fn stream_parse_complete_spanned_uses_byte_offsets() {
        let stream = ReadInputStream::<_, 2>::new(Cursor::new("\u{00E9}".as_bytes()));
        let result = parse_complete_input_spanned::<_, char>(stream.as_input()).unwrap();
        assert_eq!(result.inner, '\u{00E9}');
        assert_eq!(result.span.start, 0);
        assert_eq!(result.span.end, 2);
    }

    #[test]
    fn stream_works_outside_buffer_capacity() {
        let payload = "a".repeat(10);
        let stream = ReadInputStream::<_, 3>::new(Cursor::new(payload.as_bytes()));
        let input = stream.as_input();

        let (matched, rest) = input.take_while(|_: char| true).unwrap();
        assert_eq!(&matched, &payload);
        assert!(rest.is_empty());
    }

    #[test]
    fn stream_backtracking_beyond_window_fails() {
        let stream = ReadInputStream::<_, 3>::new(Cursor::new(b"123abc"));
        let input = stream.as_input();

        let (_, rest) = input.take_while(|c: char| c.is_ascii_digit()).unwrap();
        let err = input.slice_to(rest).unwrap_err();
        assert!(err.to_string().contains("replay window"));
    }

    #[test]
    fn stream_parses_owned_string_larger_than_buffer() {
        let payload = "abcdefghijklmnopqrstuvwxyz";
        let stream = ReadInputStream::<_, 4>::new(Cursor::new(payload.as_bytes()));
        let parsed = parse_complete_input::<_, String>(stream.as_input()).unwrap();
        assert_eq!(parsed, payload);
    }

    #[test]
    fn stream_parses_owned_alpha_string_larger_than_buffer() {
        let payload = "hellotypewardstreaming";
        let stream = ReadInputStream::<_, 5>::new(Cursor::new(payload.as_bytes()));
        let parsed = parse_complete_input::<_, AlphaString>(stream.as_input()).unwrap();
        assert_eq!(parsed.value, payload);
    }

    #[test]
    fn stream_parses_bool_across_buffer_boundary() {
        let stream = ReadInputStream::<_, 2>::new(Cursor::new(b"false"));
        let parsed = parse_complete_input::<_, bool>(stream.as_input()).unwrap();
        assert!(!parsed);
    }

    #[test]
    fn stream_parses_i64_larger_than_buffer() {
        let payload = "1234567890123456789";
        let stream = ReadInputStream::<_, 3>::new(Cursor::new(payload.as_bytes()));
        let parsed = parse_complete_input::<_, i64>(stream.as_input()).unwrap();
        assert_eq!(parsed, 1_234_567_890_123_456_789_i64);
    }

    #[test]
    fn stream_parses_f64_larger_than_buffer() {
        let payload = "-1234567890.12345e10";
        let stream = ReadInputStream::<_, 4>::new(Cursor::new(payload.as_bytes()));
        let parsed = parse_complete_input::<_, f64>(stream.as_input()).unwrap();
        let expected: f64 = payload.parse().unwrap();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn stream_parses_multibyte_char_with_tiny_buffer() {
        let stream = ReadInputStream::<_, 1>::new(Cursor::new("é".as_bytes()));
        let parsed = parse_complete_input::<_, char>(stream.as_input()).unwrap();
        assert_eq!(parsed, 'é');
    }

    #[test]
    fn stream_take_till_owned_value_larger_than_buffer() {
        lit_token!(Sentinel, "\0");

        let payload = "segmentbeforepipe";
        let stream = ReadInputStream::<_, 4>::new(Cursor::new(payload.as_bytes()));
        let parsed = parse_complete_input::<_, crate::primitives::str::TakeTillToken<Sentinel>>(
            stream.as_input(),
        )
        .unwrap();

        assert_eq!(parsed.into_inner(), payload);
    }

    struct CountingReader {
        inner: Cursor<Vec<u8>>,
        calls: Rc<Cell<usize>>,
    }

    impl CountingReader {
        fn new(bytes: Vec<u8>, calls: Rc<Cell<usize>>) -> Self {
            Self {
                inner: Cursor::new(bytes),
                calls,
            }
        }
    }

    impl Read for CountingReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.calls.set(self.calls.get() + 1);
            self.inner.read(buf)
        }
    }

    #[test]
    fn stream_reads_in_chunks_instead_of_per_byte() {
        const WINDOW: usize = 8;

        let expected = "x".repeat(257);
        let calls = Rc::new(Cell::new(0));
        let reader = CountingReader::new(expected.as_bytes().to_vec(), Rc::clone(&calls));
        let stream = ReadInputStream::<_, WINDOW>::new(reader);

        let parsed = parse_complete_input::<_, String>(stream.as_input()).unwrap();
        assert_eq!(parsed, expected);

        let expected_max_calls = expected.len().div_ceil(WINDOW) + 1;
        assert!(
            calls.get() <= expected_max_calls,
            "expected <= {expected_max_calls} read calls, got {}",
            calls.get()
        );
    }
}
