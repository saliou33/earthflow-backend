use std::collections::HashMap;
use serde_json::Value;
use petgraph::visit::EdgeRef;
use crate::engine::dag::WorkflowGraph;
use crate::nodes::{NodeRegistry, PortMap, NodeContext};

pub struct WorkflowExecutor<'a> {
    registry: &'a NodeRegistry,
    ctx: NodeContext,
}

impl<'a> WorkflowExecutor<'a> {
    pub fn new(registry: &'a NodeRegistry, ctx: NodeContext) -> Self {
        Self { registry, ctx }
    }

    pub async fn execute(
        &self,
        _workflow_id: &str,
        graph_json: &Value,
        cached_outputs: HashMap<String, PortMap>,
        target_node_id: Option<String>,
    ) -> Result<HashMap<String, PortMap>, String> {
        let dag = WorkflowGraph::from_json(graph_json)?;
        let sorted_nodes = dag.topological_sort()?;

        let mut node_outputs = cached_outputs;
        
        // Map of node_id -> node_json_data for parameters
        let nodes_data: HashMap<String, &Value> = graph_json["nodes"]
            .as_array()
            .ok_or("Nodes is not an array")?
            .iter()
            .map(|n| (n["id"].as_str().unwrap().to_string(), n))
            .collect();

        for node_id in sorted_nodes {
            // If target_node_id is set, we only care about executing it and its dependencies.
            // But since we are doing a topological sort, we can just skip nodes that are:
            // 1. Already in cache
            // 2. NOT the target_node_id (if target_node_id is set)
            
            let is_target = target_node_id.as_ref().map_or(false, |id| id == &node_id);
            
            if node_outputs.contains_key(&node_id) && !is_target {
                tracing::debug!("Using cached output for node: {}", node_id);
                continue;
            }

            let node_json = nodes_data.get(&node_id).ok_or(format!("Node data not found: {}", node_id))?;
            let type_id = node_json["type"].as_str().ok_or(format!("Node type not found: {}", node_id))?;
            let params = &node_json["data"];

            let handler = self.registry.get(type_id)
                .ok_or(format!("No handler registered for type: {}", type_id))?;

            // Resolve inputs from edges
            let mut inputs = PortMap::new();
            if let Some(node_idx) = dag.node_map.get(&node_id) {
                // Find all incoming edges
                for edge in dag.graph.edges_directed(*node_idx, petgraph::Direction::Incoming) {
                    let source_idx = edge.source();
                    let source_id = &dag.graph[source_idx];
                    let metadata = edge.weight();

                    if let Some(prev_outputs) = node_outputs.get(source_id) {
                        if let Some(val) = prev_outputs.get(&metadata.source_handle) {
                            inputs.insert(metadata.target_handle.clone(), val.clone());
                        }
                    }
                }
            }

            // Execute node
            tracing::info!("Executing node: {} ({})", node_id, type_id);
            let result = handler.execute(&self.ctx, &inputs, params).await
                .map_err(|e| format!("Node {} failed: {}", node_id, e))?;

            node_outputs.insert(node_id, result);
        }

        Ok(node_outputs)
    }
}
