pub mod core;
pub mod io;
pub mod utils;
pub mod vector_nodes;
pub mod raster_nodes;
pub mod analysis_nodes;
pub mod table_nodes;
pub mod style_nodes;
pub mod postgres_nodes;

use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::Value;

pub const PORT_INPUT: &str = "input";
pub const PORT_OUTPUT: &str = "output";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum PortValue {
    Scalar(f64),
    Integer(i64),
    String(String),
    Boolean(bool),
    Asset(crate::models::asset::Asset),
    Json(Value),
    Array(Vec<PortValue>),
}

impl PortValue {
    pub fn as_asset(&self) -> Result<&crate::models::asset::Asset, String> {
        match self {
            PortValue::Asset(a) => Ok(a),
            PortValue::Array(arr) => {
                if arr.len() == 1 {
                    arr[0].as_asset()
                } else {
                    Err("Expected single asset, but found multiple".to_string())
                }
            }
            _ => Err("Value is not an Asset".to_string()),
        }
    }

    pub fn as_assets(&self) -> Result<Vec<&crate::models::asset::Asset>, String> {
        match self {
            PortValue::Asset(a) => Ok(vec![a]),
            PortValue::Array(arr) => {
                let mut assets = Vec::new();
                for val in arr {
                    assets.push(val.as_asset()?);
                }
                Ok(assets)
            }
            _ => Err("Value is not an Asset or Asset Array".to_string()),
        }
    }

    pub fn as_float(&self) -> Result<f64, String> {
        match self {
            PortValue::Scalar(f) => Ok(*f),
            PortValue::Integer(i) => Ok(*i as f64),
            PortValue::String(s) => s.parse::<f64>().map_err(|e| e.to_string()),
            _ => Err("Value is not a number".to_string()),
        }
    }
}

pub type PortMap = HashMap<String, PortValue>;

#[derive(Clone)]
pub struct NodeContext {
    pub pool: sqlx::PgPool,
    pub s3_client: aws_sdk_s3::Client,
    pub execution_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeMetadata {
    pub type_id: String,
    pub label: String,
    pub description: String,
    pub inputs: Vec<PortMetadata>,
    pub outputs: Vec<PortMetadata>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PortMetadata {
    pub id: String,
    pub label: String,
    pub port_type: String,
}

#[async_trait]
pub trait NodeHandler: Send + Sync {
    fn metadata(&self) -> NodeMetadata;
    
    async fn execute(
        &self,
        ctx: &NodeContext,
        inputs: &PortMap,
        params: &Value,
    ) -> Result<PortMap, String>;
}

pub struct NodeRegistry {
    handlers: HashMap<String, Box<dyn NodeHandler>>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(&mut self, handler: Box<dyn NodeHandler>) {
        let meta = handler.metadata();
        self.handlers.insert(meta.type_id, handler);
    }

    pub fn get(&self, type_id: &str) -> Option<&dyn NodeHandler> {
        self.handlers.get(type_id).map(|b| b.as_ref())
    }
}
