use regex::Regex;
use std::sync::LazyLock;

pub mod js;
pub mod xml;

static CODE_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"%\{(.+?)\}").unwrap());