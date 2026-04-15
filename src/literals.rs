use crate::define_tokens;

define_tokens!(
    /// The `true` boolean literal token.
    KwTrue, "true";
    /// The `false` boolean literal token.
    KwFalse, "false";
    /// The `null` literal token.
    KwNull, "null";
    /// The `(` left parenthesis token.
    LParen, "(";
    /// The `)` right parenthesis token.
    RParen, ")";
    /// The `[` left bracket token.
    LBracket, "[";
    /// The `]` right bracket token.
    RBracket, "]";
    /// The `{` left brace token.
    LBrace, "{";
    /// The `}` right brace token.
    RBrace, "}";
    /// The `.` dot token.
    Dot, ".";
    /// The `,` comma token.
    Comma, ",";
    /// The `:` colon token.
    Colon, ":";
    /// The `;` semicolon token.
    Semi, ";";
    /// The `=` equals token.
    Eq, "=";
    /// The `+` plus token.
    Plus, "+";
    /// The `-` minus token.
    Minus, "-";
    /// The `*` asterisk token.
    Star, "*";
    /// The `/` slash token.
    Slash, "/";
    /// The `!` exclamation token.
    Bang, "!";
    /// The `&` ampersand token.
    Amp, "&";
    /// The `|` pipe token.
    Pipe, "|";
    /// The `<` less-than token.
    Lt, "<";
    /// The `>` greater-than token.
    Gt, ">";
);
