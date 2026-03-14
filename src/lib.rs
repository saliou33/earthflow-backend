pub mod api;
pub mod db;
pub mod engine;
pub mod models;
pub mod nodes;
pub mod routes;

use std::sync::Arc;
use crate::nodes::NodeRegistry;
use aws_sdk_s3::Client as S3Client;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub registry: Arc<NodeRegistry>,
    pub s3_client: S3Client,
}
