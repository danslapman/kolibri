#[derive(Clone)]
pub enum RequestBody {
    AbsentRequestBody,
    SimpleRequestBody {
        value: String
    }
}

impl RequestBody {
    pub fn extract_string(&self) -> Option<String> {
        match self {
            RequestBody::SimpleRequestBody { value } => Some(value.clone()),
            _ => None
        }
    }
}