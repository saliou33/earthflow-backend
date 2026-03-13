use petgraph::algo::toposort;
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableDiGraph;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct EdgeMetadata {
    pub source_handle: String,
    pub target_handle: String,
}

pub struct WorkflowGraph {
    pub graph: StableDiGraph<String, EdgeMetadata>, // NodeId is the weight
    pub node_map: HashMap<String, NodeIndex>,
}

impl WorkflowGraph {
    pub fn from_json(graph_json: &Value) -> Result<Self, String> {
        let mut graph = StableDiGraph::new();
        let mut node_map = HashMap::new();

        let nodes = graph_json["nodes"].as_array().ok_or("Invalid graph: missing nodes")?;
        let edges = graph_json["edges"].as_array().ok_or("Invalid graph: missing edges")?;

        for node in nodes {
            let id = node["id"].as_str().ok_or("Invalid node: missing id")?.to_string();
            let idx = graph.add_node(id.clone());
            node_map.insert(id, idx);
        }

        for edge in edges {
            let source = edge["source"].as_str().ok_or("Invalid edge: missing source")?;
            let target = edge["target"].as_str().ok_or("Invalid edge: missing target")?;
            let source_handle = edge["sourceHandle"].as_str().unwrap_or("output").to_string();
            let target_handle = edge["targetHandle"].as_str().unwrap_or("input").to_string();

            let source_idx = match node_map.get(source) {
                Some(idx) => *idx,
                None => {
                    tracing::warn!("Skipping edge with missing source node: {}", source);
                    continue;
                }
            };

            let target_idx = match node_map.get(target) {
                Some(idx) => *idx,
                None => {
                    tracing::warn!("Skipping edge with missing target node: {}", target);
                    continue;
                }
            };

            graph.add_edge(source_idx, target_idx, EdgeMetadata { source_handle, target_handle });
        }

        Ok(Self { graph, node_map })
    }

    pub fn topological_sort(&self) -> Result<Vec<String>, String> {
        match toposort(&self.graph, None) {
            Ok(indices) => {
                let sorted_ids = indices.iter()
                    .map(|idx| self.graph[*idx].clone())
                    .collect();
                Ok(sorted_ids)
            }
            Err(_) => Err("Graph has cycles - not a valid DAG".to_string()),
        }
    }
}
