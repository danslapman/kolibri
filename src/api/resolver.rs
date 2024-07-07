use crate::api::model::RequestBody;
use crate::error::Error;
use crate::misc::Renderable;
use crate::model::*;
use crate::predicate_dsl::json::JsonPredicate;
use futures::future::join_all;
use log::{error, info};
use persistent::{HttpStub, State};
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct StubResolver {
    mocks: Vec<HttpStub>,
    states: RwLock<HashMap<Uuid, State>>
}

impl StubResolver {
    pub fn new(mocks: Vec<HttpStub>, states: RwLock<HashMap<Uuid, State>>) -> StubResolver {
        StubResolver { mocks, states }
    }

    pub async fn find_stub_and_state(&self, in_scope: Scope, with_method: &HttpMethod, with_path: &str, with_headers: &HashMap<String, String>, query_object: &Value, body: &RequestBody) -> Result<Option<(HttpStub, Option<State>)>, Error> {
        info!("Searching searching stubs for {:?} of scope {:?}", with_path, in_scope);

        let candidates0: Vec<HttpStub> = self.mocks.clone().into_iter()
            .filter(|m| m.scope == in_scope && m.method == *with_method &&
                (m.path.as_ref().is_some_and(|p| *p == with_path) || m.path_pattern.as_ref().is_some_and(|pp| pp.is_match(with_path))) &&
                (in_scope != Scope::Countdown || m.times.is_some_and(|rem| rem > 0))
            )
            .collect();

        if candidates0.is_empty() {
            info!("Stubs for {:?} were not found in scope {:?}", with_path, in_scope);
            return Ok(None);
        }

        let candidates1 = candidates0.into_iter().filter(|s| s.request.check_query_params(query_object.clone())).collect::<Vec<_>>();

        if candidates1.is_empty() {
            info!("There are no {:?} candidates in scope {:?} after query parameters check", with_path, in_scope);
            return Ok(None);
        }

        let candidates2 = candidates1.into_iter().filter(|s| s.request.check_headers(with_headers.clone())).collect::<Vec<_>>();

        if candidates2.is_empty() {
            info!("There are no {:?} candidates in scope {:?} after headers check", with_path, in_scope);
            return Ok(None);
        }

        let candidates3 = candidates2.into_iter().filter(|s| s.request.check_body(body)).collect::<Vec<_>>();

        if candidates3.is_empty() {
            info!("There are no {:?} candidates in scope {:?} after body check", with_path, in_scope);
            return Ok(None);
        }

        let candidates4: Vec<(HttpStub, Vec<State>)> = join_all(candidates3.into_iter().map(|s| async {
            let body_json = s.request.extract_json(body);
            let groups = s.extract_groups(with_path);
            let segments = groups.map(|gs| 
                gs.into_iter().map(|(name, value)| 
                    (name, serde_json::from_str(&value).unwrap_or(Value::String(value))))
                ).map(Value::from_iter);

            let additional_fields = json!({
                "__query": query_object,
                "__segments": segments,
                "__headers": with_headers
            });

            let mut matching_states = Vec::new();

            if let Some(mut state_spec) = s.state.clone() {
                state_spec.fill(additional_fields);
                if let Some(bd) = body_json {
                    state_spec.fill(bd);
                }
                let predicate = JsonPredicate::from_spec(state_spec);

                for state in self.states.read().await.values() {
                    match predicate.validate(state.data.clone()) {
                        Ok(true) => matching_states.push(state.clone()),
                        Ok(false) => (),
                        Err(err) => error!("{err}"),
                    }
                }
            }

            (s, matching_states)
        })).await;

        if candidates4.iter().any(|(_, states)| states.len() > 1) {
            error!("For one or more stubs, multiple suitable states were found");
            return Err(Error::new("For one or more stubs, multiple suitable states were found".to_string()));
        }

        if candidates4.iter().filter(|(_, states)| !states.is_empty()).count() > 1 {
            error!("For more than one stub, suitable states were found");
            return Err(Error::new("For more than one stub, suitable states were found".to_string()));
        }

        if candidates4.len() > 1 && candidates4.iter().all(|(stub, states)| stub.state.is_some() && states.is_empty()) {
            error!("No suitable state found for any stub");
            return Err(Error::new("No suitable state found for any stub".to_string()));
        }

        if candidates4.len() > 1 && candidates4.iter().all(|(stub, _)| stub.state.is_none()) {
            error!("More than one stateless stub found");
            return Err(Error::new("More than one stateless stub found".to_string()));
        }

        let res = candidates4.iter().find(|(_, states)| states.len() == 1).or(candidates4.iter().find(|(stub, _)| stub.state.is_none()));

        Ok(res.map(|(stub, states)| (stub.clone(), states.first().map(|s| s.clone()))))
    }

    //TODO: move to a separate abstraction?
    pub async fn upsert_state(&self, state: State) {
        let mut states = self.states.write().await;
        states.insert(state.id, state);
    }
}