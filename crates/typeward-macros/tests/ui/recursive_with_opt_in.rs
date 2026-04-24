use typeward::prelude::*;
use typeward_macros::Parse;

#[derive(Parse)]
#[parse(recursive)]
enum RecursiveValue {
    Number(Ws<i64>),
    Array(Vec<RecursiveValue>),
}

fn main() {
    let _ = RecursiveValue::parse("1");
}
