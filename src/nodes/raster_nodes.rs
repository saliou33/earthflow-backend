use async_trait::async_trait;
use serde_json::Value;
use crate::nodes::{NodeHandler, NodeMetadata, PortMetadata, PortMap, NodeContext};

pub struct RasterClipNode;

#[async_trait]
impl NodeHandler for RasterClipNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "raster.clip_by_extent".to_string(),
            label: "Clip Raster".to_string(),
            description: "Clip raster to a bounding box".to_string(),
            inputs: vec![
                PortMetadata { id: "raster".to_string(), label: "Raster".to_string(), port_type: "raster".to_string() },
                PortMetadata { id: "bbox".to_string(), label: "BBox".to_string(), port_type: "bbox".to_string() },
            ],
            outputs: vec![PortMetadata { id: "output".to_string(), label: "Output".to_string(), port_type: "raster".to_string() }],
        }
    }
    async fn execute(&self, _ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let asset = inputs.get("raster").cloned().ok_or("Input 'raster' missing")?;
        let mut outputs = PortMap::new();
        outputs.insert("output".to_string(), asset);
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
            outputs: vec![PortMetadata { id: "output".to_string(), label: "Table".to_string(), port_type: "table".to_string() }],
        }
    }
    async fn execute(&self, _ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let _asset = inputs.get("raster").cloned().ok_or("Input 'raster' missing")?;
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
            inputs: vec![PortMetadata { id: "raster".to_string(), label: "DEM".to_string(), port_type: "raster".to_string() }],
            outputs: vec![PortMetadata { id: "output".to_string(), label: "Hillshade".to_string(), port_type: "raster".to_string() }],
        }
    }
    async fn execute(&self, _ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let asset = inputs.get("raster").cloned().ok_or("Input 'raster' missing")?;
        let mut outputs = PortMap::new();
        outputs.insert("output".to_string(), asset);
        Ok(outputs)
    }
}
