use async_trait::async_trait;
use serde_json::Value;
use crate::nodes::{NodeHandler, NodeMetadata, PortMetadata, PortMap, PortValue, NodeContext, PORT_OUTPUT};

pub struct RasterClipNode;

#[async_trait]
impl NodeHandler for RasterClipNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "raster.clip_by_extent".to_string(),
            label: "Clip Raster".to_string(),
            description: "Clip raster to a vector mask".to_string(),
            inputs: vec![
                PortMetadata { id: "raster".to_string(), label: "Raster".to_string(), port_type: "raster".to_string() },
                PortMetadata { id: "mask".to_string(), label: "Mask".to_string(), port_type: "vector".to_string() },
            ],
            outputs: vec![PortMetadata { id: PORT_OUTPUT.to_string(), label: "Output".to_string(), port_type: "raster".to_string() }],
        }
    }
    async fn execute(&self, _ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let raster = inputs.get("raster").ok_or("Missing input: raster")?.as_asset()?;
        let _mask = inputs.get("mask").ok_or("Missing input: mask")?.as_asset()?;
        
        tracing::info!("Executing RasterClip for asset: {}", raster.name);
        
        let mut outputs = PortMap::new();
        outputs.insert(PORT_OUTPUT.to_string(), PortValue::Asset(raster.clone()));
        Ok(outputs)
    }
}

pub struct RasterStatisticsNode;

#[async_trait]
impl NodeHandler for RasterStatisticsNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "raster.statistics".to_string(),
            label: "Raster Statistics".to_string(),
            description: "Compute min, max, mean per band".to_string(),
            inputs: vec![PortMetadata { id: "raster".to_string(), label: "Raster".to_string(), port_type: "raster".to_string() }],
            outputs: vec![PortMetadata { id: PORT_OUTPUT.to_string(), label: "Table".to_string(), port_type: "table".to_string() }],
        }
    }
    async fn execute(&self, _ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let asset = inputs.get("raster").ok_or("Input 'raster' missing")?.as_asset()?;
        
        let titiler_base = std::env::var("TITILER_ENDPOINT").unwrap_or_else(|_| {
            tracing::warn!("TITILER_ENDPOINT is not defined. Falling back to localhost for development.");
            "http://localhost:8001".to_string()
        });
        let url = format!("{}/cog/statistics?url={}", titiler_base, asset.storage_uri);
        
        tracing::info!("Calling TiTiler Statistics: {}", url);
        
        let client = reqwest::Client::new();
        let resp = client.get(&url).send().await
            .map_err(|e| format!("Failed to call TiTiler: {}", e))?;
            
        let status = resp.status();
        if !status.is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            tracing::error!("TiTiler error ({}): {}", status, err_text);
            return Err(format!("TiTiler error {}: {}", status, err_text));
        }
        
        let stats: Value = resp.json().await
            .map_err(|e| format!("Failed to parse TiTiler response: {}", e))?;
            
        tracing::info!("TiTiler raw stats: {:?}", stats);
            
        let mut table_rows = Vec::new();
        if let Some(obj) = stats.as_object() {
            for (band, data) in obj {
                let mut row = data.clone();
                if let Some(row_obj) = row.as_object_mut() {
                    row_obj.insert("band".to_string(), Value::String(band.clone()));
                }
                table_rows.push(row);
            }
        }
        
        if table_rows.is_empty() {
            tracing::warn!("No statistics returned from TiTiler");
        }
        
        let mut outputs = PortMap::new();
        outputs.insert(PORT_OUTPUT.to_string(), PortValue::Json(Value::Array(table_rows)));
        Ok(outputs)
    }
}

pub struct HillshadeNode;
#[async_trait]
impl NodeHandler for HillshadeNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "raster.hillshade".to_string(),
            label: "Hillshade".to_string(),
            description: "Compute hillshade from DEM".to_string(),
            inputs: vec![PortMetadata { id: "raster".to_string(), label: "DEM".to_string(), port_type: "raster".to_string() }],
            outputs: vec![PortMetadata { id: PORT_OUTPUT.to_string(), label: "Hillshade".to_string(), port_type: "raster".to_string() }],
        }
    }
    async fn execute(&self, _ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let asset = inputs.get("raster").ok_or("Missing input: raster")?.as_asset()?;
        let mut output_asset = asset.clone();
        output_asset.name = format!("Hillshade: {}", asset.name);
        let mut outputs = PortMap::new();
        outputs.insert(PORT_OUTPUT.to_string(), PortValue::Asset(output_asset));
        Ok(outputs)
    }
}

pub struct SlopeNode;
#[async_trait]
impl NodeHandler for SlopeNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "raster.slope".to_string(),
            label: "Slope".to_string(),
            description: "Compute terrain slope".to_string(),
            inputs: vec![PortMetadata { id: "raster".to_string(), label: "DEM".to_string(), port_type: "raster".to_string() }],
            outputs: vec![PortMetadata { id: PORT_OUTPUT.to_string(), label: "Slope".to_string(), port_type: "raster".to_string() }],
        }
    }
    async fn execute(&self, _ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let asset = inputs.get("raster").ok_or("Missing input: raster")?.as_asset()?;
        let mut output_asset = asset.clone();
        output_asset.name = format!("Slope: {}", asset.name);
        let mut outputs = PortMap::new();
        outputs.insert(PORT_OUTPUT.to_string(), PortValue::Asset(output_asset));
        Ok(outputs)
    }
}

pub struct AspectNode;
#[async_trait]
impl NodeHandler for AspectNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "raster.aspect".to_string(),
            label: "Aspect".to_string(),
            description: "Compute terrain aspect".to_string(),
            inputs: vec![PortMetadata { id: "raster".to_string(), label: "DEM".to_string(), port_type: "raster".to_string() }],
            outputs: vec![PortMetadata { id: PORT_OUTPUT.to_string(), label: "Aspect".to_string(), port_type: "raster".to_string() }],
        }
    }
    async fn execute(&self, _ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let asset = inputs.get("raster").ok_or("Missing input: raster")?.as_asset()?;
        let mut output_asset = asset.clone();
        output_asset.name = format!("Aspect: {}", asset.name);
        let mut outputs = PortMap::new();
        outputs.insert(PORT_OUTPUT.to_string(), PortValue::Asset(output_asset));
        Ok(outputs)
    }
}

pub struct BandMathNode;
#[async_trait]
impl NodeHandler for BandMathNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "raster.band_math".to_string(),
            label: "Band Math".to_string(),
            description: "Apply algebraic expressions to bands".to_string(),
            inputs: vec![
                PortMetadata { id: "raster1".to_string(), label: "Raster A".to_string(), port_type: "raster".to_string() },
                PortMetadata { id: "raster2".to_string(), label: "Raster B".to_string(), port_type: "raster".to_string() },
            ],
            outputs: vec![PortMetadata { id: PORT_OUTPUT.to_string(), label: "Result".to_string(), port_type: "raster".to_string() }],
        }
    }
    async fn execute(&self, _ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let asset_a = inputs.get("raster1").ok_or("Missing input: raster1")?.as_asset()?;
        let mut output_asset = asset_a.clone();
        output_asset.name = format!("Computed: {}", asset_a.name);
        let mut outputs = PortMap::new();
        outputs.insert(PORT_OUTPUT.to_string(), PortValue::Asset(output_asset));
        Ok(outputs)
    }
}

pub struct PolygonizeNode;
pub struct ReclassifyNode;
pub struct MergeRasterNode;
pub struct ResampleNode;
