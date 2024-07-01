use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

#[derive(Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
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

impl Debug for Keyword {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Equals => write!(f, "=="),
            Self::NotEq => write!(f, "!="),
            Self::Greater => write!(f, ">"),
            Self::Gte => write!(f, ">="),
            Self::Less => write!(f, "<"),
            Self::Lte => write!(f, "<="),
            Self::Rx => write!(f, "~="),
            Self::Size => write!(f, "size"),
            Self::Exists => write!(f, "exists"),
            Self::In => write!(f, "[_]"),
            Self::NotIn => write!(f, "![_]"),
            Self::AllIn => write!(f, "&[_]"),
        }
    }
}

impl Display for Keyword {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Equals => write!(f, "=="),
            Self::NotEq => write!(f, "!="),
            Self::Greater => write!(f, ">"),
            Self::Gte => write!(f, ">="),
            Self::Less => write!(f, "<"),
            Self::Lte => write!(f, "<="),
            Self::Rx => write!(f, "~="),
            Self::Size => write!(f, "size"),
            Self::Exists => write!(f, "exists"),
            Self::In => write!(f, "[_]"),
            Self::NotIn => write!(f, "![_]"),
            Self::AllIn => write!(f, "&[_]"),
        }
    }
}