use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub enum Keyword {
    #[serde(rename = "==")]
    Eq,
    #[serde(rename = "!=")]
    NotEq,
    #[serde(rename = "<")]
    Less,
    #[serde(rename = "<=")]
    Lte,
    #[serde(rename = ">")]
    Greater,
    #[serde(rename = ">=")]
    Gte,
    #[serde(rename = "~=")]
    Rx,
    #[serde(rename = "^")]
    StartsWith
}