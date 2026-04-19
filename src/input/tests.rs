use super::{Input, ReadInput};

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
fn read_input_take_while() {
    let input = ReadInput::new(b"abc123");
    let (alpha, rest) = input.take_while(char::is_alphabetic).unwrap();
    assert_eq!(alpha, "abc");
    assert_eq!(rest.as_bytes(), b"123");
}
