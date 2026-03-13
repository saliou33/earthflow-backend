use async_trait::async_trait;
use serde_json::Value;
use crate::nodes::{NodeHandler, NodeMetadata, PortMetadata, PortMap, PortValue, NodeContext};
use crate::engine::expression::ExpressionEngine;

pub struct VariableNode;

#[async_trait]
impl NodeHandler for VariableNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "variable".to_string(),
            label: "Variable".to_string(),
            description: "A variable that can be referenced by other nodes".to_string(),
            inputs: vec![],
            outputs: vec![PortMetadata {
                id: "value".to_string(),
                label: "Value".to_string(),
                port_type: "any".to_string(),
            }],
        }
    }

    async fn execute(&self, _ctx: &NodeContext, _inputs: &PortMap, params: &Value) -> Result<PortMap, String> {
        let input_type = params["inputType"].as_str().unwrap_or("string");
        let val_raw = &params["value"];
        
        let port_val = match input_type {
            "float" => {
                let f = if val_raw.is_number() {
                    val_raw.as_f64().unwrap()
                } else {
                    val_raw.as_str().unwrap_or("0").parse::<f64>().map_err(|_| "Invalid float")?
                };
                PortValue::Scalar(f)
            },
            "int" => {
                let i = if val_raw.is_number() {
                    val_raw.as_i64().unwrap()
                } else {
                    val_raw.as_str().unwrap_or("0").parse::<i64>().map_err(|_| "Invalid integer")?
                };
                PortValue::Integer(i)
            },
            "bool" => {
                let b = if val_raw.is_boolean() {
                    val_raw.as_bool().unwrap()
                } else {
                    val_raw.as_str().unwrap_or("false").parse::<bool>().map_err(|_| "Invalid boolean")?
                };
                PortValue::Boolean(b)
            },
            _ => PortValue::String(val_raw.as_str().unwrap_or("").to_string()),
        };

        let mut outputs = PortMap::new();
        outputs.insert("value".to_string(), port_val);
        Ok(outputs)
    }
}

pub struct ExpressionNode {
    engine: ExpressionEngine,
}

impl ExpressionNode {
    pub fn new() -> Self {
        Self { engine: ExpressionEngine::new() }
    }
}

#[async_trait]
impl NodeHandler for ExpressionNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "expression".to_string(),
            label: "Expression".to_string(),
            description: "Evaluates a Rhai expression".to_string(),
            inputs: vec![PortMetadata {
                id: "in".to_string(),
                label: "Inputs".to_string(),
                port_type: "any".to_string(),
            }],
            outputs: vec![PortMetadata {
                id: "result".to_string(),
                label: "Result".to_string(),
                port_type: "any".to_string(),
            }],
        }
    }

    async fn execute(&self, _ctx: &NodeContext, inputs: &PortMap, params: &Value) -> Result<PortMap, String> {
        let script = params["expression"].as_str().ok_or("Missing parameter: expression")?;
        
        let result = self.engine.eval(script, inputs)?;
        
        let mut outputs = PortMap::new();
        outputs.insert("result".to_string(), result);
        Ok(outputs)
    }
}
