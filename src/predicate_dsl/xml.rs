use crate::predicate_dsl::json::JsonPredicate;
use crate::predicate_dsl::xml_keyword::XmlKeyword;
use crate::utils::IntoBD;
use bigdecimal::BigDecimal;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Error;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;
use sxd_document::parser as xml_parser;
use sxd_xpath::context::Context;
use sxd_xpath::{Factory, Value as XPathValue};

type Spec = HashMap<String, HashMap<XmlKeyword, Value>>;

#[derive(Clone)]
pub struct XmlPredicate {
    definition: Spec,
}

impl XmlPredicate {
    pub fn from_spec(spec: Spec) -> XmlPredicate {
        XmlPredicate { definition: spec }
    }

    /// Validates the given XML text against this predicate.
    /// Returns `Ok(false)` if the XML is malformed or any condition fails.
    pub fn validate(&self, xml_text: &str) -> Result<bool, XmlPredicateError> {
        let package = xml_parser::parse(xml_text)
            .map_err(|e| XmlPredicateError::ParseError(format!("{:?}", e)))?;
        let document = package.as_document();
        let factory = Factory::new();

        for (xpath_str, conditions) in &self.definition {
            let xpath = factory
                .build(xpath_str)
                .map_err(|e| XmlPredicateError::XPathBuildError(format!("{:?}", e)))?
                .ok_or_else(|| {
                    XmlPredicateError::XPathBuildError(format!("Empty XPath expression: {}", xpath_str))
                })?;

            let context = Context::new();
            let value = xpath
                .evaluate(&context, document.root())
                .map_err(|e| XmlPredicateError::XPathEvalError(format!("{:?}", e)))?;

            for (kwd, spec) in conditions {
                if !validate_one(kwd, spec, &value)? {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }
}

fn xpath_string_value(value: &XPathValue<'_>) -> String {
    value.string()
}

fn xpath_is_empty(value: &XPathValue<'_>) -> bool {
    match value {
        XPathValue::Nodeset(ns) => ns.size() == 0,
        _ => false,
    }
}

fn xpath_to_bigdecimal(value: &XPathValue<'_>) -> Option<BigDecimal> {
    match value {
        XPathValue::Number(n) => BigDecimal::from_str(&n.to_string()).ok(),
        _ => BigDecimal::from_str(value.string().trim()).ok(),
    }
}

fn json_value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => "null".to_string(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

fn validate_one(
    kwd: &XmlKeyword,
    spec: &Value,
    value: &XPathValue<'_>,
) -> Result<bool, XmlPredicateError> {
    match kwd {
        XmlKeyword::Equals => {
            let text = xpath_string_value(value);
            Ok(text == json_value_to_string(spec))
        }
        XmlKeyword::NotEq => {
            let text = xpath_string_value(value);
            Ok(text != json_value_to_string(spec))
        }
        XmlKeyword::Greater => {
            if let Value::Number(bound) = spec {
                let b = bound.to_big_decimal();
                Ok(xpath_to_bigdecimal(value).map_or(false, |v| v > b))
            } else {
                Err(XmlPredicateError::InvalidSpec(format!("'>' requires a number argument, got {:?}", spec)))
            }
        }
        XmlKeyword::Gte => {
            if let Value::Number(bound) = spec {
                let b = bound.to_big_decimal();
                Ok(xpath_to_bigdecimal(value).map_or(false, |v| v >= b))
            } else {
                Err(XmlPredicateError::InvalidSpec(format!("'>=' requires a number argument, got {:?}", spec)))
            }
        }
        XmlKeyword::Less => {
            if let Value::Number(bound) = spec {
                let b = bound.to_big_decimal();
                Ok(xpath_to_bigdecimal(value).map_or(false, |v| v < b))
            } else {
                Err(XmlPredicateError::InvalidSpec(format!("'<' requires a number argument, got {:?}", spec)))
            }
        }
        XmlKeyword::Lte => {
            if let Value::Number(bound) = spec {
                let b = bound.to_big_decimal();
                Ok(xpath_to_bigdecimal(value).map_or(false, |v| v <= b))
            } else {
                Err(XmlPredicateError::InvalidSpec(format!("'<=' requires a number argument, got {:?}", spec)))
            }
        }
        XmlKeyword::Rx => {
            if let Value::String(pattern) = spec {
                let text = xpath_string_value(value);
                match Regex::new(&format!("^(?:{})$", pattern)) {
                    Ok(rx) => Ok(rx.is_match(&text)),
                    Err(_) => Err(XmlPredicateError::InvalidSpec(format!("Invalid regex: {}", pattern))),
                }
            } else {
                Err(XmlPredicateError::InvalidSpec(format!("'~=' requires a string (regex) argument, got {:?}", spec)))
            }
        }
        XmlKeyword::Size => {
            if let Value::Number(n) = spec {
                let expected = n.as_u64().unwrap_or(0) as usize;
                let text = xpath_string_value(value);
                Ok(text.len() == expected)
            } else {
                Err(XmlPredicateError::InvalidSpec(format!("'size' requires a number argument, got {:?}", spec)))
            }
        }
        XmlKeyword::Exists => match spec {
            Value::Bool(true) => Ok(!xpath_is_empty(value)),
            Value::Bool(false) => Ok(xpath_is_empty(value)),
            _ => Err(XmlPredicateError::InvalidSpec(format!("'exists' requires a boolean argument, got {:?}", spec))),
        },
        XmlKeyword::Cdata => {
            if let Value::Object(map) = spec {
                let text = xpath_string_value(value);
                if let Some(Value::String(expected)) = map.get("==") {
                    return Ok(&text == expected);
                }
                if let Some(Value::String(pattern)) = map.get("~=") {
                    return match Regex::new(&format!("^(?:{})$", pattern)) {
                        Ok(rx) => Ok(rx.is_match(&text)),
                        Err(_) => Err(XmlPredicateError::InvalidSpec(format!("Invalid regex in cdata: {}", pattern))),
                    };
                }
            }
            Err(XmlPredicateError::InvalidSpec(format!(
                "'cdata' requires {{\"==\": \"..\"}} or {{\"~=\": \"..\"}} argument, got {:?}",
                spec
            )))
        }
        XmlKeyword::JCdata => {
            let predicate = serde_json::from_value::<JsonPredicate>(spec.clone())
                .map_err(|e| XmlPredicateError::InvalidSpec(format!("'jcdata' spec is not a valid JsonPredicate: {}", e)))?;
            let text = xpath_string_value(value);
            let json_val: Value = serde_json::from_str(text.trim())
                .map_err(|_| XmlPredicateError::ParseError("CDATA content is not valid JSON".to_string()))?;
            Ok(predicate.validate(json_val).unwrap_or(false))
        }
    }
}

fn validate_condition(kwd: &XmlKeyword, spec: &Value) -> bool {
    match (kwd, spec) {
        (XmlKeyword::Equals | XmlKeyword::NotEq | XmlKeyword::Rx | XmlKeyword::Exists, _) => true,
        (XmlKeyword::Greater | XmlKeyword::Gte | XmlKeyword::Less | XmlKeyword::Lte | XmlKeyword::Size, Value::Number(_)) => true,
        (XmlKeyword::Cdata, Value::Object(map)) => {
            (map.contains_key("==") || map.contains_key("~=")) && map.len() == 1
        }
        (XmlKeyword::JCdata, Value::Object(_)) => {
            serde_json::from_value::<JsonPredicate>(spec.clone()).is_ok()
        }
        _ => false,
    }
}

impl Serialize for XmlPredicate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.definition.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for XmlPredicate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let spec = Spec::deserialize(deserializer)?;

        let mut faulty_fields: Vec<String> = vec![];

        for (xpath, cond) in spec.iter() {
            for (kwd, v) in cond.iter() {
                if !validate_condition(kwd, v) {
                    faulty_fields.push(xpath.clone());
                }
            }
        }

        if !faulty_fields.is_empty() {
            Err(D::Error::custom(format!(
                "XML predicate conditions are faulty on paths: {}",
                faulty_fields.join(", ")
            )))
        } else {
            Ok(XmlPredicate { definition: spec })
        }
    }
}

impl Debug for XmlPredicate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(&self.definition).expect("Unserializable XmlPredicate!")
        )
    }
}

