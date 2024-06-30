use serde::{Deserialize, Serialize};

pub mod persistent;
pub mod sql_json;

#[derive(Debug)]
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Persistent,
    Ephemeral,
    Countdown
}

#[derive(Debug)]
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Head,
    Options,
    Patch,
    Put,
    Delete
}