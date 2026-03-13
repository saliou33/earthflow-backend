use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::workflow::{CreateWorkflowRequest, UpdateWorkflowRequest, Workflow};

// In a real app, user_id would come from auth middleware
// For POC, we'll use a mocked user ID or expect it in headers (simplified here)
async fn get_mock_user_id() -> Uuid {
    // POC: hardcoded user ID for testing since we skip real auth in backend POC
    Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap()
}

pub async fn list_workflows(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Workflow>>, (StatusCode, String)> {
    let user_id = get_mock_user_id().await;
    
    let workflows = sqlx::query_as!(
        Workflow,
        "SELECT * FROM workflows WHERE owner_id = $1 ORDER BY updated_at DESC",
        user_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(workflows))
}

pub async fn create_workflow(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateWorkflowRequest>,
) -> Result<(StatusCode, Json<Workflow>), (StatusCode, String)> {
    let user_id = get_mock_user_id().await;
    
    let default_graph = serde_json::json!({"nodes": [], "edges": []});
    
    let workflow = sqlx::query_as!(
        Workflow,
        r#"
        INSERT INTO workflows (owner_id, name, description, graph)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
        user_id,
        payload.name,
        payload.description,
        default_graph
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create workflow: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    Ok((StatusCode::CREATED, Json(workflow)))
}

pub async fn get_workflow(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<Workflow>, (StatusCode, String)> {
    let user_id = get_mock_user_id().await;
    
    let workflow = sqlx::query_as!(
        Workflow,
        "SELECT * FROM workflows WHERE id = $1 AND owner_id = $2",
        id,
        user_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match workflow {
        Some(w) => Ok(Json(w)),
        None => Err((StatusCode::NOT_FOUND, "Workflow not found".to_string())),
    }
}

pub async fn update_workflow(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateWorkflowRequest>,
) -> Result<Json<Workflow>, (StatusCode, String)> {
    let user_id = get_mock_user_id().await;
    
    // Check if it exists and belongs to user first
    let _existing = sqlx::query!("SELECT id FROM workflows WHERE id = $1 AND owner_id = $2", id, user_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Workflow not found".to_string()))?;

    let workflow = sqlx::query_as!(
        Workflow,
        r#"
        UPDATE workflows
        SET 
            name = COALESCE($1, name),
            description = COALESCE($2, description),
            graph = COALESCE($3, graph),
            tags = COALESCE($4, tags),
            is_public = COALESCE($5, is_public),
            updated_at = now()
        WHERE id = $6 AND owner_id = $7
        RETURNING *
        "#,
        payload.name,
        payload.description,
        payload.graph,
        payload.tags.as_deref() as Option<&[String]>,
        payload.is_public,
        id,
        user_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(workflow))
}
