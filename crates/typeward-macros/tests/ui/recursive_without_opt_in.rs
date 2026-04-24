use typeward::prelude::*;
use typeward_macros::Parse;

#[derive(Parse)]
enum RecursiveValue {
    Number(Ws<i64>),
    Array(Vec<RecursiveValue>),
}

fn main() {}
