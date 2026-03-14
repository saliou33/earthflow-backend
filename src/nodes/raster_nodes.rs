use async_trait::async_trait;
use serde_json::Value;
use crate::nodes::{NodeHandler, NodeMetadata, PortMetadata, PortMap, PortValue, NodeContext, PORT_INPUT, PORT_OUTPUT};

pub struct RasterClipNode;

#[async_trait]
impl NodeHandler for RasterClipNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "raster.clip_by_extent".to_string(),
            label: "Clip Raster".to_string(),
            description: "Clip raster to a bounding box".to_string(),
            inputs: vec![
                PortMetadata { id: PORT_INPUT.to_string(), label: "Data".to_string(), port_type: "any".to_string() },
            ],
            outputs: vec![PortMetadata { id: PORT_OUTPUT.to_string(), label: "Output".to_string(), port_type: "raster".to_string() }],
        }
    }
    async fn execute(&self, _ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let input_val = inputs.get(PORT_INPUT).ok_or("Missing input: input")?;
        // For Clip, we expect an array of [Raster, BBox] or just the Raster
        // For now, let's just extract the first asset as the "item to clip"
        let asset = input_val.as_asset()?;
        let mut outputs = PortMap::new();
        outputs.insert(PORT_OUTPUT.to_string(), PortValue::Asset(asset.clone()));
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
            inputs: vec![PortMetadata { id: PORT_INPUT.to_string(), label: "Raster".to_string(), port_type: "raster".to_string() }],
            outputs: vec![PortMetadata { id: PORT_OUTPUT.to_string(), label: "Table".to_string(), port_type: "table".to_string() }],
        }
    }
    async fn execute(&self, _ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let _asset = inputs.get(PORT_INPUT).ok_or("Input 'input' missing")?.as_asset()?;
        Ok(PortMap::new())
    }
}

pub struct HillshadeNode;
pub struct SlopeNode;
pub struct AspectNode;
pub struct PolygonizeNode;
pub struct BandMathNode;
pub struct ReclassifyNode;
pub struct MergeRasterNode;
pub struct ResampleNode;

#[async_trait]
impl NodeHandler for HillshadeNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "raster.hillshade".to_string(),
            label: "Hillshade".to_string(),
            description: "Compute hillshade from DEM".to_string(),
            inputs: vec![PortMetadata { id: PORT_INPUT.to_string(), label: "DEM".to_string(), port_type: "raster".to_string() }],
            outputs: vec![PortMetadata { id: PORT_OUTPUT.to_string(), label: "Hillshade".to_string(), port_type: "raster".to_string() }],
        }
    }
    async fn execute(&self, _ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let asset = inputs.get(PORT_INPUT).ok_or("Missing input: input")?.as_asset()?;
        let mut outputs = PortMap::new();
        outputs.insert(PORT_OUTPUT.to_string(), PortValue::Asset(asset.clone()));
        Ok(outputs)
    }
}
