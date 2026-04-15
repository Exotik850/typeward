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