#[derive(Debug)]
pub enum XmlPredicateError {
    ParseError(String),
    XPathBuildError(String),
    XPathEvalError(String),
    InvalidSpec(String),
}

impl Display for XmlPredicateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            XmlPredicateError::ParseError(msg) => write!(f, "XML parse error: {}", msg),
            XmlPredicateError::XPathBuildError(msg) => write!(f, "XPath build error: {}", msg),
            XmlPredicateError::XPathEvalError(msg) => write!(f, "XPath evaluation error: {}", msg),
            XmlPredicateError::InvalidSpec(msg) => write!(f, "Invalid predicate spec: {}", msg),
        }
    }
}

#[cfg(test)]
mod xml_tests {
    use super::*;
    use serde_json::json;

    fn predicate_from(spec: Value) -> XmlPredicate {
        serde_json::from_value::<XmlPredicate>(spec).expect("Valid predicate spec")
    }

    #[test]
    fn xml_equals_string() {
        let pred = predicate_from(json!({"/root/name": {"==": "Alice"}}));
        assert!(pred.validate("<root><name>Alice</name></root>").unwrap());
        assert!(!pred.validate("<root><name>Bob</name></root>").unwrap());
    }

    #[test]
    fn xml_not_equals() {
        let pred = predicate_from(json!({"/root/name": {"!=": "Alice"}}));
        assert!(!pred.validate("<root><name>Alice</name></root>").unwrap());
        assert!(pred.validate("<root><name>Bob</name></root>").unwrap());
    }

