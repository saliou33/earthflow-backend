use aws_sdk_s3::primitives::ByteStream;
use crate::nodes::NodeContext;
use crate::models::asset::Asset;
use uuid::Uuid;
use serde_json::Value;

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
    owner_id: Uuid
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
        INSERT INTO assets (id, owner_id, name, asset_type, storage_uri)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#
    )
    .bind(asset_id)
    .bind(owner_id)
    .bind(name)
    .bind("VECTOR")
    .bind(storage_uri)
    .fetch_one(&ctx.pool)
    .await
    .map_err(|e| format!("DB Insert failed: {}", e))?;

    Ok(asset)
}
