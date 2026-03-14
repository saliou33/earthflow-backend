use rhai::{Engine, Scope, Dynamic};
use std::collections::HashMap;

pub struct ExpressionEngine {
    engine: Engine,
}

impl ExpressionEngine {
    pub fn new() -> Self {
        let engine = Engine::new();
        Self { engine }
    }

    pub fn eval(&self, script: &str, inputs: &HashMap<String, crate::nodes::PortValue>) -> Result<crate::nodes::PortValue, String> {
        let mut scope = Scope::new();
        
        for (name, value) in inputs {
            let dynamic_val = match value {
                crate::nodes::PortValue::Scalar(f) => Dynamic::from_float(*f),
                crate::nodes::PortValue::Integer(i) => Dynamic::from_int(*i),
                crate::nodes::PortValue::String(s) => Dynamic::from(s.clone()),
                crate::nodes::PortValue::Boolean(b) => Dynamic::from_bool(*b),
                crate::nodes::PortValue::Json(v) => Dynamic::from(v.to_string()),
                crate::nodes::PortValue::Asset(a) => Dynamic::from(serde_json::to_string(a).unwrap_or_default()),
                crate::nodes::PortValue::Array(_) => Dynamic::from(serde_json::to_string(value).unwrap_or_default()),
            };
            scope.push(name.clone(), dynamic_val);
        }

        self.engine.eval_with_scope::<Dynamic>(&mut scope, script)
            .map(|res| {
                if res.is_float() {
                    crate::nodes::PortValue::Scalar(res.as_float().unwrap())
                } else if res.is_int() {
                    crate::nodes::PortValue::Integer(res.as_int().unwrap())
                } else if res.is_bool() {
                    crate::nodes::PortValue::Boolean(res.as_bool().unwrap())
                } else {
                    crate::nodes::PortValue::String(res.to_string())
                }
            })
            .map_err(|e| format!("Expression error: {}", e))
    }
}
