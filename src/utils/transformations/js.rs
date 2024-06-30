use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use serde_json::de;
use serde_json::{Number, Value};
use std::collections::HashMap;
use crate::utils::js::optic::{JsonOptic, ValueExt};

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
    values: Value
}

impl JsonTemplater {
    pub fn new(values: Value) -> JsonTemplater {
        JsonTemplater { values }
    }

    pub fn make_patcher_fn<'l>(&'l self, defn: &'l str) -> Option<JsonPatcher> {
        let captures = JSON_OPTIC_PATTERN.captures_iter(defn).collect::<Vec<_>>();

        if captures.is_empty() {
            return None;
        }

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

        None
    }
}

pub trait JsonTransformations {
    fn update_in_place_by_fn(&mut self, modify: fn(&mut Value));
    fn update_in_place_by_closure(&mut self, modify: &dyn Fn(&mut Value));
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

    fn substitute_in_place(&mut self, values: Value) {
        let templater = JsonTemplater::new(values);

        let upd = |vx: &mut Value| {
            match &vx {
                Value::String(s) => {
                    if let Some(patcher) = templater.make_patcher_fn(&s) {
                        patcher.apply(vx)
                    }
                },
                _ => ()
            }
        };

        self.update_in_place_by_closure(&upd);
    }

    fn patch_in_place(&mut self, values: Value, schema: HashMap<JsonOptic, String>) {
        let templater = JsonTemplater::new(values);

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
    use serde_json::{json, Value};
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