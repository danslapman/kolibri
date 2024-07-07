use crate::api::model::RequestBody;
use crate::api::resolver::StubResolver;
use crate::error::Error;
use crate::misc::{Renderable, Substitute};
use crate::model::*;
use crate::model::persistent::HttpStubResponse;
use json_value_merge::Merge;
use std::collections::HashMap;
use persistent::State;
use serde_json::{json, Value};

pub struct ExecHandler {
    stub_res: StubResolver
}

impl ExecHandler {
    pub fn new(stub_res: StubResolver) -> ExecHandler {
        ExecHandler { stub_res }
    }

    pub async fn exec(&self, with_method: HttpMethod, with_path: String, with_headers: HashMap<String, String>, query_object: Value, body: RequestBody) -> Result<HttpStubResponse, Error> {
        let (mut stub, state_op) = 
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
        let segments = groups.map(|gs| 
            gs.into_iter().map(|(name, value)| 
                (name, serde_json::from_str(&value).unwrap_or(Value::String(value))))
            ).map(Value::from_iter);

        let data = json!({
            "req": body_json,
            "state": state_op.clone().map(|s| s.data),
            "query": query_object,
            "pathParts": segments,
            "headers": with_headers
        });

        stub.response.substitute(data.clone());

        if let Some(mut persist_spec) = stub.persist {
            persist_spec.fill(data);
            
            let mut current_state = state_op.unwrap_or(State::fresh());
            current_state.data.merge(&persist_spec.render_json());

            self.stub_res.upsert_state(current_state).await;
        }

        Ok(stub.response)
    }
}