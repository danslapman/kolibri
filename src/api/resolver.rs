use crate::api::model::RequestBody;
use crate::error::Error;
use crate::model::*;
use crate::predicate_dsl::keyword::Keyword;
use crate::utils::js::optic::JsonOptic;
use log::info;
use persistent::{HttpStub, State};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct StubResolver {
    mocks: Vec<HttpStub>,
    states: Mutex<Vec<State>>
}

type StateSpec = HashMap<JsonOptic, HashMap<Keyword, Value>>;

impl StubResolver {
    pub fn new(mocks: Vec<HttpStub>, states: Mutex<Vec<State>>) -> StubResolver {
        StubResolver { mocks, states }
    }

    fn find_stub_and_state(&self, in_scope: Scope, with_method: HttpMethod, with_path: String, with_headers: HashMap<String, String>, query_object: Value, body: RequestBody) -> Result<Option<HttpStub>, Error> {
        info!("Searching searching stubs for {:?} of scope {:?}", with_path, in_scope);

        let candidates0: Vec<HttpStub> = self.mocks.clone().into_iter()
            .filter(|m| m.scope == in_scope && m.method == with_method &&
                (m.path.as_ref().is_some_and(|p| *p == with_path) || m.path_pattern.as_ref().is_some_and(|pp| pp.is_match(with_path.as_str()))) &&
                (in_scope != Scope::Countdown || m.times.is_some_and(|rem| rem > 0))
            )
            .collect();

        if candidates0.is_empty() {
            info!("Stubs for {:?} not found in scope {:?}", with_path, in_scope);
            return Err(Error::new(format!("Stubs for {:?} not found in scope {:?}", with_path, in_scope)));
        }

        let candidates1 = candidates0.into_iter().filter(|s| s.request.check_query_params(query_object.clone())).collect::<Vec<_>>();

        if candidates1.is_empty() {
            info!("There are no {:?} candidates in scope {:?} after query parameters check", with_path, in_scope);
            return Err(Error::new(format!("There are no {:?} candidates in scope {:?} after query parameters check", with_path, in_scope)));
        }

        let candidates2 = candidates1.into_iter().filter(|s| s.request.check_headers(with_headers.clone())).collect::<Vec<_>>();

        if candidates2.is_empty() {
            info!("There are no {:?} candidates in scope {:?} after headers check", with_path, in_scope);
            return Err(Error::new(format!("There are no {:?} candidates in scope {:?} after headers check", with_path, in_scope)));
        }

        let candidates3 = candidates2.into_iter().filter(|s| s.request.check_body(body.clone())).collect::<Vec<_>>();

        if candidates3.is_empty() {
            info!("There are no {:?} candidates in scope {:?} after body check", with_path, in_scope);
            return Err(Error::new(format!("There are no {:?} candidates in scope {:?} after body check", with_path, in_scope)));
        }

        Ok(None)
    }
}