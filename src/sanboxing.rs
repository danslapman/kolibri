use crate::error::Error;
use deno_core::v8;
use deno_core::JsRuntime;
use deno_core::RuntimeOptions;
use ouroboros::self_referencing;
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::LazyLock;

static PRELUDE: LazyLock<Cow<'_, str>> = LazyLock::new(|| String::from_utf8_lossy(include_bytes!("prelude.js")));

//Basically JsRuntime::eval, but without Error constraint
fn deno_eval<'s, T>(scope: &mut v8::HandleScope<'s>, code: &str,) -> Option<v8::Local<'s, T>>
    where
        v8::Local<'s, T>: TryFrom<v8::Local<'s, v8::Value>>,
{
    let scope = &mut v8::EscapableHandleScope::new(scope);
    let source = v8::String::new(scope, code).unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    let v = script.run(scope)?;
    scope.escape(v).try_into().ok()
}

#[self_referencing]
pub struct CodeRunner {
    runtime: JsRuntime,

    #[borrows(mut runtime)]
    #[not_covariant]
    scope: v8::HandleScope<'this>,
}

impl CodeRunner {
    pub fn eval(&mut self, code: &str) -> Result<Value, Error> {
        self.with_scope_mut(|scope| {
            let evaluated: Option<v8::Local<v8::Value>> = deno_eval::<v8::Value>(scope, code);
            let local: v8::Local<v8::Value> = evaluated.ok_or(Error::new("JS evaluation failed".to_string()))?;

            serde_v8::from_v8::<serde_json::Value>(scope, local).map_err(Error::from)
        })
    }
}

pub struct JsSandbox {}

impl JsSandbox {
    pub fn make_runner(environment: HashMap<String, Value>) -> Result<CodeRunner, Error> {
        let mut runner = CodeRunnerBuilder {
            runtime: JsRuntime::new(RuntimeOptions::default()),
            scope_builder: |rt| rt.handle_scope()
        }.build();

        runner.with_scope_mut(|scope| {
            deno_eval::<v8::Value>(scope, &PRELUDE);

            for (key, value) in environment.into_iter() {
                let serialized_value = serde_json::to_string(&value).expect("It cannot happen");
                deno_eval::<v8::Value>(scope, format!("var {} = {};", key, serialized_value).as_str());
            }
        });

        Ok(runner)
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