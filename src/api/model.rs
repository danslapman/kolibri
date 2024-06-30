#[derive(Clone)]
pub enum RequestBody {
    AbsentRequestBody,
    SimpleRequestBody {
        value: String
    }
}