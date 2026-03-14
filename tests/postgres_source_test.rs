mod common;
use common::TestHarness;
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_postgres_source_node() {
    let harness = TestHarness::new().await;
    let pool = &harness.ctx.pool;
    let user_id = harness.get_mock_user_id().await;

    // 1. Create a connection record in our DB pointing to the external DB we just created
    // Note: Since everything is in Docker, 'localhost' might not work if tests run in another container,
    // but here they run on the host (linux). Let's use 127.0.0.1 or the docker host ip if needed.
    // Based on docker-compose, port 5432 is mapped to host.
    let external_db_url = "postgres://postgres:postgres@127.0.0.1:5432/external_gis";
    
    let connection_name = "ExternalTestDB";
    sqlx::query!(
        r#"
        INSERT INTO connections (owner_id, name, provider, credentials, config)
        VALUES ($1, $2, 'postgres', $3, $4)
        ON CONFLICT (owner_id, name) DO UPDATE SET config = $4
        "#,
        user_id,
        connection_name,
        &vec![0u8], // Dummy credentials
        json!({"url": external_db_url})
    )
    .execute(pool)
    .await
    .expect("Failed to create test connection");

    // 2. Define workflow with SourcePostgresNode
    let workflow_json = json!({
        "nodes": [
            {
                "id": "pg_source",
                "type": "source.postgres",
                "data": {
                    "connectionName": connection_name,
                    "query": "SELECT * FROM test_features"
                }
            },
            {
                "id": "buffer",
                "type": "vector.buffer",
                "data": {
                    "distance": 100.0
                }
            }
        ],
        "edges": [
            {
                "source": "pg_source",
                "sourceHandle": "output",
                "target": "buffer",
                "targetHandle": "input"
            }
        ]
    });

    // 3. Execute
    let result = harness.executor.execute(
        "test-pg-workflow",
        &workflow_json,
        HashMap::new(),
        None
    ).await;

    assert!(result.is_ok(), "Postgres workflow failed: {:?}", result.err());
    
    let outputs = result.unwrap();
    let buffer_output = outputs.get("buffer").expect("Buffer output missing");
    let asset = buffer_output.get("output").expect("Asset output missing");
    
    println!("Postgres Source Test Success. Result asset: {:?}", asset);
}
