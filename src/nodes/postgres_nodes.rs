use async_trait::async_trait;
use serde_json::{Value, json};
use crate::nodes::{NodeHandler, NodeMetadata, PortMetadata, PortMap, PortValue, NodeContext};
use crate::nodes::utils::upload_geojson;
use crate::models::connection::{Connection, ConnectionProvider};
use sqlx::{PgPool, Row};

pub struct SourcePostgresNode;

#[async_trait]
impl NodeHandler for SourcePostgresNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_id: "source.postgres".to_string(),
            label: "Postgres Source".to_string(),
            description: "Load vector data from an external Postgres/PostGIS database".to_string(),
            inputs: vec![],
            outputs: vec![PortMetadata {
                id: "output".to_string(),
                label: "Vector Data".to_string(),
                port_type: "vector".to_string(),
            }],
        }
    }

    async fn execute(&self, ctx: &NodeContext, _inputs: &PortMap, params: &Value) -> Result<PortMap, String> {
        let connection_name = params["connectionName"].as_str().ok_or("Missing param: connectionName")?;
        let query = params["query"].as_str().ok_or("Missing param: query")?;

        // 1. Fetch connection info from our DB
        let connection = sqlx::query_as!(
            Connection,
            "SELECT id, owner_id, name, provider as \"provider: _\", config, last_test_ok, last_tested_at, created_at, updated_at FROM connections WHERE name = $1",
            connection_name
        )
        .fetch_one(&ctx.pool)
        .await
        .map_err(|e| format!("Connection {} not found: {}", connection_name, e))?;

        if connection.provider != ConnectionProvider::Postgres {
            return Err("Connection is not a Postgres provider".to_string());
        }

        // 2. Connect to the external DB
        // The config should contain the connection string or its parts
        let external_url = connection.config["url"].as_str().ok_or("Missing 'url' in connection config")?;
        let external_pool = PgPool::connect(external_url).await
            .map_err(|e| format!("Failed to connect to external DB: {}", e))?;

        // 3. Execute query and convert to GeoJSON
        // Use ST_AsGeoJSON to get JSON directly from PostGIS
        let rows = sqlx::query(&format!("SELECT ST_AsGeoJSON(t.*) as geojson FROM ({}) AS t", query))
            .fetch_all(&external_pool)
            .await
            .map_err(|e| format!("Query failed: {}", e))?;

        let features: Vec<Value> = rows.iter()
            .map(|row| {
                let s: String = row.get("geojson");
                serde_json::from_str(&s).unwrap_or(json!({}))
            })
            .collect();

        let geojson = json!({
            "type": "FeatureCollection",
            "features": features
        });

        // 4. Upload result to S3 as a temporary asset
        // (In a real app, we might check if this should be persistent)
        let owner_id = connection.owner_id;
        let asset = upload_geojson(ctx, &format!("Import: {}", connection_name), &geojson, owner_id).await?;

        let mut outputs = PortMap::new();
        outputs.insert("output".to_string(), PortValue::Asset(asset));
        Ok(outputs)
    }
}
