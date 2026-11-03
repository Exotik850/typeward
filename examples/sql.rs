//! A small SQL grammar built from typeward parser combinators.

use typeward::prelude::*;

typeward::define_tokens!(
    SelectToken, "SELECT";
    FromToken, "FROM";
    WhereToken, "WHERE";
    OrderToken, "ORDER";
    ByToken, "BY";
    LimitToken, "LIMIT";
    AscToken, "ASC";
    DescToken, "DESC";
    SingleQuote, "'";
);

type SelectKeyword = Ws<Kw<SelectToken>>;
type FromKeyword = Ws<Kw<FromToken>>;
type WhereKeyword = Ws<Kw<WhereToken>>;
type OrderKeyword = Ws<Kw<OrderToken>>;
type ByKeyword = Ws<Kw<ByToken>>;
type LimitKeyword = Ws<Kw<LimitToken>>;
type AscKeyword = Ws<Kw<AscToken>>;
type DescKeyword = Ws<Kw<DescToken>>;
type SqlIdentifier = Ws<Identifier>;
type SqlString = DelimitedExact<Ws<SingleQuote>, Ws<SingleQuote>, TakeTillToken<SingleQuote>>;

fn identifier_to_string(identifier: SqlIdentifier) -> String {
    identifier.into_inner().value
}

fn string_to_string(value: SqlString) -> String {
    value.into_inner().into_inner()
}

#[derive(Debug, Clone, PartialEq, Parse)]
pub enum SqlValue {
    Null(Ignore<Ws<KwNull>>),
    Boolean(#[parse(from(Ws<KwTrue>, |_| true))] bool),
    BooleanFalse(#[parse(from(Ws<KwFalse>, |_| false))] bool),
    Number(#[parse(ws)] i64),
    String(#[parse(from(SqlString, string_to_string))] String),
}

#[derive(Debug, Clone, PartialEq, Parse)]
pub enum SelectItem {
    Wildcard(Ignore<Ws<Star>>),
    Column(#[parse(from(SqlIdentifier, identifier_to_string))] String),
}

#[derive(Debug, Clone, PartialEq, Parse)]
pub struct WhereClause {
    _where: Ignore<WhereKeyword>,
    #[parse(from(SqlIdentifier, identifier_to_string))]
    pub column: String,
    _equals: Ignore<Ws<Eq>>,
    pub value: SqlValue,
}

#[derive(Debug, Clone, PartialEq, Parse)]
pub enum OrderDirection {
    Asc(Ignore<AscKeyword>),
    Desc(Ignore<DescKeyword>),
}

#[derive(Debug, Clone, PartialEq, Parse)]
pub struct OrderClause {
    _order: Ignore<OrderKeyword>,
    _by: Ignore<ByKeyword>,
    #[parse(from(SqlIdentifier, identifier_to_string))]
    pub column: String,
    pub direction: Option<OrderDirection>,
}

#[derive(Debug, Clone, PartialEq, Parse)]
pub struct LimitClause {
    _limit: Ignore<LimitKeyword>,
    #[parse(ws)]
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Parse)]
pub struct SqlQuery {
    _select: Ignore<SelectKeyword>,
    pub projection: Separated<SelectItem, Ws<Comma>>,
    _from: Ignore<FromKeyword>,
    #[parse(from(SqlIdentifier, identifier_to_string))]
    pub table: String,
    pub where_clause: Option<WhereClause>,
    pub order_by: Option<OrderClause>,
    pub limit: Option<LimitClause>,
    _semicolon: Ignore<Option<Ws<Semi>>>,
}

pub fn parse_sql(input: &str) -> ParseResult<SqlQuery> {
    parse_complete::<SqlQuery>(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_select_query() {
        let query = parse_sql(
            "SELECT id, name FROM users WHERE active = true ORDER BY name DESC LIMIT 10;",
        )
        .unwrap();

        assert_eq!(query.table, "users");
        assert_eq!(
            query.projection.items,
            vec![
                SelectItem::Column("id".into()),
                SelectItem::Column("name".into()),
            ]
        );
        assert_eq!(query.where_clause.unwrap().column, "active");
        assert_eq!(query.order_by.unwrap().column, "name");
        assert_eq!(query.limit.unwrap().count, 10);
    }

    #[test]
    fn rejects_query_without_table() {
        assert!(parse_sql("SELECT * FROM").is_err());
    }
}

fn main() {
    let query = "SELECT id, name FROM users WHERE active = true ORDER BY name DESC LIMIT 10;";

    match parse_sql(query) {
        Ok(query) => println!("Parsed SQL: {query:#?}"),
        Err(error) => eprintln!("Failed to parse SQL: {error}"),
    }
}
