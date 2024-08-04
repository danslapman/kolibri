use crate::error::Error;
use rustyscript::{Runtime, RuntimeOptions};
use serde_json::Value;
use std::collections::HashMap;

pub struct CodeRunner {
    runtime: Runtime
}

impl CodeRunner {
    pub fn eval(&mut self, code: &str) -> Result<Value, Error> {
        self.runtime.eval(code).map_err(Error::from)
    }
}

pub struct JsSandbox {}

impl JsSandbox {
    pub fn make_runner(environment: HashMap<String, Value>) -> Result<CodeRunner, Error> {
        let mut runtime = Runtime::new(RuntimeOptions::default()).map_err(Error::from)?;

        for (key, value) in environment.into_iter() {
            let serialized_value = serde_json::to_string(&value).map_err(Error::from)?;
            runtime.eval::<()>(format!("var {} = {};", key, serialized_value).as_str()).map_err(Error::from)?;
        }
        
        Ok(CodeRunner { runtime })
    }
}

#[cfg(test)]
mod sandboxing_tests {
    use std::collections::HashMap;

    use serde_json::{json, Value};
    use super::JsSandbox;
    
    #[test]
    fn eval_literals() {
        let res1: Value = JsSandbox::make_runner(HashMap::new()).unwrap().eval("[1, \"test\", true]").unwrap();
        assert_eq!(res1, json!([1, "test", true]));

        let res2: Value = JsSandbox::make_runner(HashMap::new()).unwrap().eval("var res = {'a': {'b': 'c'}}; res").unwrap();
        assert_eq!(res2, json!({"a": {"b": "c"}}));
    }

    #[test]
    fn eval_simple_arithmetics() {
        let res: Value = JsSandbox::make_runner(HashMap::new()).unwrap().eval("1 + 2").unwrap();
        assert_eq!(res, json!(3));
    }

    #[test]
    fn eval_with_context() {
        let env = HashMap::from([("a".to_string(), json!(1)), ("b".to_string(), json!(2))]);
        let res: Value = JsSandbox::make_runner(env).unwrap().eval("a + b").unwrap();
        assert_eq!(res, json!(3));
    }

    #[test]
    fn evaluations_should_not_have_shared_data() {
        JsSandbox::make_runner(HashMap::new()).unwrap().eval("var a = 42;").unwrap();
        JsSandbox::make_runner(HashMap::new()).unwrap().eval("a").expect_err("a is not defined");
    }

    #[test]
    fn get_value_from_provided_map() {
        let env = HashMap::from([("m".to_string(), json!({"f1": "hello"}))]);
        let res: Value = JsSandbox::make_runner(env).unwrap().eval("m.f1").unwrap();
        assert_eq!(res, json!("hello"));
    }
}