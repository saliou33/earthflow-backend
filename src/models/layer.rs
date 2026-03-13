use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Layer {
    pub id: Uuid,
    pub execution_id: Uuid,
    pub node_id: String,
    pub name: String,
    pub layer_type: String, // "vector" or "raster"
    pub storage_path: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}