    #[test]
    fn xml_greater_than() {
        let pred = predicate_from(json!({"/root/age": {">": 18}}));
        assert!(pred.validate("<root><age>19</age></root>").unwrap());
        assert!(!pred.validate("<root><age>18</age></root>").unwrap());
        assert!(!pred.validate("<root><age>17</age></root>").unwrap());
    }

    #[test]
    fn xml_less_than() {
        let pred = predicate_from(json!({"/root/score": {"<": 100}}));
        assert!(pred.validate("<root><score>99</score></root>").unwrap());
        assert!(!pred.validate("<root><score>100</score></root>").unwrap());
    }

    #[test]
    fn xml_regex_match() {
        let pred = predicate_from(json!({"/root/code": {"~=": r"\d{4}"}}));
        assert!(pred.validate("<root><code>1234</code></root>").unwrap());
        assert!(!pred.validate("<root><code>123</code></root>").unwrap());
        assert!(!pred.validate("<root><code>12345</code></root>").unwrap());
    }

    #[test]
    fn xml_exists_true() {
        let pred = predicate_from(json!({"/root/name": {"exists": true}}));
        assert!(pred.validate("<root><name>Alice</name></root>").unwrap());
        assert!(!pred.validate("<root></root>").unwrap());
    }

    #[test]
    fn xml_exists_false() {
        let pred = predicate_from(json!({"/root/name": {"exists": false}}));
        assert!(!pred.validate("<root><name>Alice</name></root>").unwrap());
        assert!(pred.validate("<root></root>").unwrap());
    }

    #[test]
    fn xml_multiple_conditions() {
        let pred = predicate_from(json!({
            "/root/name": {"==": "Alice"},
            "/root/age": {">": 18, "<=": 65}
        }));
        assert!(pred.validate("<root><name>Alice</name><age>30</age></root>").unwrap());
        assert!(!pred.validate("<root><name>Alice</name><age>17</age></root>").unwrap());
        assert!(!pred.validate("<root><name>Bob</name><age>30</age></root>").unwrap());
    }

    #[test]
    fn xml_cdata_equals() {
        let pred = predicate_from(json!({"/root/data": {"cdata": {"==": "hello world"}}}));
        assert!(pred.validate("<root><data><![CDATA[hello world]]></data></root>").unwrap());
        assert!(!pred.validate("<root><data><![CDATA[other]]></data></root>").unwrap());
    }

    #[test]
    fn xml_jcdata() {
        let pred = predicate_from(json!({
            "/root/payload": {
                "jcdata": {
                    "name": {"==": "Alice"}
                }
            }
        }));
        assert!(pred.validate(r#"<root><payload><![CDATA[{"name":"Alice"}]]></payload></root>"#).unwrap());
        assert!(!pred.validate(r#"<root><payload><![CDATA[{"name":"Bob"}]]></payload></root>"#).unwrap());
    }

    #[test]
    fn invalid_spec_rejected_at_deserialization() {
        let bad = json!({"/root/age": {">": "not-a-number"}});
        assert!(serde_json::from_value::<XmlPredicate>(bad).is_err());
    }
}
