mod common;
use common::TestHarness;
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_real_raster_workflow() {
    let harness = TestHarness::new().await;
    
    // 1. Download real raster
    let (name, content) = harness.download_test_raster().await;
    let raster_asset = harness.create_test_asset(&name, "RASTER", &content).await;

    // 2. Run a workflow that uses this raster
    // Since our nodes are still stubs, we are verifying the "relay" and asset ID handling with real files
    let workflow_json = json!({
        "nodes": [
            {
                "id": "r_in",
                "type": "variable",
                "data": { 
                    "inputType": "asset",
                    "value": raster_asset 
                } 
            },
            {
                "id": "clip",
                "type": "raster.clip_by_extent",
                "data": {
                    "bbox": [0, 0, 10, 10]
                }
            }
        ],
        "edges": [
            { "source": "r_in", "sourceHandle": "output", "target": "clip", "targetHandle": "raster" }
        ]
    });

    let result = harness.executor.execute(
        "real-raster-test",
        &workflow_json,
        HashMap::new(),
        None
    ).await;

    assert!(result.is_ok(), "Raster workflow failed: {:?}", result.err());
    
    let outputs = result.unwrap();
    let clip_output = outputs.get("clip").expect("Clip output missing");
    let asset = clip_output.get("output").expect("Asset output missing");
    
    println!("Real Raster Test Success. Result asset: {:?}", asset);
}
