use async_trait::async_trait;
use serde_json::Value;
use crate::nodes::{NodeHandler, NodeMetadata, PortMetadata, PortMap, NodeContext};

pub struct KernelDensityNode;

#[async_trait]
impl NodeHandler for KernelDensityNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "analysis.kernel_density".to_string(),
            label: "Kernel Density".to_string(),
            description: "Heatmap from point features".to_string(),
            inputs: vec![PortMetadata { id: "input".to_string(), label: "Points".to_string(), port_type: "vector".to_string() }],
            outputs: vec![PortMetadata { id: "output".to_string(), label: "Raster".to_string(), port_type: "raster".to_string() }],
        }
    }
    async fn execute(&self, _ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let asset = inputs.get("input").cloned().ok_or("Input 'input' missing")?;
        let mut outputs = PortMap::new();
        outputs.insert("output".to_string(), asset);
        Ok(outputs)
    }
}

pub struct ViewshedNode;
pub struct WatershedNode;
pub struct VoronoiNode;
pub struct ClusterNode;

#[async_trait]
impl NodeHandler for ViewshedNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "analysis.viewshed".to_string(),
            label: "Viewshed".to_string(),
            description: "Compute visibility from observer".to_string(),
            inputs: vec![
                PortMetadata { id: "dem".to_string(), label: "DEM".to_string(), port_type: "raster".to_string() },
                PortMetadata { id: "observer".to_string(), label: "Observer".to_string(), port_type: "vector".to_string() },
            ],
            outputs: vec![PortMetadata { id: "output".to_string(), label: "Viewshed".to_string(), port_type: "raster".to_string() }],
        }
    }
    async fn execute(&self, _ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let asset = inputs.get("dem").cloned().ok_or("Input 'dem' missing")?;
        let mut outputs = PortMap::new();
        outputs.insert("output".to_string(), asset);
        Ok(outputs)
    }
}
