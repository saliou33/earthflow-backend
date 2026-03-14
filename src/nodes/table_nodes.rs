use async_trait::async_trait;
use serde_json::Value;
use crate::nodes::{NodeHandler, NodeMetadata, PortMetadata, PortMap, NodeContext};

pub struct TableJoinNode;

#[async_trait]
impl NodeHandler for TableJoinNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "table.join".to_string(),
            label: "Table Join".to_string(),
            description: "Join two tables by a key field".to_string(),
            inputs: vec![
                PortMetadata { id: "left".to_string(), label: "Left Table".to_string(), port_type: "table".to_string() },
                PortMetadata { id: "right".to_string(), label: "Right Table".to_string(), port_type: "table".to_string() },
            ],
            outputs: vec![PortMetadata { id: "output".to_string(), label: "Joined Table".to_string(), port_type: "table".to_string() }],
        }
    }
    async fn execute(&self, _ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let asset = inputs.get("left").cloned().ok_or("Input 'left' missing")?;
        let mut outputs = PortMap::new();
        outputs.insert("output".to_string(), asset);
        Ok(outputs)
    }
}

pub struct TableFilterNode;
pub struct TableAggregateNode;
pub struct TableRenameNode;
pub struct TableFormulaNode;
pub struct TableSortNode;

#[async_trait]
impl NodeHandler for TableFilterNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "table.filter".to_string(),
            label: "Table Filter".to_string(),
            description: "Filter rows by expression".to_string(),
            inputs: vec![PortMetadata { id: "input".to_string(), label: "Table".to_string(), port_type: "table".to_string() }],
            outputs: vec![PortMetadata { id: "output".to_string(), label: "Filtered Table".to_string(), port_type: "table".to_string() }],
        }
    }
    async fn execute(&self, _ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let asset = inputs.get("input").cloned().ok_or("Input 'input' missing")?;
        let mut outputs = PortMap::new();
        outputs.insert("output".to_string(), asset);
        Ok(outputs)
    }
}
