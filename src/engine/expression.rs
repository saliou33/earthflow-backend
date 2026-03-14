use rhai::{Engine, Scope, Dynamic};
use std::collections::HashMap;
use crate::nodes::PortValue;

pub struct ExpressionEngine {
    engine: Engine,
}

impl ExpressionEngine {
    pub fn new() -> Self {
        let engine = Engine::new();
        
        // Register custom GIS-like functions here in the future
        // e.g. engine.register_fn("ST_Buffer", ...);

        Self { engine }
    }

    pub fn eval(&self, script: &str, inputs: &HashMap<String, PortValue>) -> Result<PortValue, String> {
        let mut scope = Scope::new();
        
        // Map inputs to scope
        for (name, value) in inputs {
            let dynamic_val = match value {
                PortValue::Scalar(f) => Dynamic::from_float(*f),
                PortValue::Integer(i) => Dynamic::from_int(*i),
                PortValue::String(s) => Dynamic::from(s.clone()),
                PortValue::Boolean(b) => Dynamic::from_bool(*b),
                PortValue::Json(v) => {
                    // Convert JSON to Rhai Dynamic (simplified for POC)
                    Dynamic::from(v.to_string())
                },
                PortValue::Asset(a) => {
                    Dynamic::from(serde_json::to_string(a).unwrap_or_default())
                }
            };
            scope.push(name.clone(), dynamic_val);
        }

        self.engine.eval_with_scope::<Dynamic>(&mut scope, script)
            .map(|res| {
                if res.is_float() {
                    PortValue::Scalar(res.as_float().unwrap())
                } else if res.is_int() {
                    PortValue::Integer(res.as_int().unwrap())
                } else if res.is_bool() {
                    PortValue::Boolean(res.as_bool().unwrap())
                } else {
                    PortValue::String(res.to_string())
                }
            })
            .map_err(|e| format!("Expression error: {}", e))
    }
}
