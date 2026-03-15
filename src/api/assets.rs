use axum::{
    extract::{Path, State, multipart::Multipart, Query},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use crate::models::asset::{Asset, CreateAssetRequest};
use crate::AppState;
use uuid::Uuid;
use aws_sdk_s3::primitives::ByteStream;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_assets).post(create_asset))
        .route("/upload", post(upload_asset))
        .route("/{id}", get(get_asset).put(update_asset).delete(delete_asset))
        .route("/{id}/url", get(get_asset_url))
}

#[derive(Deserialize)]
pub struct AssetQueryParams {
    pub q: Option<String>,
    pub asset_type: Option<String>,
    pub origin: Option<String>,  // "user" | "execution" | "all"
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

async fn get_asset(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Asset>, (StatusCode, String)> {
    let asset = sqlx::query_as::<_, Asset>("SELECT * FROM assets WHERE id = $1")
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, format!("Asset not found: {}", e)))?;

    Ok(Json(asset))
}

async fn list_assets(
    State(state): State<AppState>,
    Query(params): Query<AssetQueryParams>,
) -> Result<Json<Vec<Asset>>, (StatusCode, String)> {
    let limit = params.limit.unwrap_or(50);
    let offset = (params.page.unwrap_or(1) - 1) * limit;
    
    let mut query = String::from("SELECT * FROM assets WHERE 1=1");
    let mut bindings = Vec::new();
    let mut arg_idx = 1;

    if let Some(q) = params.q {
        query.push_str(&format!(" AND (name ILIKE ${} OR description ILIKE ${})", arg_idx, arg_idx + 1));
        bindings.push(format!("%{}%", q));
        bindings.push(format!("%{}%", q));
        arg_idx += 2;
    }

    if let Some(t) = params.asset_type {
        query.push_str(&format!(" AND asset_type = ${}", arg_idx));
        bindings.push(t);
        arg_idx += 1;
    }

    // Filter by origin; default to "user" so intermediates don't pollute the asset picker
    let origin_filter = params.origin.as_deref().unwrap_or("user");
    if origin_filter != "all" {
        query.push_str(&format!(" AND origin = ${}", arg_idx));
        bindings.push(origin_filter.to_string());
        arg_idx += 1;
    }

    query.push_str(&format!(" ORDER BY created_at DESC LIMIT ${} OFFSET ${}", arg_idx, arg_idx + 1));
    
    let mut q_exec = sqlx::query_as::<_, Asset>(&query);
    for binding in bindings {
        q_exec = q_exec.bind(binding);
    }
    q_exec = q_exec.bind(limit).bind(offset);

    let assets = q_exec
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(assets))
}

async fn create_asset(
    State(state): State<AppState>,
    Json(payload): Json<CreateAssetRequest>,
) -> Result<Json<Asset>, (StatusCode, String)> {
    // For external references or empty placeholders before upload
    let owner_id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap(); // mock owner
    
    let asset = sqlx::query_as::<_, Asset>(
        r#"
        INSERT INTO assets (owner_id, name, description, asset_type, storage_uri)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#
    )
    .bind(owner_id)
    .bind(payload.name)
    .bind(payload.description)
    .bind(payload.asset_type)
    .bind("") // storage_uri initially empty if creating placeholder
    .fetch_one(&state.pool)
    .await
    .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(asset))
}

