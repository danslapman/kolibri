use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

#[derive(Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub enum XmlKeyword {
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
    /// value: {"==": "text"} or {"~=": "regex"}
    #[serde(rename = "cdata")]
    Cdata,
    /// value: a JsonPredicate spec applied to the CDATA text parsed as JSON
    #[serde(rename = "jcdata")]
    JCdata,
}

impl Debug for XmlKeyword {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Equals  => write!(f, "=="),
            Self::NotEq   => write!(f, "!="),
            Self::Greater => write!(f, ">"),
            Self::Gte     => write!(f, ">="),
            Self::Less    => write!(f, "<"),
            Self::Lte     => write!(f, "<="),
            Self::Rx      => write!(f, "~="),
            Self::Size    => write!(f, "size"),
            Self::Exists  => write!(f, "exists"),
            Self::Cdata   => write!(f, "cdata"),
            Self::JCdata  => write!(f, "jcdata"),
        }
    }
}

impl Display for XmlKeyword {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}
