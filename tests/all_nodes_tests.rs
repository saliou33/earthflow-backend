mod common;
use common::TestHarness;
use serde_json::json;
use std::collections::HashMap;
use backend::nodes::PortValue;

#[tokio::test]
async fn test_all_categories_workflow() {
    let harness = TestHarness::new().await;
    
    // 1. Seed initial assets
    let vector_content = json!({
        "type": "FeatureCollection",
        "features": [{"type": "Feature", "geometry": {"type": "Point", "coordinates": [0, 0]}, "properties": {}}]
    }).to_string();
    let vector_asset = harness.create_test_asset("test.geojson", "VECTOR", vector_content.as_bytes()).await;
    
    let raster_content = b"fake-raster-tiff";
    let raster_asset = harness.create_test_asset("test.tif", "RASTER", raster_content).await;

    // 2. Define a giant workflow that touches all categories
    // io -> vector.buffer -> analysis.kernel_density -> raster.hillshade -> style.choropleth
    // (using our relay stubs)
    let workflow_json = json!({
        "nodes": [
            {
                "id": "v_in",
                "type": "source.asset",
                "data": { "assetId": vector_asset.id.to_string() }
            },
            {
                "id": "r_in",
                "type": "variable",
                "data": { 
                    "inputType": "asset",
                    "value": raster_asset 
                } 
            },
            {
                "id": "buffer",
                "type": "vector.buffer",
                "data": { "distance": 10.0 }
            },
            {
                "id": "density",
                "type": "analysis.kernel_density",
                "data": {}
            },
            {
                "id": "hillshade",
                "type": "raster.hillshade",
                "data": {}
            },
            {
                "id": "style",
                "type": "style.simple_fill",
                "data": {}
            }
        ],
        "edges": [
            { "source": "v_in", "sourceHandle": "output", "target": "buffer", "targetHandle": "input" },
            { "source": "buffer", "sourceHandle": "output", "target": "density", "targetHandle": "input" },
            { "source": "density", "sourceHandle": "output", "target": "hillshade", "targetHandle": "raster" },
            // Style nodes often don't take data input in our current stubs but let's see
        ]
    });

    // 3. Execute
    let result = harness.executor.execute(
        "all-nodes-test",
        &workflow_json,
        HashMap::new(),
        None
    ).await;

    assert!(result.is_ok(), "Category chain failed: {:?}", result.err());
    
    let outputs = result.unwrap();
    assert!(outputs.contains_key("hillshade"), "Workflow didn't reach hillshade");
    
    println!("Comprehensive test phase 1 (Chain) passed.");
}

#[tokio::test]
async fn test_table_operations() {
    let harness = TestHarness::new().await;
    
    // Seed a dummy table asset
    let table_asset = harness.create_test_asset("test.csv", "TABLE", b"col1,col2\n1,2").await;
    
    let workflow_json = json!({
        "nodes": [
            {
                "id": "t_in",
                "type": "variable",
                "data": { 
                    "inputType": "asset",
                    "value": table_asset 
                }
            },
            {
                "id": "filter",
                "type": "table.filter",
                "data": { "expression": "col1 > 0" }
            }
        ],
        "edges": [
            { "source": "t_in", "sourceHandle": "output", "target": "filter", "targetHandle": "input" }
        ]
    });

    let result = harness.executor.execute(
        "table-test",
        &workflow_json,
        HashMap::new(),
        None
    ).await;

    assert!(result.is_ok(), "Table operation failed: {:?}", result.err());
    println!("Table test passed.");
}

#[tokio::test]
async fn test_complex_vector_workflow() {
    let harness = TestHarness::new().await;
    
    // Seed initial vector asset
    let vector_content = json!({
        "type": "FeatureCollection",
        "features": [{"type": "Feature", "geometry": {"type": "Point", "coordinates": [0, 0]}, "properties": {"pop": 100}}]
    }).to_string();
    let vector_asset = harness.create_test_asset("cities.geojson", "VECTOR", vector_content.as_bytes()).await;

    // A long chain of vector operations
    let workflow_json = json!({
        "nodes": [
            { "id": "start", "type": "source.asset", "data": { "assetId": vector_asset.id.to_string() } },
            { "id": "buf1", "type": "vector.buffer", "data": { "distance": 100.0 } },
            { "id": "cent", "type": "vector.centroid", "data": {} },
            { "id": "buf2", "type": "vector.buffer", "data": { "distance": 50.0 } },
            { "id": "hull", "type": "vector.convex_hull", "data": {} },
            { "id": "simp", "type": "vector.simplify", "data": { "tolerance": 0.1 } },
            { "id": "expr", "type": "expression", "data": { "expression": "input" } }
        ],
        "edges": [
            { "source": "start", "sourceHandle": "output", "target": "buf1", "targetHandle": "input" },
            { "source": "buf1", "sourceHandle": "output", "target": "cent", "targetHandle": "input" },
            { "source": "cent", "sourceHandle": "output", "target": "buf2", "targetHandle": "input" },
            { "source": "buf2", "sourceHandle": "output", "target": "hull", "targetHandle": "input" },
            { "source": "hull", "sourceHandle": "output", "target": "simp", "targetHandle": "input" },
            { "source": "simp", "sourceHandle": "output", "target": "expr", "targetHandle": "input" }
        ]
    });

    let result = harness.executor.execute("complex-vector", &workflow_json, HashMap::new(), None).await;

    assert!(result.is_ok(), "Complex vector workflow failed: {:?}", result.err());
    let outputs = result.unwrap();
    assert!(outputs.contains_key("expr"), "Workflow didn't reach the end");
    println!("Complex vector workflow test passed.");
}
#[tokio::test]
async fn test_draw_workflow() {
    let harness = TestHarness::new().await;
    
    let draw_node_json = json!({
        "nodes": [
            {
                "id": "draw",
                "type": "io.draw",
                "data": {
                    "label": "My Sketch",
                    "geometry": {
                        "type": "Feature",
                        "geometry": { "type": "Point", "coordinates": [10.0, 20.0] },
                        "properties": {}
                    }
                }
            }
        ],
        "edges": []
    });

    let result = harness.executor.execute("draw-test", &draw_node_json, HashMap::new(), None).await;

    assert!(result.is_ok(), "Draw workflow failed: {:?}", result.err());
    let outputs = result.unwrap();
    assert!(outputs.contains_key("draw"), "Workflow didn't reach the draw node");
    
    let node_res = outputs.get("draw").expect("Node 'draw' didn't execute");
    let port_val = node_res.get("output").expect("Port 'output' not found");
    
    match port_val {
        PortValue::Asset(a) => {
            assert_eq!(a.name, "My Sketch");
            assert_eq!(a.asset_type, "VECTOR");
        },
        _ => panic!("Output should be an Asset"),
    }
    println!("Draw workflow test passed.");
}
