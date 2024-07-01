use crate::utils::js::optic::JsonOptic;
use crate::utils::transformations::js::JsonTransformations;
use serde_json::Value;
use std::collections::HashMap;

pub trait Substitute<B> {
    fn substitute(&mut self, b: B) -> &Self;
}

impl Substitute<Value> for Value {
    fn substitute(&mut self, b: Value) -> &Self {
        self.substitute_in_place(b);
        self
    }
}

pub trait Renderable {
    fn render_json(self) -> Value;
    fn fill<S: Clone>(&mut self, values: S) -> &Self where Value: Substitute<S>;
    fn with_prefix(&mut self, prefix: &str) -> &Self;
}

impl Renderable for HashMap<JsonOptic, Value> {
    fn render_json(self) -> Value {
        Value::Object(self.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    fn fill<S: Clone>(&mut self, values: S) -> &Self where Value: Substitute<S> {
        for (_, v) in self.iter_mut() {
            (*v).substitute(values.clone());
        }

        self
    }

    fn with_prefix(&mut self, prefix: &str) -> &Self {
        let mut buf = HashMap::new();

        for (k, v) in self.drain() {
            let new_k = k.append_path(prefix);

            buf.insert(new_k, v);
        }

        self.extend(buf.into_iter());
        self
    }
}