use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "connection_provider", rename_all = "snake_case")]
pub enum ConnectionProvider {
    Postgres,
    BigQuery,
    Snowflake,
    Databricks,
    S3,
    Gcs,
    AzureBlob,
    SentinelHub,
    Planet,
    Wms,
    Wfs,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Connection {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub provider: ConnectionProvider,
    pub config: serde_json::Value,
    pub last_test_ok: Option<bool>,
    pub last_tested_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConnectionRequest {
    pub name: String,
    pub provider: ConnectionProvider,
    pub credentials: Vec<u8>,
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConnectionRequest {
    pub name: Option<String>,
    pub credentials: Option<Vec<u8>>,
    pub config: Option<serde_json::Value>,
}
