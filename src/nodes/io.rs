use async_trait::async_trait;
use serde_json::Value;
use crate::nodes::{NodeHandler, NodeMetadata, PortMetadata, PortMap, PortValue, NodeContext};
use crate::models::asset::Asset;
use uuid::Uuid;

pub struct VectorInputNode;

#[async_trait]
impl NodeHandler for VectorInputNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "vector_input".to_string(),
            label: "Vector Asset".to_string(),
            description: "Load a vector dataset from the Asset Manager".to_string(),
            inputs: vec![],
            outputs: vec![PortMetadata {
                id: "output".to_string(),
                label: "Vector Data".to_string(),
                port_type: "vector".to_string(),
            }],
        }
    }

    async fn execute(&self, ctx: &NodeContext, _inputs: &PortMap, params: &Value) -> Result<PortMap, String> {
        let asset_id_str = params["assetId"].as_str().ok_or("Missing parameter: assetId")?;
        let asset_id = Uuid::parse_str(asset_id_str).map_err(|_| "Invalid assetId format")?;

        // Resolve asset from database
        let asset = sqlx::query_as::<_, Asset>("SELECT * FROM assets WHERE id = $1")
            .bind(asset_id)
            .fetch_one(&ctx.pool)
            .await
            .map_err(|e| format!("Failed to fetch asset {}: {}", asset_id, e))?;

        // For Milestone 5, we output the asset metadata and storage URI.
        // Downstream nodes or the Data Panel will use this to stream data.
        let mut outputs = PortMap::new();
        
        // We pass the whole asset as a JSON blob in the PortValue::Json for now
        // This allows the frontend to have Name, Type, and Storage URI.
        outputs.insert("output".to_string(), PortValue::Json(serde_json::to_value(asset).unwrap()));
        
        Ok(outputs)
    }
}
