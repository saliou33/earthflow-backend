use async_trait::async_trait;
use serde_json::Value;
use crate::nodes::{NodeHandler, NodeMetadata, PortMetadata, PortMap, NodeContext};

pub struct SimpleFillNode;

#[async_trait]
impl NodeHandler for SimpleFillNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "style.simple_fill".to_string(),
            label: "Simple Fill".to_string(),
            description: "Flat color fill style".to_string(),
            inputs: vec![],
            outputs: vec![PortMetadata { id: "output".to_string(), label: "Style".to_string(), port_type: "style".to_string() }],
        }
    }
    async fn execute(&self, _ctx: &NodeContext, _inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        Ok(PortMap::new())
    }
}

pub struct ChoroplethNode;
pub struct HeatmapStyleNode;
pub struct GraduatedSymbolNode;
pub struct LabelNode;

#[async_trait]
impl NodeHandler for ChoroplethNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "style.choropleth".to_string(),
            label: "Choropleth".to_string(),
            description: "Data-driven color fill style".to_string(),
            inputs: vec![],
            outputs: vec![PortMetadata { id: "output".to_string(), label: "Style".to_string(), port_type: "style".to_string() }],
        }
    }
    async fn execute(&self, _ctx: &NodeContext, _inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        Ok(PortMap::new())
    }
}
