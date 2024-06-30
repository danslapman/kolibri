use crate::utils::transformations::js::JsonTransformations;
use serde_json::Value;

pub trait Substitute<B> {
    fn substitute(&mut self, b: B) -> &Self;
}

impl Substitute<Value> for Value {
    fn substitute(&mut self, b: Value) -> &Self {
        self.substitute_in_place(b);
        self
    }
}