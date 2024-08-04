use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use serde_json::de;
use serde_json::{Number, Value};
use std::collections::HashMap;
use crate::sanboxing::{CodeRunner, JsSandbox};
use crate::utils::js::optic::{JsonOptic, ValueExt};
use crate::utils::transformations::CODE_PATTERN;

static JSON_OPTIC_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$([:~])?\{([\p{L}\d\.\[\]\-_]+)\}").unwrap());

pub struct JsonPatcher {
    new_value: Value
}

impl JsonPatcher {
    fn new(new_value: Value) -> JsonPatcher {
        JsonPatcher { new_value }
    }

    fn apply(&self, target: &mut Value) {
        *target = self.new_value.clone()
    }
}

pub struct JsonTemplater {
    values: Value,
    code_runner: CodeRunner
}

impl JsonTemplater {
    pub fn new(values: Value) -> JsonTemplater {
        let environment = serde_json::from_value::<HashMap<String, Value>>(values.clone()).unwrap_or(HashMap::new());

        JsonTemplater { values, code_runner: JsSandbox::make_runner(environment).unwrap() }
    }

    pub fn make_patcher_fn<'l>(&'l mut self, defn: &'l str) -> Option<JsonPatcher> {
        let captures = JSON_OPTIC_PATTERN.captures_iter(defn).collect::<Vec<_>>();

        if !captures.is_empty() {
            if let [cap] = &captures[..] {
                let modifier = cap.get(1).map(|m| m.as_str());
                let path = &cap[2];
                let optic = JsonOptic::from_path(path);
    
                if self.values.validate(&optic) {
                    let mut new_value = self.values.get_all(&optic)[0].clone();
    
                    if modifier == Some(":") {
                        new_value = cast_to_string(new_value);
                    } else if modifier == Some("~") {
                        new_value = cast_from_string(new_value);
                    }
    
                    return Some(JsonPatcher::new(new_value))
                }
            } else {
                let replacement = |caps: &Captures| -> String {
                    let path = &caps[2];
                    let optic = JsonOptic::from_path(path);
    
                    let str_value = self.values.get_all(&optic).first().map(|v| render_subst(v));
                    str_value.unwrap_or(path.to_string())
                };
    
                return Some(JsonPatcher::new(Value::String(
                    JSON_OPTIC_PATTERN.replace_all(defn, replacement).to_string()
                )))
            }
        } else {
            let code_captures = CODE_PATTERN.captures_iter(defn).collect::<Vec<_>>();

            if let [cap] = &code_captures[..] {
                let code = &cap[1];
                return Some(JsonPatcher::new(self.code_runner.eval(code).unwrap()));
            }
        }

        None
    }
}

pub trait JsonTransformations {
    fn update_in_place_by_fn(&mut self, modify: fn(&mut Value));
    fn update_in_place_by_closure(&mut self, modify: &dyn Fn(&mut Value));
    fn update_in_place_by_closure_mut(&mut self, modify: &mut dyn FnMut(&mut Value));
    fn substitute_in_place(&mut self, values: Value);
    fn patch_in_place(&mut self, values: Value, schema: HashMap<JsonOptic, String>);
}

impl JsonTransformations for Value {
    fn update_in_place_by_fn(&mut self, modify: fn(&mut Value)) {
        match self {
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => modify(self),
            Value::Array(vs) => vs.iter_mut().for_each(|el| el.update_in_place_by_fn(modify)),
            Value::Object(kvs) => kvs.iter_mut().for_each(|(_, val)| val.update_in_place_by_fn(modify))
        }
    }

    fn update_in_place_by_closure(&mut self, modify: &dyn Fn(&mut Value)) {
        match self {
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => modify(self),
            Value::Array(vs) => vs.iter_mut().for_each(|el| el.update_in_place_by_closure(modify)),
            Value::Object(kvs) => kvs.iter_mut().for_each(|(_, val)| val.update_in_place_by_closure(modify))
        }
    }

