# typeward

[![Crates.io](https://img.shields.io/crates/v/typeward.svg)](https://crates.io/crates/typeward)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE.md)
[![Rust Edition 2024](https://img.shields.io/badge/edition-2024-dea584)](https://doc.rust-lang.org/edition-guide/rust-2024/)

A parser combinator library that builds parsers by composing smaller parsers **within the type system**.

typeward is designed as a lightweight alternative to `syn` for general-purpose parsing. Define tokens as types, implement `Parse` for your structs and enums, and combine them using generic combinators — all with minimal boilerplate.

## Features

- **Type-level parser composition** — parsers are types, composed via generics rather than runtime closures
- **Derive macro** — `#[derive(Parse)]` generates parser implementations for structs, enums, and unions
- **Multiple input types** — works with `&str`, `&[u8]`, and read-backed `ReadInput` through a unified `Input` trait
- **Span support** — track source locations with `SourceSpan` for error reporting
- **Zero global state** — thread-safe parse context passed explicitly through parse calls

## Installation

```toml
[dependencies]
typeward = "0.2"
```

The `macros` feature (enabled by default) provides the `#[derive(Parse)]` macro. Disable it with `default-features = false` if you only need the combinator library.

## Quick Start

```rust
use typeward::prelude::*;

// Parse a complete input string
let (value, rest) = parse_complete::<f64>("3.14 remainder")?;
```

Import the prelude to get access to core traits, combinators, primitives, and the derive macro:

```rust
use typeward::prelude::*;
```

## Building Parsers with Types

Parsers are expressed as types and composed together:

```rust
// A quoted string: "..."
type QuotedString = DelimitedExact<DoubleQuote, DoubleQuote, TakeTillToken<DoubleQuote>>;

// A comma-separated list inside brackets: [a, b, c]
type List = Delimited<LBracket, RBracket, Separated0<String, Ws<Comma>>>;

// Try multiple alternatives
type Value = or!(Ws<KwNull>, Ws<bool>, Ws<f64>, QuotedString, List);

// Also implemented for `Option` and `Result`:
type OptionalValue = Option<Value>;
type FallibleValue = Result<Value, ParseError>;
```

### Implementing `Parse` for Custom Types

```rust
#[derive(Debug, Clone)]
struct Point {
    x: f64,
    y: f64,
}

impl<'a> Parse<'a> for Point {
    fn parse_with_context(
        input: &'a str,
        context: &mut ParseOffsetContext,
    ) -> ParseResult<(Self, &'a str)> {
        let (x, rest) = Ws::<f64>::parse_with_context(input, context)?;
        let (_, rest) = Ws::<Comma>::parse_with_context(rest, context)?;
        let (y, rest) = Ws::<f64>::parse_with_context(rest, context)?;
        Ok((Point { x, y }, rest))
    }
}
```

### Using the Derive Macro

Enable the `macros` feature and derive `Parse` automatically:

```rust
#[derive(Parse)]
struct Point {
    #[parse(ws)]
    x: f64,
    #[parse(ws)]
    y: f64,
}

#[derive(Parse)]
enum Shape {
    Circle { radius: f64 },
    Rect { width: f64, height: f64 },
}

#[derive(Parse)]
struct Wrapper<T>(T);
```

Field-level attributes like `#[parse(ws)]` and `#[parse(from(ParserType, mapper))]` customize how each field is parsed.

## Core Modules

| Module | Description |
|---|---|
| [`primitives`] | `Parse` implementations for built-in types (`bool`, `f64`, `i64`, `String`, etc.) |
| [`combinators`] | Parser combinators — `And`, `Or`, `Delimited`, `Separated`, `Ws`, `Span`, `Peek`, `Not` |
| [`collections`] | Repetition combinators — `Many0`, `Many1`, `Repeat` |
| [`input`] | Input abstraction — `&str`, `&[u8]`, `ReadInput` via the `Input` trait |
| [`error`] | Error types — `ParseError`, `ParseResult`, `SourceSpan` |
| [`parse`] | Core `Parse` trait and entry points like `parse_complete` |

## Macros

The crate provides helper macros for composing parsers ergonomically:

- `and!(A, B, C)` — compose a tuple parser from multiple types
- `or!(A, B, C)` — try parsers in sequence, returning the first match
- `or_match!(val, arm => expr, ...)` — destructure `Or` results into values
- `unpack_and!(val, (A, B, C))` — destructure `And` tuple results

## Example

See the [`examples/json.rs`](examples/json.rs) file for a complete JSON parser built with typeward.

## License

MIT — see [LICENSE.md](LICENSE.md)
