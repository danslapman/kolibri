use crate::utils::js::optic::{JsonOptic, ValueExt};
use regex::Captures;
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;
use sxd_document::parser as xml_parser;
use sxd_xpath::context::Context;
use sxd_xpath::Factory;

/// Matches `${json.path.to.field}` placeholders in XML body templates.
static JSON_SUBST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{([\p{L}\d\.\[\]\-_]+)\}").unwrap());

/// Matches `${{/xpath/expression}}` placeholders in XML body templates.
static XPATH_SUBST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{\{([^}]+)\}\}").unwrap());

/// Performs two-pass template substitution on an XML body string.
///
/// Pass 1: `${json.path}` is replaced with the value extracted from `data`.
/// Pass 2: `${{/xpath/expr}}` is replaced with the XPath result from `request_xml`.
///
/// Unresolved placeholders are left unchanged.
pub fn substitute_xml_template(template: &str, data: &Value, request_xml: Option<&str>) -> String {
    // Pass 1: JSON path substitution
    let after_json = JSON_SUBST.replace_all(template, |caps: &Captures| {
        let optic = JsonOptic::from_path(&caps[1]);
        data.get_all(&optic)
            .first()
            .map(|v| render_xml_value(v))
            .unwrap_or_else(|| caps[0].to_string())
    });

    // Pass 2: XPath substitution from request XML body
    let after_xpath = XPATH_SUBST.replace_all(&after_json, |caps: &Captures| {
        let xpath_str = &caps[1];
        request_xml
            .and_then(|xml| evaluate_xpath_to_string(xml, xpath_str).ok())
            .unwrap_or_else(|| caps[0].to_string())
    });

    after_xpath.into_owned()
}

fn render_xml_value(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

fn evaluate_xpath_to_string(xml: &str, xpath_str: &str) -> Result<String, String> {
    let package = xml_parser::parse(xml).map_err(|e| format!("{:?}", e))?;
    let document = package.as_document();
    let factory = Factory::new();
    let xpath = factory
        .build(xpath_str)
        .map_err(|e| format!("{:?}", e))?
        .ok_or_else(|| format!("Empty XPath: {}", xpath_str))?;
    let context = Context::new();
    let value = xpath
        .evaluate(&context, document.root())
        .map_err(|e| format!("{:?}", e))?;
    Ok(value.string())
}

#[cfg(test)]
mod xml_template_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn json_path_substitution() {
        let template = "<resp><name>${req.name}</name></resp>";
        let data = json!({"req": {"name": "Alice"}});
        let result = substitute_xml_template(template, &data, None);
        assert_eq!(result, "<resp><name>Alice</name></resp>");
    }

    #[test]
    fn xpath_substitution_from_request() {
        let template = "<resp><echo>${{/root/value}}</echo></resp>";
        let request_xml = "<root><value>hello</value></root>";
        let result = substitute_xml_template(template, &json!({}), Some(request_xml));
        assert_eq!(result, "<resp><echo>hello</echo></resp>");
    }

    #[test]
    fn unresolved_placeholder_left_unchanged() {
        let template = "<resp><x>${missing.field}</x></resp>";
        let result = substitute_xml_template(template, &json!({}), None);
        assert_eq!(result, "<resp><x>${missing.field}</x></resp>");
    }

    #[test]
    fn mixed_substitution() {
        let template = "<resp><a>${req.id}</a><b>${{/root/code}}</b></resp>";
        let data = json!({"req": {"id": 42}});
        let request_xml = "<root><code>XYZ</code></root>";
        let result = substitute_xml_template(template, &data, Some(request_xml));
        assert_eq!(result, "<resp><a>42</a><b>XYZ</b></resp>");
    }
}