    fn update_in_place_by_closure_mut(&mut self, modify: &mut dyn FnMut(&mut Value)) {
        match self {
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => modify(self),
            Value::Array(vs) => vs.iter_mut().for_each(|el| el.update_in_place_by_closure_mut(modify)),
            Value::Object(kvs) => kvs.iter_mut().for_each(|(_, val)| val.update_in_place_by_closure_mut(modify))
        }
    }

    fn substitute_in_place(&mut self, values: Value) {
        let mut templater = JsonTemplater::new(values);

        let mut upd = |vx: &mut Value| {
            match &vx {
                Value::String(s) => {
                    if let Some(patcher) = templater.make_patcher_fn(&s) {
                        patcher.apply(vx)
                    }
                },
                _ => ()
            }
        };

        self.update_in_place_by_closure_mut(&mut upd);
    }

    fn patch_in_place(&mut self, values: Value, schema: HashMap<JsonOptic, String>) {
        let mut templater = JsonTemplater::new(values);

        for (optic, defn) in schema {
            if let Some(patcher) = templater.make_patcher_fn(&defn) {
                let mut new_value = Value::Null;
                patcher.apply(&mut new_value);
                self.set(&optic, &new_value);
            }
        }
    }
}

fn cast_to_string(value: Value) -> Value {
    match value {
        Value::Bool(bv) => Value::String(bv.to_string()),
        Value::Number(nv) => Value::String(nv.to_string()),
        other => other
    }
}

fn cast_from_string(value: Value) -> Value {
    match value {
        Value::String(s) => match s.as_str() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            d if de::from_str::<'_, Number>(d).is_ok() => Value::Number(de::from_str(d).unwrap()),
            _ => Value::String(s)
        },
        other => other
    }
}

fn render_subst(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(vs) => vs.iter().map(|j| render_subst(j)).collect::<Vec<_>>().join(", "),
        _ => serde_json::to_string(value).unwrap()
    }
}

#[cfg(test)]
mod json_templater_tests {
    use fluent_assertions::*;
    use serde_json::{json, Value};
    use std::collections::HashSet;
    use uuid::Uuid;
    use crate::utils::transformations::js::*;

    #[test]
    fn fill_template() {
        let mut template: Value = json!({
            "description": "${description}",
            "topic" : "${extras.topic}",
            "comment" : "${extras.comments.[0].text}",
            "meta" : {
                "field1" : "${extras.fields.[0]}"
            },
            "composite": "${extras.topic}: ${description}"
        });

        let data: Value = json!({
            "description": "Some description",
            "extras": {
                "fields": ["f1", "f2"],
                "topic": "Main topic",
                "comments": [{"text": "First nah!"}, {"text": "Okay"}]
            }
        });

        template.substitute_in_place(data);

        assert_eq!(template, json!(
            {
               "description": "Some description",
               "topic" : "Main topic",
               "comment" : "First nah!",
               "meta" : {
                   "field1": "f1"
               },
               "composite" : "Main topic: Some description"
            }
        ))
    }

    #[test]
    fn absent_fields_should_be_ignored() {
        let mut template: Value = json!(
            {
                "value": "${description}"
            }
        );

        let data: Value = json!({});

        template.substitute_in_place(data);

        assert_eq!(template, json!({"value": "${description}"}))
    }

    #[test]
    fn substitution_of_object() {
        let mut template: Value = json!(
            {
                "value": "${message}"
            }
        );

        let data: Value = json!(
            {
                "message": {"peka": "name"}
            }
        );

        template.substitute_in_place(data);

        assert_eq!(template, json!({"value": {"peka": "name"}}))
    }

