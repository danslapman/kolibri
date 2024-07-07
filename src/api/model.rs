#[derive(Clone)]
pub enum RequestBody {
    AbsentRequestBody,
    SimpleRequestBody {
        raw_value: Vec<u8>,
        value: String
    }
}

impl RequestBody {
    pub fn extract_string(&self) -> Option<String> {
        match self {
            RequestBody::SimpleRequestBody { value, .. } => Some(value.clone()),
            _ => None
        }
    }
}