use backend::nodes::{NodeRegistry, NodeContext};
use backend::engine::executor::WorkflowExecutor;
use backend::nodes::{io::VectorInputNode, vector_nodes::BufferNode};
use std::collections::HashMap;
use serde_json::json;

#[tokio::test]
async fn test_vector_buffer_workflow() {
    dotenvy::dotenv().ok();
    
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = sqlx::PgPool::connect(&db_url).await.expect("Failed to connect to DB");
    
    let mut registry = NodeRegistry::new();
    registry.register(Box::new(VectorInputNode));
    registry.register(Box::new(BufferNode));
    // Note: NodeRegistry doesn't need to be Arc for executor if we pass by ref
    
    let minio_endpoint = std::env::var("MINIO_ENDPOINT").unwrap_or_else(|_| "http://localhost:9000".to_string());
    let minio_access_key = std::env::var("MINIO_ROOT_USER").unwrap_or_else(|_| "admin".to_string());
    let minio_secret_key = std::env::var("MINIO_ROOT_PASSWORD").unwrap_or_else(|_| "password".to_string());
    
    let s3_config = aws_sdk_s3::config::Builder::new()
        .credentials_provider(aws_sdk_s3::config::Credentials::new(
            minio_access_key,
            minio_secret_key,
            None,
            None,
            "Static",
        ))
        .region(aws_sdk_s3::config::Region::new("us-east-1"))
        .endpoint_url(minio_endpoint)
        .force_path_style(true)
        .build();
    let s3_client = aws_sdk_s3::Client::from_conf(s3_config);
    
    let ctx = NodeContext {
        pool,
        s3_client,
    };
    
    let executor = WorkflowExecutor::new(&registry, ctx);
    
    // Existing asset ID from DB (senegal.geojson)
    let asset_id = "e0b955f9-ec9f-4ef0-8ff4-d757938d33ad";
    
    let workflow_json = json!({
        "nodes": [
            {
                "id": "node_1",
                "type": "vector_input",
                "data": {
                    "assetId": asset_id
                }
            },
            {
                "id": "node_2",
                "type": "vector.buffer",
                "data": {
                    "distance": 500.0
                }
            }
        ],
        "edges": [
            {
                "source": "node_1",
                "sourceHandle": "output",
                "target": "node_2",
                "targetHandle": "input"
            }
        ]
    });
    
    let result = executor.execute(
        "test-workflow",
        &workflow_json,
        HashMap::new(),
        None
    ).await;
    
    assert!(result.is_ok(), "Workflow execution failed: {:?}", result.err());
    
    let outputs = result.unwrap();
    assert!(outputs.contains_key("node_2"), "Node 2 output missing");
    
    let node_2_output = &outputs["node_2"];
    let asset_val = node_2_output.get("output").expect("Output port missing");
    
    println!("Workflow Result: {:?}", asset_val);
}
