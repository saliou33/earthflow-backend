use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::AppState;
use crate::models::connection::{Connection, CreateConnectionRequest, UpdateConnectionRequest};

async fn get_mock_user_id() -> Uuid {
    Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap()
}

pub async fn list_connections(
    State(state): State<AppState>,
) -> Result<Json<Vec<Connection>>, (StatusCode, String)> {
    let pool = &state.pool;
    let user_id = get_mock_user_id().await;
    
    let connections = sqlx::query_as!(
        Connection,
        "SELECT id, owner_id, name, provider as \"provider: _\", config, last_test_ok, last_tested_at, created_at, updated_at FROM connections WHERE owner_id = $1 ORDER BY updated_at DESC",
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(connections))
}

pub async fn create_connection(
    State(state): State<AppState>,
    Json(payload): Json<CreateConnectionRequest>,
) -> Result<(StatusCode, Json<Connection>), (StatusCode, String)> {
    let pool = &state.pool;
    let user_id = get_mock_user_id().await;
    
    let default_config = serde_json::json!({});
    
    let connection = sqlx::query_as!(
        Connection,
        r#"
        INSERT INTO connections (owner_id, name, provider, credentials, config)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, owner_id, name, provider as "provider: _", config, last_test_ok, last_tested_at, created_at, updated_at
        "#,
        user_id,
        payload.name,
        payload.provider as _,
        &payload.credentials,
        payload.config.unwrap_or(default_config)
    )
    .fetch_one(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(connection)))
}

pub async fn get_connection(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Connection>, (StatusCode, String)> {
    let pool = &state.pool;
    let user_id = get_mock_user_id().await;
    
    let connection = sqlx::query_as!(
        Connection,
        "SELECT id, owner_id, name, provider as \"provider: _\", config, last_test_ok, last_tested_at, created_at, updated_at FROM connections WHERE id = $1 AND owner_id = $2",
        id,
        user_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match connection {
        Some(w) => Ok(Json(w)),
        None => Err((StatusCode::NOT_FOUND, "Connection not found".to_string())),
    }
}

pub async fn update_connection(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateConnectionRequest>,
) -> Result<Json<Connection>, (StatusCode, String)> {
    let pool = &state.pool;
    let user_id = get_mock_user_id().await;
    
    // In a real app the credentials should be re-encrypted here
    // For now we just update the blob if presented
    let connection = sqlx::query_as!(
        Connection,
        r#"
        UPDATE connections
        SET 
            name = COALESCE($1, name),
            credentials = COALESCE($2, credentials),
            config = COALESCE($3, config),
            updated_at = now()
        WHERE id = $4 AND owner_id = $5
        RETURNING id, owner_id, name, provider as "provider: _", config, last_test_ok, last_tested_at, created_at, updated_at
        "#,
        payload.name,
        payload.credentials,
        payload.config,
        id,
        user_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(connection))
}

pub async fn test_connection(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let pool = &state.pool;
    let user_id = get_mock_user_id().await;

    // 1. Fetch connection
    let connection = sqlx::query_as!(
        Connection,
        "SELECT id, owner_id, name, provider as \"provider: _\", config, last_test_ok, last_tested_at, created_at, updated_at FROM connections WHERE id = $1 AND owner_id = $2",
        id,
        user_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let _conn = connection.ok_or((StatusCode::NOT_FOUND, "Connection not found".to_string()))?;

    // 2. Mock test logic (always succeeds for now)
    // In a real app, we would use the 'credentials' blob to try to connect to the DB/S3
    let success = true;

    // 3. Update status
    sqlx::query!(
        "UPDATE connections SET last_test_ok = $1, last_tested_at = now() WHERE id = $2",
        success,
        id
    )
    .execute(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if success {
        Ok(StatusCode::OK)
    } else {
        Err((StatusCode::BAD_GATEWAY, "Connection test failed".to_string()))
    }
}

pub async fn delete_connection(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let pool = &state.pool;
    let user_id = get_mock_user_id().await;

    let result = sqlx::query!(
        "DELETE FROM connections WHERE id = $1 AND owner_id = $2",
        id,
        user_id
    )
    .execute(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        Err((StatusCode::NOT_FOUND, "Connection not found".to_string()))
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}
