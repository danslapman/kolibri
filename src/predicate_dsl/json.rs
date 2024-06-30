use crate::predicate_dsl::keyword::Keyword;
use crate::utils::{IntoBD, IntoUSize};
use crate::utils::js::optic::{JsonOptic, ValueExt};
use regex::Regex;
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::de::Error;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

type Spec = HashMap<JsonOptic, HashMap<Keyword, Value>>;
type Condition<'r> = (&'r Keyword, &'r Value);

pub struct JsonPredicate {
    definition: Spec
}

impl JsonPredicate {
    pub fn validate(&self, json: Value) -> Result<bool, PredicateConstructionError<'_>> {
        let mut result: Vec<Result<bool, ValidationError<'_>>> = vec![];

        for (jo, conds) in self.definition.iter() {
            let all_data = json.get_all(jo);
            let data = all_data.first().unwrap_or(&&Value::Null);

            for (kwd, etalon) in conds.iter() {
                result.push(JsonPredicate::validate_one(kwd, etalon, *data));
            }
        }

        let (oks, errs): (Vec<_>, Vec<_>) = result.into_iter().partition(|el| el.is_ok());

        if errs.len() == 0 {
            Ok(oks.into_iter().filter_map(|el| el.ok()).all(|el| el))
        } else if errs.iter().all(|err| err.as_ref().err().unwrap().is_data_error()) {
            Ok(false)
        } else {
            let condition_errors = errs.into_iter().filter_map(|el| el.err())
                .filter_map(|err| match err {
                    ValidationError::ConditionError { keyword, argument } => Some((keyword, argument)),
                    _ => None
                }).collect::<Vec<_>>();
            Err(PredicateConstructionError { problems: condition_errors })
        }
    }

    pub fn from_spec(spec: Spec) -> JsonPredicate {
        JsonPredicate { definition: spec }
    }

    fn validate_one<'r>(kwd: &'r Keyword, etalon: &'r Value, value: &Value) -> Result<bool, ValidationError<'r>> {
        match (kwd, etalon, value) {
            (Keyword::Equals, v_eq, val) => Ok(v_eq == val),
            (Keyword::NotEq, v_neq, val) => Ok(v_neq != val),
            (Keyword::Greater, Value::Number(l_b), Value::Number(nv)) => Ok(nv.to_big_decimal() > l_b.to_big_decimal()),
            (Keyword::Greater, Value::Number(_), _) => Err(ValidationError::DataError),
            (Keyword::Gte, Value::Number(l_b), Value::Number(nv)) => Ok(nv.to_big_decimal() >= l_b.to_big_decimal()),
            (Keyword::Gte, Value::Number(_), _) => Err(ValidationError::DataError),
            (Keyword::Less, Value::Number(u_b), Value::Number(nv)) => Ok(nv.to_big_decimal() < u_b.to_big_decimal()),
            (Keyword::Less, Value::Number(_), _) => Err(ValidationError::DataError),
            (Keyword::Lte, Value::Number(u_b), Value::Number(nv)) => Ok(nv.to_big_decimal() <= u_b.to_big_decimal()),
            (Keyword::Lte, Value::Number(_), _) => Err(ValidationError::DataError),
            (Keyword::Rx, Value::String(rx), Value::String(s)) => match Regex::new(rx) {
                Ok(regex) => Ok(regex.is_match(s)),
                _ => Err(ValidationError::ConditionError {keyword: kwd, argument: etalon } )
            },
            (Keyword::Rx, Value::String(rx), _) if Regex::new(rx).is_ok() => Err(ValidationError::DataError),
            (Keyword::Size, Value::Number(size), Value::String(s)) => Ok(s.len() == size.to_usize()),
            (Keyword::Size, Value::Number(size), Value::Array(v)) => Ok(v.len() == size.to_usize()),
            (Keyword::Size, Value::Number(_), _) => Err(ValidationError::DataError),
            (Keyword::Exists, Value::Bool(true), val) => Ok(!val.is_null()),
            (Keyword::Exists, Value::Bool(false), val) => Ok(val.is_null()),
            (Keyword::In, Value::Array(possible), Value::Array(vals)) => Ok(possible.iter().any(|pv| vals.contains(pv))),
            (Keyword::In, Value::Array(possible), val) => Ok(possible.contains(val)),
            (Keyword::NotIn, Value::Array(impossible), Value::Array(vals)) => Ok(impossible.iter().all(|iv| !vals.contains(iv))),
            (Keyword::NotIn, Value::Array(impossible), val) => Ok(!impossible.contains(val)),
            (Keyword::AllIn, Value::Array(mandatory), Value::Array(vals)) => Ok(mandatory.iter().all(|mv| vals.contains(mv))),
            (Keyword::AllIn, Value::Array(_), _) => Err(ValidationError::DataError),
            (k, v, _) => Err(ValidationError::ConditionError {keyword: k, argument: v } )
        }
    }
}

