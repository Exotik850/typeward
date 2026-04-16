use std::{collections::BTreeMap, str::FromStr};
use typeward::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

impl JsonValue {
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            Self::Array(value) => Some(value.as_slice()),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_object(&self) -> Option<&BTreeMap<String, JsonValue>> {
        match self {
            Self::Object(value) => Some(value),
            _ => None,
        }
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.as_object()?.get(key)
    }
}

pub fn parse_json(input: &str) -> ParseResult<JsonValue> {
    parse_complete::<JsonValue>(input)
}

impl FromStr for JsonValue {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_json(s)
    }
}

type JsonString = DelimitedExact<Ws<DoubleQuote>, DoubleQuote, TakeTillToken<DoubleQuote>>;
type JsonMember = and!(JsonString, Ws<Colon>, JsonValue);
type JsonArray = Delimited<Ws<LBracket>, Ws<RBracket>, Separated0<JsonValue, Ws<Comma>>>;
type JsonObject = Delimited<Ws<LBrace>, Ws<RBrace>, Separated0<JsonMember, Ws<Comma>>>;
type JsonParser = or!(Ws<KwNull>, Ws<bool>, Ws<f64>, JsonString, JsonArray, JsonObject);

impl<'a> Parse<'a> for JsonValue {
    fn parse(input: &'a str) -> ParseResult<(Self, &'a str)> {
        let (parsed, rest) = JsonParser::parse(input)?;

        let value = or_match!(
            parsed,
            _null => JsonValue::Null,
            boolean => JsonValue::Bool(boolean.into_inner()),
            number => JsonValue::Number(number.into_inner()),
            string => JsonValue::String(string.inner.into_inner()),
            array => JsonValue::Array(array.inner.into_items()),
            object => {
                let mut map = BTreeMap::new();
                for member in object.inner.into_items() {
                    let (key, _colon, value) =
                        unpack_and!(member, JsonString, Ws<Colon>, JsonValue);
                    map.insert(key.inner.into_inner(), value);
                }
                JsonValue::Object(map)
            },
        );

        Ok((value, rest))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_scalar_values() {
        assert_eq!(parse_json("null").unwrap(), JsonValue::Null);
        assert_eq!(parse_json("true").unwrap(), JsonValue::Bool(true));
        assert_eq!(parse_json("-12.5e2").unwrap(), JsonValue::Number(-1250.0));
        assert_eq!(
            parse_json("\"hello world\"").unwrap(),
            JsonValue::String("hello world".to_string())
        );
    }

    #[test]
    fn parse_nested_json() {
        let value = parse_json(r#"{ "name": "typeward", "ok": true, "items": [1, 2, 3] }"#)
            .unwrap();

        assert_eq!(value.get("name").and_then(JsonValue::as_str), Some("typeward"));
        assert_eq!(value.get("ok").and_then(JsonValue::as_bool), Some(true));
        assert_eq!(
            value
                .get("items")
                .and_then(JsonValue::as_array)
                .map(|items| items.len()),
            Some(3)
        );
    }

    #[test]
    fn reject_missing_value_after_colon() {
        assert!(parse_json(r#"{ "x": }"#).is_err());
    }
}
