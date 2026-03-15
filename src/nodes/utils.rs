use aws_sdk_s3::primitives::ByteStream;
use crate::nodes::NodeContext;
use crate::models::asset::Asset;
use uuid::Uuid;
use serde_json::{Value, json};
use crate::nodes::{PortMap, PortValue};
use crate::engine::expression::ExpressionEngine;
use geo::Simplify;
use geo::geometry::*;

pub async fn download_geojson(ctx: &NodeContext, asset: &Asset) -> Result<Value, String> {
    if asset.asset_type != "VECTOR" {
        return Err(format!("Asset {} is not a vector type", asset.id));
    }

    let uri = asset.storage_uri.strip_prefix("s3://").ok_or("Invalid storage URI")?;
    let parts: Vec<&str> = uri.splitn(2, '/').collect();
    if parts.len() < 2 {
        return Err("Invalid storage URI format".to_string());
    }
    let bucket = parts[0];
    let key = parts[1];

    let resp = ctx.s3_client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| format!("Failed to download from S3: {}", e))?;

    let data = resp.body.collect().await
        .map_err(|e| format!("Failed to read S3 body: {}", e))?
        .to_vec();

    serde_json::from_slice(&data).map_err(|e| format!("Failed to parse GeoJSON: {}", e))
}

pub async fn upload_geojson(
    ctx: &NodeContext, 
    name: &str, 
    geojson: &Value, 
    owner_id: Uuid,
    origin: &str,
    execution_id: Option<Uuid>,
) -> Result<Asset, String> {
    let bucket_name = std::env::var("MINIO_BUCKET_NAME").unwrap_or_else(|_| "earthflow".to_string());
    let asset_id = Uuid::new_v4();
    let object_key = format!("{}/{}.geojson", owner_id, asset_id);
    
    let file_bytes = serde_json::to_vec(geojson).map_err(|e| e.to_string())?;

    ctx.s3_client
        .put_object()
        .bucket(&bucket_name)
        .key(&object_key)
        .body(ByteStream::from(file_bytes))
        .content_type("application/geo+json")
        .send()
        .await
        .map_err(|e| format!("Failed to upload to S3: {}", e))?;
        
    let storage_uri = format!("s3://{}/{}", bucket_name, object_key);

    let asset = sqlx::query_as::<_, Asset>(
        r#"
        INSERT INTO assets (id, owner_id, name, asset_type, storage_uri, origin, execution_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#
    )
    .bind(asset_id)
    .bind(owner_id)
    .bind(name)
    .bind("VECTOR")
    .bind(storage_uri)
    .bind(origin)
    .bind(execution_id)
    .fetch_one(&ctx.pool)
    .await
    .map_err(|e| format!("DB Insert failed: {}", e))?;

    Ok(asset)
}

pub async fn upload_raster(
    ctx: &NodeContext, 
    name: &str, 
    bytes: Vec<u8>,
    owner_id: Uuid,
    origin: &str,
    execution_id: Option<Uuid>,
) -> Result<Asset, String> {
    let bucket_name = std::env::var("MINIO_BUCKET_NAME").unwrap_or_else(|_| "earthflow".to_string());
    let asset_id = Uuid::new_v4();
    let object_key = format!("{}/{}.tif", owner_id, asset_id);
    
    ctx.s3_client
        .put_object()
        .bucket(&bucket_name)
        .key(&object_key)
        .body(ByteStream::from(bytes))
        .content_type("image/tiff")
        .send()
        .await
        .map_err(|e| format!("Failed to upload to S3: {}", e))?;
        
    let storage_uri = format!("s3://{}/{}", bucket_name, object_key);

    let asset = sqlx::query_as::<_, Asset>(
        r#"
        INSERT INTO assets (id, owner_id, name, asset_type, storage_uri, origin, execution_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#
    )
    .bind(asset_id)
    .bind(owner_id)
    .bind(name)
    .bind("RASTER")
    .bind(storage_uri)
    .bind(origin)
    .bind(execution_id)
    .fetch_one(&ctx.pool)
    .await
    .map_err(|e| format!("DB Insert failed: {}", e))?;

    Ok(asset)
}

/// Parse a GeoJSON coordinate array [lng, lat] into a geo::Coord
fn coord_from_json(v: &Value) -> Option<Coord<f64>> {
    let arr = v.as_array()?;
    let x = arr.get(0)?.as_f64()?;
    let y = arr.get(1)?.as_f64()?;
    Some(Coord { x, y })
}

/// Convert GeoJSON geometry Value -> geo::Geometry
pub fn geojson_to_geo(geom: &Value) -> Option<Geometry<f64>> {
    let geom_type = geom["type"].as_str()?;
    match geom_type {
        "Point" => {
            let c = coord_from_json(&geom["coordinates"])?;
            Some(Geometry::Point(Point(c)))
        }
        "LineString" => {
            let coords: Vec<Coord<f64>> = geom["coordinates"].as_array()?
                .iter().filter_map(coord_from_json).collect();
            Some(Geometry::LineString(LineString(coords)))
        }
        "Polygon" => {
            let rings = geom["coordinates"].as_array()?;
            let exterior: Vec<Coord<f64>> = rings.get(0)?
                .as_array()?.iter().filter_map(coord_from_json).collect();
            let interiors: Vec<LineString<f64>> = rings[1..].iter().map(|ring| {
                LineString(ring.as_array().unwrap_or(&vec![])
                    .iter().filter_map(coord_from_json).collect())
            }).collect();
            Some(Geometry::Polygon(Polygon::new(LineString(exterior), interiors)))
        }
        "MultiPolygon" => {
            let polys: Vec<Polygon<f64>> = geom["coordinates"].as_array()?
                .iter().filter_map(|poly_coords| {
                    let rings = poly_coords.as_array()?;
                    let exterior: Vec<Coord<f64>> = rings.get(0)?
                        .as_array()?.iter().filter_map(coord_from_json).collect();
                    Some(Polygon::new(LineString(exterior), vec![]))
                }).collect();
            Some(Geometry::MultiPolygon(MultiPolygon(polys)))
        }
        "MultiPoint" => {
            let pts: Vec<Point<f64>> = geom["coordinates"].as_array()?
                .iter().filter_map(|v| {
                    let c = coord_from_json(v)?;
                    Some(Point(c))
                }).collect();
            Some(Geometry::MultiPoint(MultiPoint(pts)))
        }
        _ => None,
    }
}

/// Convert geo::Point -> GeoJSON geometry Value
pub fn point_to_geojson(p: &Point<f64>) -> Value {
    json!({
        "type": "Point",
        "coordinates": [p.x(), p.y()]
    })
}

/// Convert geo::Polygon -> GeoJSON geometry value
pub fn polygon_to_geojson(poly: &Polygon<f64>) -> Value {
    let exterior: Vec<Value> = poly.exterior().coords()
        .map(|c| json!([c.x, c.y]))
        .collect();
    json!({
        "type": "Polygon",
        "coordinates": [exterior]
    })
}

/// Collect all Point coords from a GeoJSON FeatureCollection for convex hull
pub fn collect_all_coords(geojson: &Value) -> Vec<Coord<f64>> {
    let mut coords = Vec::new();
    if let Some(features) = geojson["features"].as_array() {
        for feature in features {
            if let Some(geo) = geojson_to_geo(&feature["geometry"]) {
                match geo {
                    Geometry::Point(p) => coords.push(p.0),
                    Geometry::LineString(ls) => coords.extend(ls.0),
                    Geometry::Polygon(poly) => coords.extend(poly.exterior().0.clone()),
                    Geometry::MultiPolygon(mp) => {
                        for poly in mp.0 {
                            coords.extend(poly.exterior().0.clone());
                        }
                    }
                    Geometry::MultiPoint(mp) => {
                        for pt in mp.0 { coords.push(pt.0); }
                    }
                    _ => {}
                }
            }
        }
    }
    coords
}

/// Simplify a GeoJSON geometry (Polygon/LineString) with epsilon
pub fn simplify_geojson_geom(geom: &Value, epsilon: f64) -> Value {
    if let Some(geo) = geojson_to_geo(geom) {
        match geo {
            Geometry::Polygon(poly) => {
                let simplified = poly.simplify(&epsilon);
                return polygon_to_geojson(&simplified);
            }
            Geometry::LineString(ls) => {
                let simplified = ls.simplify(&epsilon);
                let coords: Vec<Value> = simplified.0.iter().map(|c| json!([c.x, c.y])).collect();
                return json!({ "type": "LineString", "coordinates": coords });
            }
            _ => {}
        }
    }
    geom.clone() // fallback: return unchanged
}

/// Evaluates a node parameter, resolving variables and Rhai expressions if necessary.
pub fn evaluate_parameter(param: &Value, inputs: &PortMap) -> Result<PortValue, String> {
    match param {
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                Ok(PortValue::Scalar(f))
            } else if let Some(i) = n.as_i64() {
                Ok(PortValue::Integer(i))
            } else {
                Err("Invalid number format".to_string())
            }
        }
        Value::Bool(b) => Ok(PortValue::Boolean(*b)),
        Value::String(s) => {
            // Try to evaluate as a Rhai expression.
            let engine = ExpressionEngine::new();
            match engine.eval(s, inputs) {
                Ok(val) => Ok(val),
                Err(_) => {
                    // Fallback: If evaluation fails (e.g. it's just a literal string like "City" or "my_column"),
                    // return the string itself as a literal.
                    Ok(PortValue::String(s.clone()))
                }
            }
        }
        Value::Null => Err("Parameter is missing or null".to_string()),
        _ => Ok(PortValue::Json(param.clone())),
    }
}
