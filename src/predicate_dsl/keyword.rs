use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub enum Keyword {
    #[serde(rename = "==")]
    Equals,
    #[serde(rename = "!=")]
    NotEq,
    #[serde(rename = ">")]
    Greater,
    #[serde(rename = ">=")]
    Gte,
    #[serde(rename = "<")]
    Less,
    #[serde(rename = "<=")]
    Lte,
    #[serde(rename = "~=")]
    Rx,
    #[serde(rename = "size")]
    Size,
    #[serde(rename = "exists")]
    Exists,
    #[serde(rename = "[_]")]
    In,
    #[serde(rename = "![_]")]
    NotIn,
    #[serde(rename = "&[_]")]
    AllIn
}