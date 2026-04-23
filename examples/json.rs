use std::collections::BTreeMap;
use typeward::prelude::*;

type JsonString = DelimitedExact<Ws<DoubleQuote>, DoubleQuote, TakeTillToken<DoubleQuote>>;
type JsonMember = and!(JsonString, Ws<Colon>, JsonValue);
type JsonObject = Delimited<Ws<LBrace>, Ws<RBrace>, Separated0<JsonMember, Ws<Comma>>>;

#[derive(Debug, Clone, PartialEq, Parse)]
pub enum JsonValue {
    Null(Ignore<Ws<KwNull>>),
    Bool(Ws<bool>),
    Number(Ws<f64>),
    String(JsonString),
    Array(Vec<JsonValue>),
    Object(
        #[parse(from(JsonObject, object_to_map))]
        BTreeMap<String, JsonValue>
    ),
}

fn object_to_map(object: JsonObject) -> BTreeMap<String, JsonValue> {
    let mut map = BTreeMap::new();
    for member in object.inner.into_items() {
        let (key, _colon, value) = unpack_and!(member, (JsonString, Ws<Colon>, JsonValue));
        map.insert(key.inner.into_inner(), value);
    }
    map
}

impl JsonValue {
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value.inner()),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value.inner()),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value.inner()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_scalar_values() {
        assert_eq!(parse_json("null").unwrap(), JsonValue::Null(Ignore::new()));
        assert_eq!(parse_json("true").unwrap(), JsonValue::Bool(Ws::new(true)));
        assert_eq!(parse_json("-12.5e2").unwrap(), JsonValue::Number(Ws::new(-1250.0)));
        assert_eq!(
            parse_json("\"hello world\"").unwrap(),
            JsonValue::String(JsonString::new("hello world".to_string()))
        );
    }

    #[test]
    fn parse_nested_json() {
        let value =
            parse_json(r#"{ "name": "typeward", "ok": true, "items": [1, 2, 3] }"#).unwrap();

        assert_eq!(
            value.get("name").and_then(JsonValue::as_str),
            Some("typeward")
        );
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

fn main() {
    let json_str = r#"
        {
            "name": "typeward",
            "version": 0.2,
            "features": ["derive", "arrays"],
            "nested": { "a": 1, "b": [true, false] }
        }
    "#;

    match parse_json(json_str) {
        Ok(json) => println!("Parsed JSON: {:#?}", json),
        Err(err) => eprintln!("Failed to parse JSON: {err}"),
    }
}
