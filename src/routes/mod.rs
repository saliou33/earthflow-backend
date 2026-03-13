use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;

use crate::api::{connections, workflows};

pub fn v1_routes() -> Router<PgPool> {
    Router::new()
        .route("/workflows", get(workflows::list_workflows))
        .route("/workflows", post(workflows::create_workflow))
        .route("/workflows/{id}", get(workflows::get_workflow))
        .route("/workflows/{id}", axum::routing::put(workflows::update_workflow))
        .route("/connections", get(connections::list_connections))
        .route("/connections", post(connections::create_connection))
        .route("/connections/{id}", get(connections::get_connection))
        .route("/connections/{id}", axum::routing::put(connections::update_connection))
}
