//! Example: Parsing a simple `let` statement.
//!
//! Demonstrates how to combine token parsers and primitive parsers
//! to build a structured parser for a simple statement.
//!
//! Example input: `let x = 42;`

use crate::error::ParseResult;
use crate::parse::Parse;
use crate::token::ParseToken;

use super::tokens::{KwEq, KwLet, Semi};

/// A struct representing a simple let statement: `let <name> = <value>;`
#[derive(Debug, Default)]
pub struct LetStatement {
    /// The variable name being assigned.
    pub name: String,
    /// The integer value being assigned.
    pub value: i64,
}

impl<'a> Parse<'a> for LetStatement {
    fn parse(input: &'a str) -> ParseResult<(Self, &'a str)> {
        let input = input.trim_start();

        // Parse "let" keyword
        let (_, remaining) = KwLet::parse_token(input)?;

        // Skip whitespace
        let remaining = remaining.trim_start();

        // Parse identifier
        let (name, remaining) = String::parse(remaining)?;

        // Skip whitespace
        let remaining = remaining.trim_start();

        // Parse "="
        let (_, remaining) = KwEq::parse_token(remaining)?;

        // Skip whitespace
        let remaining = remaining.trim_start();

        // Parse value
        let (value, remaining) = i64::parse(remaining)?;

        // Skip whitespace
        let remaining = remaining.trim_start();

        // Parse ";"
        let (_, remaining) = Semi::parse_token(remaining)?;

        Ok((LetStatement { name, value }, remaining))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_complete;

    #[test]
    fn test_parse_let_statement() {
        let input = "let x = 42;";
        let stmt = parse_complete::<LetStatement>(input).unwrap();
        assert_eq!(stmt.name, "x");
        assert_eq!(stmt.value, 42);
    }

    #[test]
    fn test_parse_let_statement_with_trailing() {
        let input = "let x = 42; extra";
        let result = parse_complete::<LetStatement>(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_let_statement_with_whitespace() {
        let input = "  let   x   =   42  ;  ";
        let stmt = parse_complete::<LetStatement>(input).unwrap();
        assert_eq!(stmt.name, "x");
        assert_eq!(stmt.value, 42);
    }
}
