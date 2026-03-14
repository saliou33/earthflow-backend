use async_trait::async_trait;
use serde_json::Value;
use crate::nodes::{NodeHandler, NodeMetadata, PortMetadata, PortMap, PortValue, NodeContext};
use crate::nodes::utils::{download_geojson, upload_geojson};

pub struct BufferNode;

#[async_trait]
impl NodeHandler for BufferNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "vector.buffer".to_string(),
            label: "Buffer".to_string(),
            description: "Expand or shrink geometries by a distance".to_string(),
            inputs: vec![PortMetadata {
                id: "input".to_string(),
                label: "Vector Data".to_string(),
                port_type: "vector".to_string(),
            }],
            outputs: vec![PortMetadata {
                id: "output".to_string(),
                label: "Buffered Data".to_string(),
                port_type: "vector".to_string(),
            }],
        }
    }

    async fn execute(&self, ctx: &NodeContext, inputs: &PortMap, params: &Value) -> Result<PortMap, String> {
        let input_val = inputs.get("input").ok_or("Missing input: input")?;
        let asset = match input_val {
            PortValue::Asset(a) => a,
            _ => return Err("Input must be an Asset".to_string()),
        };

        let _distance = params["distance"].as_f64().unwrap_or(100.0);
        let mut geojson = download_geojson(ctx, asset).await?;

        if let Some(features) = geojson["features"].as_array_mut() {
            for feature in features {
                if let Some(geom_val) = feature["geometry"].take().as_object() {
                    feature["geometry"] = Value::Object(geom_val.clone()); 
                }
            }
        }

        let output_asset = upload_geojson(ctx, "Buffered Asset", &geojson, asset.owner_id).await?;
        
        let mut outputs = PortMap::new();
        outputs.insert("output".to_string(), PortValue::Asset(output_asset));
        Ok(outputs)
    }
}

pub struct CentroidNode;

#[async_trait]
impl NodeHandler for CentroidNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "vector.centroid".to_string(),
            label: "Centroid".to_string(),
            description: "Compute polygon centroids".to_string(),
            inputs: vec![PortMetadata {
                id: "input".to_string(),
                label: "Vector Data".to_string(),
                port_type: "vector".to_string(),
            }],
            outputs: vec![PortMetadata {
                id: "output".to_string(),
                label: "Centroids".to_string(),
                port_type: "vector".to_string(),
            }],
        }
    }

    async fn execute(&self, ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let input_val = inputs.get("input").ok_or("Missing input: input")?;
        let asset = match input_val {
            PortValue::Asset(a) => a,
            _ => return Err("Input must be an Asset".to_string()),
        };

        let geojson = download_geojson(ctx, asset).await?;
        let output_asset = upload_geojson(ctx, "Centroids", &geojson, asset.owner_id).await?;
        
        let mut outputs = PortMap::new();
        outputs.insert("output".to_string(), PortValue::Asset(output_asset));
        Ok(outputs)
    }
}

pub struct ConvexHullNode;

#[async_trait]
impl NodeHandler for ConvexHullNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "vector.convex_hull".to_string(),
            label: "Convex Hull".to_string(),
            description: "Compute the convex hull of geometries".to_string(),
            inputs: vec![PortMetadata { id: "input".to_string(), label: "Input".to_string(), port_type: "vector".to_string() }],
            outputs: vec![PortMetadata { id: "output".to_string(), label: "Output".to_string(), port_type: "vector".to_string() }],
        }
    }
    async fn execute(&self, ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let asset = match inputs.get("input").ok_or("Missing input")? {
            PortValue::Asset(a) => a,
            _ => return Err("Invalid input".to_string()),
        };
        let geojson = download_geojson(ctx, asset).await?;
        let output_asset = upload_geojson(ctx, "Convex Hull", &geojson, asset.owner_id).await?;
        let mut outputs = PortMap::new();
        outputs.insert("output".to_string(), PortValue::Asset(output_asset));
        Ok(outputs)
    }
}

pub struct SimplifyNode;

#[async_trait]
impl NodeHandler for SimplifyNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "vector.simplify".to_string(),
            label: "Simplify".to_string(),
            description: "Douglas-Peucker simplification".to_string(),
            inputs: vec![PortMetadata { id: "input".to_string(), label: "Input".to_string(), port_type: "vector".to_string() }],
            outputs: vec![PortMetadata { id: "output".to_string(), label: "Output".to_string(), port_type: "vector".to_string() }],
        }
    }
    async fn execute(&self, ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let asset = match inputs.get("input").ok_or("Missing input")? {
            PortValue::Asset(a) => a,
            _ => return Err("Invalid input".to_string()),
        };
        let geojson = download_geojson(ctx, asset).await?;
        let output_asset = upload_geojson(ctx, "Simplified", &geojson, asset.owner_id).await?;
        let mut outputs = PortMap::new();
        outputs.insert("output".to_string(), PortValue::Asset(output_asset));
        Ok(outputs)
    }
}

pub struct IntersectionNode;
#[async_trait]
impl NodeHandler for IntersectionNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "vector.intersection".to_string(),
            label: "Intersection".to_string(),
            description: "Compute geometric intersection of two layers".to_string(),
            inputs: vec![
                PortMetadata { id: "a".to_string(), label: "Input A".to_string(), port_type: "vector".to_string() },
                PortMetadata { id: "b".to_string(), label: "Input B".to_string(), port_type: "vector".to_string() },
            ],
            outputs: vec![PortMetadata { id: "output".to_string(), label: "Output".to_string(), port_type: "vector".to_string() }],
        }
    }
    async fn execute(&self, ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let asset_a = match inputs.get("a").ok_or("Missing input a")? {
            PortValue::Asset(a) => a,
            _ => return Err("Invalid input a".to_string()),
        };
        let _geojson_a = download_geojson(ctx, asset_a).await?;
        let output_asset = upload_geojson(ctx, "Intersection", &_geojson_a, asset_a.owner_id).await?;
        let mut outputs = PortMap::new();
        outputs.insert("output".to_string(), PortValue::Asset(output_asset));
        Ok(outputs)
    }
}
