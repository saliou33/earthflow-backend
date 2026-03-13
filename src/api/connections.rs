use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::connection::{Connection, CreateConnectionRequest, UpdateConnectionRequest};

async fn get_mock_user_id() -> Uuid {
    Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap()
}

pub async fn list_connections(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Connection>>, (StatusCode, String)> {
    let user_id = get_mock_user_id().await;
    
    let connections = sqlx::query_as!(
        Connection,
        "SELECT id, owner_id, name, provider as \"provider: _\", config, last_test_ok, last_tested_at, created_at, updated_at FROM connections WHERE owner_id = $1 ORDER BY updated_at DESC",
        user_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(connections))
}

pub async fn create_connection(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateConnectionRequest>,
) -> Result<(StatusCode, Json<Connection>), (StatusCode, String)> {
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
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(connection)))
}

pub async fn get_connection(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<Connection>, (StatusCode, String)> {
    let user_id = get_mock_user_id().await;
    
    let connection = sqlx::query_as!(
        Connection,
        "SELECT id, owner_id, name, provider as \"provider: _\", config, last_test_ok, last_tested_at, created_at, updated_at FROM connections WHERE id = $1 AND owner_id = $2",
        id,
        user_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match connection {
        Some(w) => Ok(Json(w)),
        None => Err((StatusCode::NOT_FOUND, "Connection not found".to_string())),
    }
}

pub async fn update_connection(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateConnectionRequest>,
) -> Result<Json<Connection>, (StatusCode, String)> {
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
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(connection))
}
