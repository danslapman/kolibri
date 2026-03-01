use crate::api::model::RequestBody;
use crate::misc::Substitute;
use crate::model::*;
use crate::predicate_dsl::json::JsonPredicate;
use crate::predicate_dsl::keyword::Keyword;
use crate::predicate_dsl::xml::XmlPredicate;
use crate::utils::js::optic::JsonOptic;
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "mode")]
pub enum HttpStubRequest {
    #[serde(rename = "no_body")]
    RequestWithoutBody {
        headers: HashMap<String, String>,
        #[serde(default = "HashMap::new")]
        query: HashMap<JsonOptic, HashMap<Keyword, Value>>
    },
    #[serde(rename = "json")]
    JsonRequest {
        headers: HashMap<String, String>,
        #[serde(default = "HashMap::new")]
        query: HashMap<JsonOptic, HashMap<Keyword, Value>>,
        body: Value
    },
    #[serde(rename = "raw")]
    RawRequest {
        headers: HashMap<String, String>,
        #[serde(default = "HashMap::new")]
        query: HashMap<JsonOptic, HashMap<Keyword, Value>>,
        body: String
    },
    #[serde(rename = "jlens")]
    JLensRequest {
        headers: HashMap<String, String>,
        #[serde(default = "HashMap::new")]
        query: HashMap<JsonOptic, HashMap<Keyword, Value>>,
        body: JsonPredicate
    },
    #[serde(rename = "xml")]
    XmlRequest {
        headers: HashMap<String, String>,
        #[serde(default = "HashMap::new")]
        query: HashMap<JsonOptic, HashMap<Keyword, Value>>,
        body: String
    },
    #[serde(rename = "xpath")]
    XPathRequest {
        headers: HashMap<String, String>,
        #[serde(default = "HashMap::new")]
        query: HashMap<JsonOptic, HashMap<Keyword, Value>>,
        body: XmlPredicate
    }
}

impl HttpStubRequest {
    pub fn check_headers(&self, hs: HashMap<String, String>) -> bool {
        self.headers().iter().all(|(k, v)| hs.get(k).is_some_and(|vx| vx.to_lowercase() == v.to_lowercase()))
    }

    pub fn check_query_params(&self, params: Value) -> bool {
        if self.query().is_empty() {
            true
        } else {
            JsonPredicate::from_spec(self.query().clone()).validate(params).unwrap_or(false)
        }
    }

    pub fn check_body(&self, r_body: &RequestBody) -> bool {
        match self {
            HttpStubRequest::RequestWithoutBody { .. } =>
                match r_body {
                    RequestBody::AbsentRequestBody => true,
                    _ => false
                },
            HttpStubRequest::JsonRequest { body, .. } =>
                self.extract_json(r_body).map_or(false, |jx| &jx == body),
            HttpStubRequest::RawRequest { body, .. } =>
                match r_body {
                    RequestBody::SimpleRequestBody { value, .. } => value == body,
                    _ => false
                },
            HttpStubRequest::JLensRequest { body, .. } =>
                self.extract_json(r_body).and_then(|jx| body.validate(jx).ok()).unwrap_or(false),
            HttpStubRequest::XmlRequest { body, .. } =>
                match r_body {
                    RequestBody::SimpleRequestBody { value, .. } => xml_canonical_equal(value, body),
                    _ => false
                },
            HttpStubRequest::XPathRequest { body, .. } =>
                match r_body {
                    RequestBody::SimpleRequestBody { value, .. } => body.validate(value).unwrap_or(false),
                    _ => false
                }
        }
    }

    pub fn extract_json(&self, r_body: &RequestBody) -> Option<Value> {
        match (self, r_body) {
            (HttpStubRequest::JsonRequest { .. } | HttpStubRequest::JLensRequest { .. }, RequestBody::SimpleRequestBody { value, .. }) =>
                serde_json::from_str(&value).ok(),
            _ => None
        }
    }

    fn headers(&self) -> &HashMap<String, String> {
        match self {
            HttpStubRequest::RequestWithoutBody { headers, .. } => headers,
            HttpStubRequest::JsonRequest { headers, .. } => headers,
            HttpStubRequest::RawRequest { headers, .. } => headers,
            HttpStubRequest::JLensRequest { headers, .. } => headers,
            HttpStubRequest::XmlRequest { headers, .. } => headers,
            HttpStubRequest::XPathRequest { headers, .. } => headers,
        }
    }

