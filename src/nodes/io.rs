use async_trait::async_trait;
use serde_json::Value;
use crate::nodes::{NodeHandler, NodeMetadata, PortMetadata, PortMap, PortValue, NodeContext};
use crate::nodes::utils::upload_geojson;
use crate::models::asset::Asset;
use uuid::Uuid;

pub struct AssetInputNode;

#[async_trait]
impl NodeHandler for AssetInputNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "source.asset".to_string(),
            label: "Asset".to_string(),
            description: "Load an asset (Vector/Raster) from the Asset Manager".to_string(),
            inputs: vec![],
            outputs: vec![PortMetadata {
                id: "output".to_string(),
                label: "Data".to_string(),
                port_type: "asset".to_string(),
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

        let mut outputs = PortMap::new();
        outputs.insert("output".to_string(), PortValue::Asset(asset));
        
        Ok(outputs)
    }
}

pub struct DrawNode;

#[async_trait]
impl NodeHandler for DrawNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "io.draw".to_string(),
            label: "Draw Data".to_string(),
            description: "Interactive geometry drawing".to_string(),
            inputs: vec![],
            outputs: vec![PortMetadata {
                id: "output".to_string(),
                label: "Geometry".to_string(),
                port_type: "asset".to_string(),
            }],
        }
    }

    async fn execute(&self, ctx: &NodeContext, _inputs: &PortMap, params: &Value) -> Result<PortMap, String> {
        let geometry = params["geometry"].clone();
        if geometry.is_null() {
            return Err("Missing parameter: geometry".to_string());
        }

        let name = params["label"].as_str().unwrap_or("Drawn Geometry");
        // Using a dummy owner_id for now - in production this would be from the user context
        let owner_id = Uuid::nil();
        
        let asset = upload_geojson(ctx, name, &geometry, owner_id, "execution", ctx.execution_id).await?;

        let mut outputs = PortMap::new();
        outputs.insert("output".to_string(), PortValue::Asset(asset));
        
        Ok(outputs)
    }
}
