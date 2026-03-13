pub mod core;
pub mod io;

use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum PortValue {
    Scalar(f64),
    Integer(i64),
    String(String),
    Boolean(bool),
    // Placeholder for Milestone 4 (Raster/Vector)
    Json(Value),
}

pub type PortMap = HashMap<String, PortValue>;

#[derive(Clone)]
pub struct NodeContext {
    pub pool: sqlx::PgPool,
    pub s3_client: aws_sdk_s3::Client,
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