impl Serialize for JsonPredicate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.definition.serialize(serializer)
    }
}

impl <'de> Deserialize<'de> for JsonPredicate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let spec = Spec::deserialize(deserializer)?;

        let mut faulty_fields: Vec<String> = vec![];

        for (optic, cond) in spec.iter() {
            for (kwd, v) in cond.iter() {
                if !validate_condition(kwd, v) {
                    faulty_fields.push(optic.to_string());
                }
            }
        }

        if !faulty_fields.is_empty() {
            Err(D::Error::custom(format!("Conditions are faulty on fields: {}", faulty_fields.join(", "))))
        } else {
            Ok(JsonPredicate { definition: spec })
        }
    }
}

impl Debug for JsonPredicate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(&self.definition).expect("Unserializable JsonPredicate!")
        )
    }
}

fn validate_condition<'r>(kwd: &'r Keyword, etalon: &'r Value) -> bool {
    match (kwd, etalon) {
        (Keyword::Equals | Keyword::NotEq, _) => true,
        (Keyword::Greater | Keyword::Gte | Keyword::Less | Keyword::Lte, Value::Number(_)) => true,
        (Keyword::Rx, Value::String(rx)) if Regex::new(rx).is_ok() => true,
        (Keyword::Size, Value::Number(_)) => true,
        (Keyword::Exists, Value::Bool(_)) => true,
        (Keyword::In | Keyword::NotIn | Keyword::AllIn, Value::Array(_)) => true,
        (_, _) => false
    }
}

pub struct PredicateConstructionError<'r> {
    pub problems: Vec<Condition<'r>>
}

enum ValidationError<'r> {
    ConditionError {
        keyword: &'r Keyword,
        argument: &'r Value
    },
    DataError
}

impl ValidationError<'_> {
    fn is_data_error(&self) -> bool {
        match self {
            ValidationError::DataError => true,
            _ => false
        }
    }
}

#[cfg(test)]
mod json_tests {
    use crate::predicate_dsl::keyword::Keyword;
    use crate::predicate_dsl::json::JsonPredicate;
    use crate::utils::js::optic::JsonOptic;
    use serde_json::{json, Value};
    use std::collections::HashMap;

    #[test]
    fn keyword_correctly_deserializes() {
        let kwd = serde_json::from_value::<Keyword>(json!("=="));

        assert!(kwd.is_ok());
    }

    #[test]
    fn json_predicate_should_produce_validator_from_correct_specification() {
        let json_spec: Value = json!({"field1": {"==": "test"}});

        let predicate = serde_json::from_value::<JsonPredicate>(json_spec);

        assert!(predicate.is_ok());
        assert_eq!(predicate.ok().unwrap().definition, HashMap::from([(JsonOptic::from_path("field1"), HashMap::from([(Keyword::Equals, json!("test"))]))]))
    }

