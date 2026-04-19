use std::borrow::Cow;

use crate::input::BorrowInput;

use super::{Input, ReadInput};

#[test]
fn bytes_take_while() {
    let input: &[u8] = b"abc123";
    let (alpha, rest) = input.take_while(char::is_alphabetic).unwrap();
    assert_eq!(alpha, Cow::Borrowed("abc"));
    assert_eq!(rest, b"123");
}

#[test]
fn str_take_while() {
    let input = "abc123";
    let (alpha, rest) = input.take_while(char::is_alphabetic).unwrap();
    assert_eq!(alpha, Cow::Borrowed("abc"));
    assert_eq!(rest, "123");
}

#[test]
fn read_input_take_while() {
    let input = ReadInput::new(b"abc123");
    let (alpha, rest) = input.take_while(char::is_alphabetic).unwrap();
    assert_eq!(alpha, Cow::Borrowed("abc"));
    assert_eq!(rest.as_bytes(), b"123");
}

#[test]
fn string_deref_supports_borrowed_take_while() {
    let input = String::from("abc123");
    let (matched, rest) = input.take_while_borrowed(char::is_alphabetic).unwrap();
    assert_eq!(matched, "abc");
    assert_eq!(rest, "123");
}
