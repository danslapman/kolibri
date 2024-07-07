use crate::{api::model::RequestBody, misc::Substitute};
use crate::api::resolver::StubResolver;
use crate::error::Error;
use crate::model::*;
use crate::model::persistent::HttpStubResponse;
use std::collections::HashMap;
use serde_json::{json, Value};

pub struct ExecHandler {
    stub_res: StubResolver
}

impl ExecHandler {
    pub fn new(stub_res: StubResolver) -> ExecHandler {
        ExecHandler { stub_res }
    }

    pub async fn exec(&self, with_method: HttpMethod, with_path: String, with_headers: HashMap<String, String>, query_object: Value, body: RequestBody) -> Result<HttpStubResponse, Error> {
        let (mut stub, stateOp) = 
            match self.stub_res.find_stub_and_state(Scope::Countdown, &with_method, &with_path, &with_headers, &query_object, &body).await?
                .or(self.stub_res.find_stub_and_state(Scope::Ephemeral, &with_method, &with_path, &with_headers, &query_object, &body).await?)
                .or(self.stub_res.find_stub_and_state(Scope::Persistent, &with_method, &with_path, &with_headers, &query_object, &body).await?) {
                    Some((s, sto)) => (s, sto),
                    None => {
                        return Err(Error::new(format!("Can't find any stub for [{:?}] {:?}", with_method, with_path)))
                    }
                };

        let body_json = stub.request.extract_json(&body);
        let groups = stub.extract_groups(&with_path);

        let data = json!({
            "req": body_json,
            "query": query_object,
            "pathParts": groups,
            "headers": with_headers
        });

        stub.response.substitute(data);

        Ok(stub.response)
    }
}