use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::AppState;
use crate::models::workflow::{CreateWorkflowRequest, UpdateWorkflowRequest, Workflow};
use crate::models::execution::WorkflowExecution;
use crate::engine::executor::WorkflowExecutor;
use crate::nodes::{PortMap, NodeContext};
use serde::Deserialize;
use std::collections::HashMap;

// In a real app, user_id would come from auth middleware
// For POC, we'll use a mocked user ID or expect it in headers (simplified here)
async fn get_mock_user_id() -> Uuid {
    // POC: hardcoded user ID for testing since we skip real auth in backend POC
    Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap()
}

pub async fn list_workflows(
    State(state): State<AppState>,
) -> Result<Json<Vec<Workflow>>, (StatusCode, String)> {
    let pool = &state.pool;
    let user_id = get_mock_user_id().await;
    
    let workflows = sqlx::query_as!(
        Workflow,
        "SELECT * FROM workflows WHERE owner_id = $1 ORDER BY updated_at DESC",
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(workflows))
}

pub async fn create_workflow(
    State(state): State<AppState>,
    Json(payload): Json<CreateWorkflowRequest>,
) -> Result<(StatusCode, Json<Workflow>), (StatusCode, String)> {
    let pool = &state.pool;
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
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create workflow: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    Ok((StatusCode::CREATED, Json(workflow)))
}

pub async fn get_workflow(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Workflow>, (StatusCode, String)> {
    let pool = &state.pool;
    let user_id = get_mock_user_id().await;
    
    let workflow = sqlx::query_as!(
        Workflow,
        "SELECT * FROM workflows WHERE id = $1 AND owner_id = $2",
        id,
        user_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match workflow {
        Some(w) => Ok(Json(w)),
        None => Err((StatusCode::NOT_FOUND, "Workflow not found".to_string())),
    }
}

pub async fn update_workflow(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateWorkflowRequest>,
) -> Result<Json<Workflow>, (StatusCode, String)> {
    let pool = &state.pool;
    let user_id = get_mock_user_id().await;
    
    // Check if it exists and belongs to user first
    let _existing = sqlx::query!("SELECT id FROM workflows WHERE id = $1 AND owner_id = $2", id, user_id)
        .fetch_optional(pool)
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
    .fetch_one(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(workflow))
}

#[derive(Deserialize)]
pub struct ExecuteWorkflowRequest {
    pub node_id: Option<String>,
}

pub async fn execute_workflow(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<Option<ExecuteWorkflowRequest>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let pool = &state.pool;
    let user_id = get_mock_user_id().await;

    // Fetch workflow
    let workflow = sqlx::query_as!(
        Workflow,
        "SELECT * FROM workflows WHERE id = $1 AND owner_id = $2",
        id,
        user_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Workflow not found".to_string()))?;

    // Fetch latest successful execution results for caching
    let last_execution = sqlx::query!(
        "SELECT results FROM workflow_executions WHERE workflow_id = $1 AND status = 'completed' ORDER BY created_at DESC LIMIT 1",
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let cached_outputs: HashMap<String, PortMap> = if let Some(exec) = last_execution {
        serde_json::from_value(exec.results).unwrap_or_default()
    } else {
        HashMap::new()
    };

    let target_node_id = payload.and_then(|p| p.node_id);

    // Start execution timer
    let start_time = std::time::Instant::now();

    // Execute
    let ctx = NodeContext {
        pool: state.pool.clone(),
        s3_client: state.s3_client.clone(),
    };
    let executor = WorkflowExecutor::new(&state.registry, ctx);
    let results = executor.execute(&id.to_string(), &workflow.graph, cached_outputs, target_node_id)
        .await;

    let execution_time_ms = start_time.elapsed().as_millis() as i64;
    
    let (status, results_val) = match results {
        Ok(r) => ("completed", serde_json::to_value(r).unwrap()),
        Err(e) => ("failed", serde_json::json!({ "error": e })),
    };

    // Persist execution
    sqlx::query!(
        r#"
        INSERT INTO workflow_executions (workflow_id, owner_id, status, results, execution_time_ms)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        id,
        user_id,
        status,
        results_val,
        execution_time_ms
    )
    .execute(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if status == "failed" {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, results_val["error"].as_str().unwrap_or("Unknown error").to_string()));
    }

    Ok(Json(results_val))
}

pub async fn get_latest_workflow_execution(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Option<WorkflowExecution>>, (StatusCode, String)> {
    let pool = &state.pool;
    let user_id = get_mock_user_id().await;

    let execution = sqlx::query_as!(
        WorkflowExecution,
        r#"
        SELECT * FROM workflow_executions 
        WHERE workflow_id = $1 AND owner_id = $2 
        ORDER BY created_at DESC 
        LIMIT 1
        "#,
        id,
        user_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(execution))
}

pub async fn list_executions(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<WorkflowExecution>>, (StatusCode, String)> {
    let pool = &state.pool;
    let user_id = get_mock_user_id().await;

    let executions = sqlx::query_as!(
        WorkflowExecution,
        "SELECT * FROM workflow_executions WHERE workflow_id = $1 AND owner_id = $2 ORDER BY created_at DESC LIMIT 50",
        id,
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(executions))
}

pub async fn clear_executions(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let pool = &state.pool;
    let user_id = get_mock_user_id().await;

    sqlx::query!(
        "DELETE FROM workflow_executions WHERE workflow_id = $1 AND owner_id = $2",
        id,
        user_id
    )
    .execute(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
