use async_trait::async_trait;
use serde_json::{Value, json};
use geo::{Centroid, ConvexHull, BoundingRect};
use geo::geometry::*;
use crate::nodes::{NodeHandler, NodeMetadata, PortMetadata, PortMap, PortValue, NodeContext, PORT_INPUT, PORT_OUTPUT};
use crate::nodes::utils::{
    download_geojson, upload_geojson,
    geojson_to_geo, point_to_geojson, polygon_to_geojson,
    collect_all_coords, simplify_geojson_geom,
    evaluate_parameter,
};

pub struct BufferNode;

#[async_trait]
impl NodeHandler for BufferNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "vector.buffer".to_string(),
            label: "Buffer".to_string(),
            description: "Expand or shrink geometries by a distance".to_string(),
            inputs: vec![PortMetadata {
                id: PORT_INPUT.to_string(),
                label: "Vector Data".to_string(),
                port_type: "vector".to_string(),
            }],
            outputs: vec![PortMetadata {
                id: PORT_OUTPUT.to_string(),
                label: "Buffered Data".to_string(),
                port_type: "vector".to_string(),
            }],
        }
    }

    async fn execute(&self, ctx: &NodeContext, inputs: &PortMap, params: &Value) -> Result<PortMap, String> {
        let input_val = inputs.get(PORT_INPUT).ok_or("Missing input")?;
        let asset = input_val.as_asset()?;

        // Distance in degrees (approximate: ~0.001 deg ~= 100m at equator)
        let distance_meters = evaluate_parameter(&params["distance"], inputs)
            .and_then(|v| v.as_float())
            .unwrap_or(100.0);
        let distance_deg = distance_meters / 111_000.0;

        let mut geojson = download_geojson(ctx, asset).await?;

        if let Some(features) = geojson["features"].as_array_mut() {
            for feature in features.iter_mut() {
                let geom = &feature["geometry"];
                if let Some(geo_geom) = geojson_to_geo(geom) {
                    if let Some(bbox) = geo_geom.bounding_rect() {
                        // Expand the bounding box by distance_deg in all directions
                        let buffered = Rect::new(
                            Coord { x: bbox.min().x - distance_deg, y: bbox.min().y - distance_deg },
                            Coord { x: bbox.max().x + distance_deg, y: bbox.max().y + distance_deg },
                        );
                        let polygon = buffered.to_polygon();
                        feature["geometry"] = polygon_to_geojson(&polygon);
                    }
                }
            }
        }

        let output_asset = upload_geojson(ctx, "Buffered Asset", &geojson, asset.owner_id, "execution", ctx.execution_id).await?;
        
        let mut outputs = PortMap::new();
        outputs.insert(PORT_OUTPUT.to_string(), PortValue::Asset(output_asset));
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
                id: PORT_INPUT.to_string(),
                label: "Vector Data".to_string(),
                port_type: "vector".to_string(),
            }],
            outputs: vec![PortMetadata {
                id: PORT_OUTPUT.to_string(),
                label: "Centroids".to_string(),
                port_type: "vector".to_string(),
            }],
        }
    }

    async fn execute(&self, ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let input_val = inputs.get(PORT_INPUT).ok_or("Missing input")?;
        let asset = input_val.as_asset()?;

        let geojson = download_geojson(ctx, asset).await?;

        let mut centroid_features: Vec<Value> = Vec::new();

        if let Some(features) = geojson["features"].as_array() {
            for feature in features {
                let geom = &feature["geometry"];
                if let Some(geo_geom) = geojson_to_geo(geom) {
                    if let Some(centroid) = geo_geom.centroid() {
                        centroid_features.push(json!({
                            "type": "Feature",
                            "properties": feature["properties"].clone(),
                            "geometry": point_to_geojson(&centroid)
                        }));
                    }
                }
            }
        }

        if centroid_features.is_empty() {
            return Err("No valid geometries found to compute centroids from".to_string());
        }

        let output_geojson = json!({
            "type": "FeatureCollection",
            "features": centroid_features
        });

        let output_asset = upload_geojson(ctx, "Centroids", &output_geojson, asset.owner_id, "execution", ctx.execution_id).await?;
        
        let mut outputs = PortMap::new();
        outputs.insert(PORT_OUTPUT.to_string(), PortValue::Asset(output_asset));
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
            inputs: vec![PortMetadata { id: PORT_INPUT.to_string(), label: "Input".to_string(), port_type: "vector".to_string() }],
            outputs: vec![PortMetadata { id: PORT_OUTPUT.to_string(), label: "Output".to_string(), port_type: "vector".to_string() }],
        }
    }
    async fn execute(&self, ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let asset = inputs.get(PORT_INPUT).ok_or("Missing input")?.as_asset()?;
        let geojson = download_geojson(ctx, asset).await?;

        let all_coords = collect_all_coords(&geojson);
        if all_coords.is_empty() {
            return Err("No coordinates found to compute convex hull".to_string());
        }

        // Build a MultiPoint and compute its convex hull
        let multi_point = MultiPoint(all_coords.into_iter().map(Point).collect());
        let hull: Polygon<f64> = multi_point.convex_hull();

        let output_geojson = json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": {},
                "geometry": polygon_to_geojson(&hull)
            }]
        });

        let output_asset = upload_geojson(ctx, "Convex Hull", &output_geojson, asset.owner_id, "execution", ctx.execution_id).await?;
        let mut outputs = PortMap::new();
        outputs.insert(PORT_OUTPUT.to_string(), PortValue::Asset(output_asset));
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
            inputs: vec![PortMetadata { id: PORT_INPUT.to_string(), label: "Input".to_string(), port_type: "vector".to_string() }],
            outputs: vec![PortMetadata { id: PORT_OUTPUT.to_string(), label: "Output".to_string(), port_type: "vector".to_string() }],
        }
    }
    async fn execute(&self, ctx: &NodeContext, inputs: &PortMap, params: &Value) -> Result<PortMap, String> {
        let asset = inputs.get(PORT_INPUT).ok_or("Missing input")?.as_asset()?;
        let epsilon = params["epsilon"].as_f64().unwrap_or(0.001);
        let mut geojson = download_geojson(ctx, asset).await?;

        if let Some(features) = geojson["features"].as_array_mut() {
            for feature in features.iter_mut() {
                let geom = feature["geometry"].clone();
                feature["geometry"] = simplify_geojson_geom(&geom, epsilon);
            }
        }

        let output_asset = upload_geojson(ctx, "Simplified", &geojson, asset.owner_id, "execution", ctx.execution_id).await?;
        let mut outputs = PortMap::new();
        outputs.insert(PORT_OUTPUT.to_string(), PortValue::Asset(output_asset));
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
                PortMetadata { id: PORT_INPUT.to_string(), label: "Input Layers".to_string(), port_type: "vector".to_string() },
            ],
            outputs: vec![PortMetadata { id: PORT_OUTPUT.to_string(), label: "Output".to_string(), port_type: "vector".to_string() }],
        }
    }
    async fn execute(&self, ctx: &NodeContext, inputs: &PortMap, _params: &Value) -> Result<PortMap, String> {
        let assets = inputs.get(PORT_INPUT).ok_or("Missing input")?.as_assets()?;
        if assets.len() < 2 {
            return Err("Intersection requires at least two input layers".to_string());
        }
        let asset_a = assets[0];
        let asset_b = assets[1];

        let geojson_a = download_geojson(ctx, asset_a).await?;
        let geojson_b = download_geojson(ctx, asset_b).await?;

        // Compute the bounding box of layer B
        let coords_b = collect_all_coords(&geojson_b);
        if coords_b.is_empty() {
            return Err("Layer B has no valid geometries for intersection".to_string());
        }
        let min_x = coords_b.iter().map(|c| c.x).fold(f64::INFINITY, f64::min);
        let max_x = coords_b.iter().map(|c| c.x).fold(f64::NEG_INFINITY, f64::max);
        let min_y = coords_b.iter().map(|c| c.y).fold(f64::INFINITY, f64::min);
        let max_y = coords_b.iter().map(|c| c.y).fold(f64::NEG_INFINITY, f64::max);

        // Filter layer A features to those whose centroid falls within the BBOX of B
        let filtered_features: Vec<Value> = geojson_a["features"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter(|feature| {
                if let Some(geo) = geojson_to_geo(&feature["geometry"]) {
                    if let Some(centroid) = geo.centroid() {
                        return centroid.x() >= min_x && centroid.x() <= max_x
                            && centroid.y() >= min_y && centroid.y() <= max_y;
                    }
                }
                false
            })
            .cloned()
            .collect();

        let output_geojson = json!({
            "type": "FeatureCollection",
            "features": filtered_features
        });

        let output_asset = upload_geojson(ctx, "Intersection", &output_geojson, asset_a.owner_id, "execution", ctx.execution_id).await?;
        let mut outputs = PortMap::new();
        outputs.insert(PORT_OUTPUT.to_string(), PortValue::Asset(output_asset));
        Ok(outputs)
    }
}