async fn delete_asset(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    sqlx::query!("DELETE FROM assets WHERE id = $1", id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

async fn update_asset(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<crate::models::asset::UpdateAssetRequest>,
) -> Result<Json<Asset>, (StatusCode, String)> {
    let mut query = String::from("UPDATE assets SET updated_at = NOW()");
    let mut params_count = 1;

    if payload.name.is_some() {
        params_count += 1;
        query.push_str(&format!(", name = ${}", params_count));
    }
    if payload.description.is_some() {
        params_count += 1;
        query.push_str(&format!(", description = ${}", params_count));
    }

    query.push_str(&format!(" WHERE id = $1 RETURNING *"));

    let mut q = sqlx::query_as::<_, Asset>(&query).bind(id);

    if let Some(name) = payload.name {
        q = q.bind(name);
    }
    if let Some(description) = payload.description {
        q = q.bind(description);
    }

    let asset = q.fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(asset))
}

// Upload handles multipart form, saves to MinIO and creates DB entry
async fn upload_asset(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<Asset>, (StatusCode, String)> {
    let owner_id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
    let bucket_name = std::env::var("MINIO_BUCKET_NAME").unwrap_or_else(|_| "earthflow".to_string());
    
    let mut name = String::new();
    let mut description = String::new();
    let mut asset_type = String::new();
    let mut file_bytes = Vec::new();
    let mut original_filename = String::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))? {
        let field_name = field.name().unwrap_or("").to_string();
        
        if field_name == "file" {
            original_filename = field.file_name().unwrap_or("upload.bin").to_string();
            let data = field.bytes().await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            file_bytes = data.to_vec();
        } else if field_name == "name" {
            name = field.text().await.unwrap_or_default();
        } else if field_name == "description" {
            description = field.text().await.unwrap_or_default();
        } else if field_name == "asset_type" {
            asset_type = field.text().await.unwrap_or_default();
        }
    }

    if file_bytes.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "File is missing".to_string()));
    }
    
    if name.is_empty() {
        name = original_filename.clone();
    }
    if asset_type.is_empty() {
        // default guess based on extension
        if original_filename.ends_with(".geojson") || original_filename.ends_with(".json") {
            asset_type = "VECTOR".to_string();
        } else if original_filename.ends_with(".tif") || original_filename.ends_with(".geotiff") || original_filename.ends_with(".tiff") {
            asset_type = "RASTER".to_string();
        } else {
            asset_type = "UNKNOWN".to_string();
        }
    }

    // Generate unique object key
    let asset_id = Uuid::new_v4();
    let extension = original_filename.split('.').last().unwrap_or("bin");
    let object_key = format!("{}/{}.{}", owner_id, asset_id, extension);
    
    // Upload to MinIO
    state.s3_client
        .put_object()
        .bucket(&bucket_name)
        .key(&object_key)
        .body(ByteStream::from(file_bytes))
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to upload to S3: {}", e)))?;
        
    let storage_uri = format!("s3://{}/{}", bucket_name, object_key);

    // Create DB Record
    let asset = sqlx::query_as::<_, Asset>(
        r#"
        INSERT INTO assets (id, owner_id, name, description, asset_type, storage_uri)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#
    )
    .bind(asset_id)
    .bind(owner_id)
    .bind(name)
    .bind(Some(description))
    .bind(asset_type)
    .bind(storage_uri)
    .fetch_one(&state.pool)
    .await
    .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(asset))
}

async fn get_asset_url(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let asset = sqlx::query_as::<_, Asset>("SELECT * FROM assets WHERE id = $1")
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, format!("Asset not found: {}", e)))?;

    // Parse s3://bucket/key
    let uri = asset.storage_uri.strip_prefix("s3://").ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Invalid storage URI".to_string()))?;
    let parts: Vec<&str> = uri.splitn(2, '/').collect();
    if parts.len() < 2 {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "Invalid storage URI format".to_string()));
    }
    let bucket = parts[0];
    let key = parts[1];

    let expires_in = std::time::Duration::from_secs(3600);
    let presigned_request = state.s3_client
        .get_object()
        .bucket(bucket)
        .key(key)
        .presigned(aws_sdk_s3::presigning::PresigningConfig::expires_in(expires_in).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut response = serde_json::json!({
        "url": presigned_request.uri().to_string(),
        "method": presigned_request.method().to_string(),
        "expires_in": 3600
    });

    // If it's a RASTER, also provide the tile URL template
    if asset.asset_type == "RASTER" {
        let titiler_base = std::env::var("TITILER_ENDPOINT").unwrap_or_else(|_| "http://localhost:8001".to_string());
        let tile_url = format!("{}/tiles/WebMercatorQuad/{{z}}/{{x}}/{{y}}@1x?url={}", titiler_base, asset.storage_uri);
        if let Some(obj) = response.as_object_mut() {
            obj.insert("url_template".to_string(), serde_json::Value::String(tile_url));
        }
    }

    Ok(Json(response))
}