    fn query(&self) -> &HashMap<JsonOptic, HashMap<Keyword, Value>> {
        match self {
            HttpStubRequest::RequestWithoutBody { query, .. } => query,
            HttpStubRequest::JsonRequest { query, .. } => query,
            HttpStubRequest::RawRequest { query, .. } => query,
            HttpStubRequest::JLensRequest { query, .. } => query,
            HttpStubRequest::XmlRequest { query, .. } => query,
            HttpStubRequest::XPathRequest { query, .. } => query,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "mode")]
pub enum HttpStubResponse {
    #[serde(rename = "raw")]
    RawResponse {
        code: u16,
        headers: HashMap<String, String>,
        body: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        delay: Option<Duration>
    },
    #[serde(rename = "json")]
    JsonResponse {
        code: u16,
        headers: HashMap<String, String>,
        body: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        delay: Option<Duration>,
        //is_template: bool
    },
    #[serde(rename = "xml")]
    XmlResponse {
        code: u16,
        headers: HashMap<String, String>,
        body: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        delay: Option<Duration>,
        #[serde(default = "default_true")]
        is_template: bool
    }
}

fn default_true() -> bool { true }

impl HttpStubResponse {
    pub fn get_delay(&self) -> &Option<Duration> {
        match self {
            HttpStubResponse::RawResponse { delay, .. } => delay,
            HttpStubResponse::JsonResponse { delay, .. } => delay,
            HttpStubResponse::XmlResponse { delay, .. } => delay
        }
    }

    pub fn substitute_xml(&mut self, data: Value, request_xml: Option<&str>) {
        use crate::utils::transformations::xml::substitute_xml_template;
        if let HttpStubResponse::XmlResponse { body, is_template: true, .. } = self {
            *body = substitute_xml_template(body, &data, request_xml);
        }
    }
}

impl Substitute<Value> for HttpStubResponse {
    fn substitute(&mut self, b: Value) -> &Self {

        match self {
            HttpStubResponse::JsonResponse { body, .. } =>
                drop(body.substitute(b)),
            _ => ()
        }

        self
    }
}

fn xml_canonical_equal(a: &str, b: &str) -> bool {
    use sxd_document::parser as xml_parser;
    use sxd_document::writer;

    let canonicalize = |text: &str| -> Option<String> {
        let package = xml_parser::parse(text).ok()?;
        let document = package.as_document();
        let mut output = Vec::new();
        writer::format_document(&document, &mut output).ok()?;
        String::from_utf8(output).ok()
    };

    match (canonicalize(a), canonicalize(b)) {
        (Some(ca), Some(cb)) => ca == cb,
        _ => false
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpStub {
    #[serde(default)]
    pub created: DateTime<Utc>,
    pub scope: Scope,
    #[serde(default)]
    pub times: Option<i64>,
    pub name: String,
    pub method: HttpMethod,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(with = "serde_regex")]
    #[serde(default)]
    pub path_pattern: Option<Regex>,
    #[serde(default)]
    pub seed: Option<Value>,
    #[serde(default)]
    pub state: Option<HashMap<JsonOptic, HashMap<Keyword, Value>>>,
    pub request: HttpStubRequest,
    #[serde(default)]
    pub persist: Option<HashMap<JsonOptic, Value>>,
    pub response: HttpStubResponse,
    #[serde(default)]
    pub callback: Option<Callback>
}

impl HttpStub {
    pub fn extract_groups(&self, path: &str) -> Option<HashMap<String, String>> {
        self.path_pattern.clone().and_then(|pattern| {
            let names = pattern.capture_names().filter_map(|n| n); 

            pattern.captures(path).map(|c| {
                names.filter_map(|n| c.name(n).map(|m| (n.to_string(), m.as_str().to_string()))).collect::<Vec<_>>()
            })
        }).map(|v| HashMap::from_iter(v))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CallbackResponseMode {
    Json
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "mode")]
pub enum CallbackRequest {
    #[serde(rename = "no_body")]
    CallbackRequestWithoutBody {
        url: String,
        method: HttpMethod,
        headers: HashMap<String, String>
    },
    #[serde(rename = "raw")]
    RawCallbackRequest {
        url: String,
        method: HttpMethod,
        headers: HashMap<String, String>,
        body: String
    },
    #[serde(rename = "json")]
    JsonCallbackRequest {
        url: String,
        method: HttpMethod,
        headers: HashMap<String, String>,
        body: Value
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Callback {
    HttpCallback {
        request: CallbackRequest,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        response_mode: Option<CallbackResponseMode>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        persist: Option<HashMap<JsonOptic, Value>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        callback: Option<Box<Callback>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        delay: Option<Duration>
    }
}

#[derive(Clone)]
pub struct State {
    pub id: Uuid,
    pub created: DateTime<Utc>,
    pub data: Value
}

impl State {
    pub fn fresh() -> State {
        State {
            id: Uuid::new_v4(),
            created: Utc::now(),
            data: json!({})
        }
    }
}