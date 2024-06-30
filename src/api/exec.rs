use crate::api::resolver::StubResolver;

pub struct ExecHandler {
    stub_res: StubResolver
}

impl ExecHandler {
    pub fn new(stub_res: StubResolver) -> ExecHandler {
        ExecHandler { stub_res }
    }
}