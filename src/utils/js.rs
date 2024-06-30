use crate::utils::{IntoBD, IntoUSize};
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive, Zero};
use serde_json::{Number, Value, json};
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};

pub mod optic;

impl IntoBD for &Number {
    fn to_big_decimal(self) -> BigDecimal {
        self.as_i64().map(|i| BigDecimal::from(i))
            .or(self.as_u64().map(|u| BigDecimal::from(u)))
            .or(self.as_f64().and_then(|f| BigDecimal::from_f64(f)))
            .unwrap_or(BigDecimal::zero())
    }
}

impl IntoUSize for &Number {
    fn to_usize(self) -> usize {
        self.as_i64().and_then(|i| i.to_usize())
            .or(self.as_u64().and_then(|u| u.to_usize()))
            .or(self.as_f64().and_then(|f| f.to_usize()))
            .unwrap_or(0)
    }
}

/// This Json representation explicitly owns number representations
pub enum Jsn {
    Null,
    Bool(bool),
    Signed(i64),
    Float(f64),
    String(String),
    Array(Vec<Jsn>),
    Object(HashMap<String, Jsn>),
}

impl Jsn {
    pub fn is_string(&self) -> bool {
        match self {
            Jsn::String(_) => true,
            _ => false
        }
    }
}

impl From<Value> for Jsn {
    fn from(value: Value) -> Self {
        match value {
            Value::Null => Jsn::Null,
            Value::Bool(boolean) => Jsn::Bool(boolean),
            Value::Number(n) if n.is_i64() => Jsn::Signed(n.as_i64().unwrap()),
            Value::Number(n) if n.is_u64() => {
                if n.as_u64().unwrap() <= i64::MAX as u64 {
                    Jsn::Signed(n.as_u64().unwrap() as i64)
                } else {
                    Jsn::Float(n.as_f64().unwrap())
                }
            },
            Value::Number(n) if n.is_f64() => Jsn::Float(n.as_f64().unwrap()),
            Value::String(str) => Jsn::String(str),
            Value::Array(els) => Jsn::Array(els.iter().map(|el| el.clone().clone().into()).collect::<Vec<_>>()),
            Value::Object(flds) => Jsn::Object(flds.iter().map(|(k, v)| (k.clone(), v.clone().into())).collect::<HashMap<_, _>>()),
            _ => panic!("This should never happen")
        }
    }
}

impl Display for Jsn {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Jsn::Null => write!(fmt, "{}", "null"),
            Jsn::Bool(b) => write!(fmt, "{}", b),
            Jsn::Signed(i) => write!(fmt, "{}", i),
            Jsn::Float(f) => write!(fmt, "{}", f),
            Jsn::String(s) => write!(fmt, "\"{}\"", s.escape_default()),
            Jsn::Array(els) => write!(fmt, "{:?}", els),
            Jsn::Object(ps) => {
                write!(fmt, "{{")?;
                for (k,v) in ps {
                    write!(fmt, "\"{}\": {:?}", k.escape_default(), v)?;
                }
                write!(fmt, "}}")
            }
        }
    }
}

impl Debug for Jsn {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{}", self)
    }
}

trait ValueExtInternal {
    fn modify_field_in_place(&mut self, name: &String, modify: impl Fn(&mut Value) -> (), default: impl Fn() -> Value);
    fn modify_position_in_place(&mut self, index: usize, modify: impl Fn(&mut Value) -> (), default: impl Fn() -> Value);
    fn traverse_in_place(&mut self, modify: impl Fn(&mut Value) -> (), default: impl Fn() -> Value);
    fn verify_field(&self, name: &String) -> bool;
    fn verify_position(&self, index: usize) -> bool;
    fn field(&self, field_name: &String) -> Option<&Value>;
    fn at_index(&self, index: usize) -> Option<&Value>;
    fn remove_field(&mut self, field_name: &String);
    fn remove_at_index(&mut self, index: usize);
}

impl ValueExtInternal for Value {
    fn modify_field_in_place(&mut self, name: &String, modify: impl Fn(&mut Value) -> (), default: impl Fn() -> Value) {
        match self {
            Value::Object(jo) => {
                if let Some(jv) = jo.get_mut(name) {
                    modify(jv);
                } else {
                    let mut new_val = default();
                    modify(&mut new_val);
                    jo.insert(name.clone(), new_val);
                }
            }
            _ => {
                let mut new_val = default();
                modify(&mut new_val);
                *self = json!({ name: new_val })
            }
        }
    }

    fn modify_position_in_place(&mut self, idx: usize, modify: impl Fn(&mut Value) -> (), default: impl Fn() -> Value) {
        match self {
            Value::Array(ja) => {
                if ja.len() <= idx {
                    let items_to_add = idx - ja.len() + 1;
                    ja.append(&mut vec![Value::Null; items_to_add]);
                }

                modify(ja.get_mut(idx).unwrap())
            }
            _ => {
                let mut new_val = default();
                modify(&mut new_val);

                let mut blank = vec![Value::Null; idx];
                blank.push(new_val);

                *self = Value::Array(blank)
            }
        }
    }

    fn traverse_in_place(&mut self, modify: impl Fn(&mut Value) -> (), default: impl Fn() -> Value) {
        match self {
            Value::Array(ja) => {
                for jv in ja.iter_mut() {
                    modify(jv);
                }
            }
            _ => {
                let mut new_val = default();
                modify(&mut new_val);
                *self = json!([new_val])
            }
        }
    }

    fn verify_field(&self, name: &String) -> bool {
        self.as_object().map(|m| m.contains_key(name)).unwrap_or(false)
    }

    fn verify_position(&self, idx: usize) -> bool {
        self.as_array().map(|a| a.len() > idx).unwrap_or(false)
    }

    fn field(&self, field_name: &String) -> Option<&Value> {
        match self {
            Value::Object(map) => map.get(field_name),
            _ => None,
        }
    }

    fn at_index(&self, index: usize) -> Option<&Value> {
        match self {
            Value::Array(vec) => vec.get(index),
            _ => None,
        }
    }

    fn remove_field(&mut self, field_name: &String) {
        if let Some(jmap) = self.as_object_mut() {
            jmap.remove(field_name);
        }
    }

    fn remove_at_index(&mut self, index: usize) {
        if let Some(jarr) = self.as_array_mut() {
            jarr.remove(index);
        }
    }
}

#[cfg(test)]
mod js_tests {
    use crate::utils::js::Jsn;
    use serde_json::json;

    #[test]
    fn check_jsn_debug() {
        let jsn: Jsn = json!({"a": ["b", 3, false, null]}).into();
        let sut = format!("{:?}", jsn);

        assert_eq!(sut, r#"{"a": ["b", 3, false, null]}"#)
    }
}