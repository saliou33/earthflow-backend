use axum::{
    routing::{get, post},
    Router,
};
use crate::api::{connections, workflows};
use crate::AppState;

pub fn v1_routes() -> Router<AppState> {
    Router::new()
        .route("/workflows", get(workflows::list_workflows))
        .route("/workflows", post(workflows::create_workflow))
        .route("/workflows/{id}", get(workflows::get_workflow))
        .route("/workflows/{id}", axum::routing::put(workflows::update_workflow))
        .route("/workflows/{id}/execute", post(workflows::execute_workflow))
        .route("/workflows/{id}/executions/latest", get(workflows::get_latest_workflow_execution))
        .route("/connections", get(connections::list_connections))
        .route("/connections", post(connections::create_connection))
        .route("/connections/{id}", get(connections::get_connection))
        .route("/connections/{id}", axum::routing::put(connections::update_connection))
}