    #[test]
    fn json_predicate_should_emit_correct_error_for_poor_specification() {
        let json_spec: Value = json!({"field1": {">=": "test"}});

        let predicate = serde_json::from_value::<JsonPredicate>(json_spec);

        assert!(predicate.is_err());
        assert_eq!(predicate.err().unwrap().to_string(), "Conditions are faulty on fields: field1")
    }

    #[test]
    fn check_equality() {
        let json_spec: Value = json!({
            "field1": {"==": "test"},
            "field2": {"==": [1, 2, 3]},
            "field3": {"==": {"name": "peka"}}
        });
        let predicate = serde_json::from_value::<JsonPredicate>(json_spec).ok().unwrap();

        assert!(predicate.validate(json!({
            "field1": "test",
            "field2": [1, 2, 3],
            "field3": {"name": "peka"}
        })).ok().unwrap());

        assert!(!predicate.validate(json!({
            "field1": "peka",
            "field2": [1, 2, 3],
            "field3": {"name": "peka"}
        })).ok().unwrap());
    }

    #[test]
    fn check_inequality() {
        let json_spec: Value = json!({
            "field1": {"!=": "test"},
            "field2": {"==": [1, 2, 3]},
            "field3": {"==": {"name": "peka"}}
        });
        let predicate = serde_json::from_value::<JsonPredicate>(json_spec).ok().unwrap();

        assert!(!predicate.validate(json!({
            "field1": "test",
            "field2": [1, 2, 3],
            "field3": {"name": "peka"}
        })).ok().unwrap());

        assert!(predicate.validate(json!({
            "field1": "peka",
            "field2": [1, 2, 3],
            "field3": {"name": "peka"}
        })).ok().unwrap());
    }

