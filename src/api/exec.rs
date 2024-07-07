use crate::api::model::RequestBody;
use crate::api::resolver::StubResolver;
use crate::error::Error;
use crate::model::*;
use crate::model::persistent::HttpStubResponse;
use std::collections::HashMap;
use serde_json::Value;

pub struct ExecHandler {
    stub_res: StubResolver
}

impl ExecHandler {
    pub fn new(stub_res: StubResolver) -> ExecHandler {
        ExecHandler { stub_res }
    }

    pub fn exec(&self, with_method: HttpMethod, with_path: String, with_headers: HashMap<String, String>, query_object: Value, body: RequestBody) -> Result<HttpStubResponse, Error> {
        let (stub, stateOp) = 
            match self.stub_res.find_stub_and_state(Scope::Countdown, &with_method, &with_path, &with_headers, &query_object, &body)?
                .or(self.stub_res.find_stub_and_state(Scope::Ephemeral, &with_method, &with_path, &with_headers, &query_object, &body)?)
                .or(self.stub_res.find_stub_and_state(Scope::Persistent, &with_method, &with_path, &with_headers, &query_object, &body)?) {
                    Some((s, sto)) => (s, sto),
                    None => {
                        return Err(Error::new(format!("Can't find any stub for [{:?}] {:?}", with_method, with_path)))
                    }
                };

        let body_string = body.extract_string();
        let body_json = stub.request.extract_json(&body);
        let groups: Option<HashMap<String, String>> = stub.path_pattern.and_then(|pattern| {
            let names = pattern.capture_names().filter_map(|n| n); 

            pattern.captures(&with_path).map(|c| {
                names.filter_map(|n| c.name(n).map(|m| (n.to_string(), m.as_str().to_string()))).collect::<Vec<_>>()
            })
        }).map(|v| HashMap::from_iter(v));

        Ok(stub.response)
    }
}