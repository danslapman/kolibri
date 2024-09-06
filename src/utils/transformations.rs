use regex::Regex;
use std::sync::LazyLock;

pub mod js;

static CODE_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"%\{(.+?)\}").unwrap());