    #[test]
    fn check_greater_than() {
        let json_spec: Value = json!({"f" : {">": 42}});
        let predicate = serde_json::from_value::<JsonPredicate>(json_spec).ok().unwrap();

        assert!(predicate.validate(json!({"f": 43})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": 42})).ok().unwrap());
    }

    #[test]
    fn check_greater_or_equals() {
        let json_spec: Value = json!({"f" : {">=": 42}});
        let predicate = serde_json::from_value::<JsonPredicate>(json_spec).ok().unwrap();

        assert!(predicate.validate(json!({"f": 43})).ok().unwrap());
        assert!(predicate.validate(json!({"f": 42})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": 41})).ok().unwrap());
    }

    #[test]
    fn check_less_than() {
        let json_spec: Value = json!({"f" : {"<": 42}});
        let predicate = serde_json::from_value::<JsonPredicate>(json_spec).ok().unwrap();

        assert!(predicate.validate(json!({"f": 41})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": 42})).ok().unwrap());
    }

    #[test]
    fn check_less_or_equals() {
        let json_spec: Value = json!({"f" : {"<=": 42}});
        let predicate = serde_json::from_value::<JsonPredicate>(json_spec).ok().unwrap();

        assert!(predicate.validate(json!({"f": 41})).ok().unwrap());
        assert!(predicate.validate(json!({"f": 42})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": 43})).ok().unwrap());
    }

    #[test]
    fn check_range() {
        let json_spec: Value = json!({"f": {">": 40, "<=": 45, "!=": 43}});
        let predicate = serde_json::from_value::<JsonPredicate>(json_spec).ok().unwrap();

        assert!(!predicate.validate(json!({"f": 39})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": 40})).ok().unwrap());
        assert!(predicate.validate(json!({"f": 41})).ok().unwrap());
        assert!(predicate.validate(json!({"f": 42})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": 43})).ok().unwrap());
        assert!(predicate.validate(json!({"f": 44})).ok().unwrap());
        assert!(predicate.validate(json!({"f": 45})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": 46})).ok().unwrap());
    }

    #[test]
    fn check_regex_match() {
        let json_spec: Value = json!({"f": {"~=": r"\d{4,}"}});
        let predicate = serde_json::from_value::<JsonPredicate>(json_spec).ok().unwrap();

        assert!(!predicate.validate(json!({"f": "123"})).ok().unwrap());
        assert!(predicate.validate(json!({"f": "1234"})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": 1234})).ok().unwrap());
        assert!(predicate.validate(json!({"f": "1234a"})).ok().unwrap()); //TODO: fix incomplete match
        assert!(predicate.validate(json!({"f": "12345"})).ok().unwrap());
    }

    #[test]
    fn check_size() {
        let json_spec: Value = json!({"f": {"size": 4}});
        let predicate = serde_json::from_value::<JsonPredicate>(json_spec).ok().unwrap();

        assert!(predicate.validate(json!({"f": "1234"})).ok().unwrap());
        assert!(predicate.validate(json!({"f": [1, 2, 3, 4]})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": 1234})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": 4})).ok().unwrap());
    }

    #[test]
    fn check_exists() {
        let json_spec: Value = json!({"f": {"exists": true}});
        let predicate = serde_json::from_value::<JsonPredicate>(json_spec).ok().unwrap();

        assert!(predicate.validate(json!({"f": 42})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": null})).ok().unwrap());
        assert!(!predicate.validate(json!({})).ok().unwrap());
    }

    #[test]
    fn check_not_exists() {
        let json_spec: Value = json!({"f": {"exists": false}});
        let predicate = serde_json::from_value::<JsonPredicate>(json_spec).ok().unwrap();

        assert!(!predicate.validate(json!({"f": 42})).ok().unwrap());
        assert!(predicate.validate(json!({"f": null})).ok().unwrap());
        assert!(predicate.validate(json!({})).ok().unwrap());
    }

    #[test]
    fn check_in() {
        let json_spec: Value = json!({"f": {"[_]": ["1", 2, true]}});
        let predicate = serde_json::from_value::<JsonPredicate>(json_spec).ok().unwrap();

        assert!(predicate.validate(json!({"f": "1"})).ok().unwrap());
        assert!(predicate.validate(json!({"f": 2})).ok().unwrap());
        assert!(predicate.validate(json!({"f": true})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": "2"})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": 1})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": false})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": []})).ok().unwrap());
        assert!(predicate.validate(json!({"f": ["1"]})).ok().unwrap());
        assert!(predicate.validate(json!({"f": [2]})).ok().unwrap());
        assert!(predicate.validate(json!({"f": [true]})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": [1]})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": {}})).ok().unwrap());
    }

    #[test]
    fn check_not_in() {
        let json_spec: Value = json!({"f": {"![_]": ["1", 2, true]}});
        let predicate = serde_json::from_value::<JsonPredicate>(json_spec).ok().unwrap();

        assert!(!predicate.validate(json!({"f": "1"})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": 2})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": true})).ok().unwrap());
        assert!(predicate.validate(json!({"f": "2"})).ok().unwrap());
        assert!(predicate.validate(json!({"f": 1})).ok().unwrap());
        assert!(predicate.validate(json!({"f": false})).ok().unwrap());
        assert!(predicate.validate(json!({"f": []})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": ["1"]})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": [2]})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": [true]})).ok().unwrap());
        assert!(predicate.validate(json!({"f": [1]})).ok().unwrap());
        assert!(predicate.validate(json!({"f": {}})).ok().unwrap());
    }

    #[test]
    fn check_all_in() {
        let json_spec: Value = json!({"f": {"&[_]": ["1", 2, true]}});
        let predicate = serde_json::from_value::<JsonPredicate>(json_spec).ok().unwrap();

        assert!(!predicate.validate(json!({"f": 1})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": "1"})).ok().unwrap());
        assert!(predicate.validate(json!({"f": ["1", 2, true]})).ok().unwrap());
        assert!(predicate.validate(json!({"f": [2, "1", true]})).ok().unwrap());
        assert!(!predicate.validate(json!({"f": [2, "1", false]})).ok().unwrap());
    }
}