use once_cell::sync::Lazy;
use regex::Regex;

pub mod js;

static CODE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"%\{(.+?)\}").unwrap());