    #[test]
    fn convert_to_a_string() {
        let mut template: Value = json!(
            {
                "a": "$:{b1}",
                "b" : "$:{b2}",
                "c" : "$:{n}"
            }
        );

        let data: Value = json!(
            {
                "b1" : true,
                "b2" : false,
                "n" : 45.99
            }
        );

        template.substitute_in_place(data);

        assert_eq!(template, json!(
            {
                "a" : "true",
                "b" : "false",
                "c" : "45.99"
            }
        ))
    }

    #[test]
    fn convert_from_string() {
        let mut template: Value = json!(
            {
                "a": "$~{b1}",
                "b" : "$~{b2}",
                "c" : "$~{n}"
            }
        );

        let data: Value = json!(
            {
                "b1" : "true",
                "b2" : "false",
                "n" : "45.99"
            }
        );

        template.substitute_in_place(data);

        assert_eq!(template, json!(
            {
                "a" : true,
                "b" : false,
                "c" : 45.99
            }
        ))
    }

    #[test]
    fn javascript_eval() {
        let mut target: Value = json!(
            {
                "a" : "%{randomString(10)}",
                "ai" : "%{randomString(\"ABCDEF1234567890\", 4, 6)}",
                "b" : "%{randomInt(5)}",
                "bi" : "%{randomInt(3, 8)}",
                "c" : "%{randomLong(5)}",
                "ci" : "%{randomLong(3, 8)}",
                "d" : "%{UUID()}"
            }
        );

        target.substitute_in_place(Value::Null);

        let allowed_chars = HashSet::from(['A', 'B', 'C', 'D', 'D', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0']);

        target.get_all(&JsonOptic::from_path("a")).first().and_then(|v| v.as_str()).filter(|s| s.len() == 10).should().be_some();
        target.get_all(&JsonOptic::from_path("ai")).first().and_then(|v| v.as_str())
            .filter(|&s| s.chars().all(|c| allowed_chars.contains(&c))).should().be_some();
        target.get_all(&JsonOptic::from_path("b")).first().and_then(|v| v.as_i64()).filter(|&i| i < 5).should().be_some();
        target.get_all(&JsonOptic::from_path("bi")).first().and_then(|v| v.as_i64()).filter(|&i| i >= 3 && i < 8).should().be_some();
        target.get_all(&JsonOptic::from_path("c")).first().and_then(|v| v.as_i64()).filter(|&i| i < 5).should().be_some();
        target.get_all(&JsonOptic::from_path("ci")).first().and_then(|v| v.as_i64()).filter(|&i| i >= 3 && i < 8).should().be_some();
        target.get_all(&JsonOptic::from_path("d")).first().and_then(|v| v.as_str()).and_then(|s| Uuid::try_parse(s).ok()).should().be_some();
    }

    #[test]
    fn formatted_eval() {
        let mut target: Value = json!(
            {
                "fmt" : "%{randomInt(10) + ': ' + randomLong(10) + ' | ' + randomString(12)}"
            }
        );

        target.substitute_in_place(Value::Null);

        target.get_all(&JsonOptic::from_path("fmt")).first().and_then(|v| v.as_str()).filter(|s| s.len() == 19).should().be_some();
    }

    #[test]
    fn json_patcher() {
        let mut target: Value = json!(
            {
                "f1" : "v1",
                "a2" : ["e1", "e2", "e3"],
                "o3" : {}
            }
        );

        let source: Value = json!(
            {
                "name" : "Peka",
                "surname" : "Kekovsky",
                "comment" : "nondesc"
            }
        );

        let schema = HashMap::from([
            (JsonOptic::from_path("a2.[4]"), "${comment}".to_string()),
            (JsonOptic::from_path("o3.client"), "${name} ${surname}".to_string())
        ]);

        target.patch_in_place(source, schema);

        assert_eq!(target.get_all(&JsonOptic::from_path("a2.[4]")), vec![&Value::String("nondesc".to_string())]);
        assert_eq!(target.get_all(&JsonOptic::from_path("o3.client")), vec![&Value::String("Peka Kekovsky".to_string())]);
    }
}