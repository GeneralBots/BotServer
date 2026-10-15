//! Knowledge Graph module for the Vibe platform (Issues #522, #799).
//!
//! Provides endpoints to retrieve knowledge graphs that represent the
//! relationships between Vibe runs, tool calls, and use cases. The graph
//! is returned as nodes and edges suitable for force-directed visualization
//! in the frontend (`vibe-graph.js`).
//!
//! # Endpoints
//! - `GET /api/vibe/graph/{use_case}` — graph for a use case
//! - `GET /api/vibe/graph/run/{run_id}` — graph for a specific run
//!
//! The graph is built from live run data via a [`GraphDataSource`], making
//! the builders pure functions that are trivially testable without HTTP.

use crate::api::VibeApiInner;
use axum::{
    extract::{Extension, Path, Query},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

pub const NODE_USE_CASE: &str = "use_case";
pub const NODE_RUN: &str = "run";
pub const NODE_TOOL: &str = "tool";
pub const REL_CONTAINS: &str = "contains";
pub const REL_TRIGGERED: &str = "triggered";

/// #1446 — canvas bounds: at most this many tool nodes are drawn (top by
/// call count) and the runs×tools triggered edges are capped so the graph
/// stays readable for long-lived workspaces.
const MAX_TOOL_NODES: usize = 12;
const MAX_TRIGGERED_EDGES: usize = 400;

/// A single node in the knowledge graph (e.g., a use case, run, tool call).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Unique identifier for the node.
    pub id: String,
    /// Human-readable label displayed on the graph canvas.
    pub label: String,
    /// Category of the node (see `NODE_*` constants).
    pub node_type: String,
    /// Arbitrary key-value metadata attached to the node.
    pub properties: HashMap<String, String>,
}

/// A directed edge connecting two graph nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source node ID.
    pub source: String,
    /// Target node ID.
    pub target: String,
    /// Relationship label (e.g., "contains", "triggered").
    pub relationship: String,
    /// Edge thickness hint for visualization (0.0–1.0).
    pub weight: f64,
}

/// Complete knowledge graph composed of nodes and edges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    /// All nodes in the graph.
    pub nodes: Vec<GraphNode>,
    /// All edges connecting the nodes.
    pub edges: Vec<GraphEdge>,
}

/// JSON response wrapper for graph API calls.
#[derive(Debug, Serialize)]
pub struct GraphResponse {
    /// Whether the request succeeded.
    pub success: bool,
    /// The knowledge graph payload (absent on error).
    pub graph: Option<KnowledgeGraph>,
    /// Error message if the request failed.
    pub error: Option<String>,
}

/// Lightweight projection of a run used to build graph nodes from live data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunNodeInfo {
    /// Run UUID as a string.
    pub run_id: String,
    /// Use case this run belongs to (e.g. "software_development").
    pub use_case: String,
    /// Current run state (pending, running, completed, ...).
    pub state: String,
    /// User intent captured when the run was created.
    pub intent: String,
    /// Names of the tools triggered during this run.
    pub tool_names: Vec<String>,
    /// Vibe project this run operated on (uuid string); used to scope the
    /// graph to the selected project so unrelated runs do not pollute it.
    pub project_id: Option<String>,
    /// #1446 — when the run was created. The graph sorts "newest first" by
    /// this timestamp; a lexical `run_id` (UUID v4) sort is meaningless.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Future alias used by [`GraphDataSource::snapshot_runs`] so the trait
/// stays dyn-compatible without an async-fn-in-trait lint.
pub type GraphFuture<T> =
    std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'static>>;

/// Supplies run snapshots to the graph builders. Implemented against the live
/// in-memory run store in the API layer; mocked in tests.
pub trait GraphDataSource: Send + Sync {
    fn snapshot_runs(&self) -> GraphFuture<Vec<RunNodeInfo>>;
}

fn run_id_node(run: &RunNodeInfo) -> String {
    format!("run:{}", run.run_id)
}

fn tool_id(tool_name: &str) -> String {
    format!("tool:{tool_name}")
}

fn weight_for(count: u32) -> f64 {
    ((count as f64) / 10.0).min(1.0)
}

/// Builds a graph for a use case containing its runs and the tools they
/// triggered. Runs from other use cases are filtered out; when `project_id`
/// is set, only runs belonging to that project are included (so the graph
/// reflects the selected project instead of every run in the workspace).
pub fn build_use_case_graph(
    use_case: &str,
    runs: &[RunNodeInfo],
    project_id: Option<&str>,
) -> KnowledgeGraph {
    let mut nodes = vec![GraphNode {
        id: "root".to_string(),
        label: format!("Vibe Graph for {use_case}"),
        node_type: NODE_USE_CASE.to_string(),
        properties: HashMap::from([("use_case".to_string(), use_case.to_string())]),
    }];
    let mut edges: Vec<GraphEdge> = Vec::new();
    let mut tool_counts: HashMap<String, u32> = HashMap::new();

    for run in runs {
        if run.use_case != use_case {
            continue;
        }
        if let Some(pid) = project_id {
            if run.project_id.as_deref() != Some(pid) {
                continue;
            }
        }
        nodes.push(GraphNode {
            id: run_id_node(run),
            label: format!("Run {}", run.run_id),
            node_type: NODE_RUN.to_string(),
            properties: HashMap::from([
                ("state".to_string(), run.state.clone()),
                ("intent".to_string(), run.intent.clone()),
                ("tools".to_string(), run.tool_names.len().to_string()),
            ]),
        });
        edges.push(GraphEdge {
            source: "root".to_string(),
            target: run_id_node(run),
            relationship: REL_CONTAINS.to_string(),
            weight: 1.0,
        });
        for tool in &run.tool_names {
            *tool_counts.entry(tool.clone()).or_default() += 1;
        }
    }

    let mut tool_names: Vec<String> = tool_counts.keys().cloned().collect();
    // #1446 — bound the canvas: draw only the most-called tools (the
    // runs×tools edge product is unbounded otherwise — 800 edges at limit
    // 100×8) and cap the triggered edges; remaining calls aggregate into one
    // "other tools" node so nothing silently disappears.
    tool_names.sort_by(|a, b| {
        tool_counts
            .get(b)
            .copied()
            .unwrap_or_default()
            .cmp(&tool_counts.get(a).copied().unwrap_or_default())
            .then(a.cmp(b))
    });
    tool_names.truncate(MAX_TOOL_NODES);
    let drawn: std::collections::HashSet<String> = tool_names.iter().cloned().collect();
    let other_calls: u32 = tool_counts
        .iter()
        .filter(|(t, _)| !drawn.contains(*t))
        .map(|(_, c)| c)
        .sum();
    for tool in tool_names.clone() {
        let count = tool_counts.get(&tool).copied().unwrap_or_default();
        nodes.push(GraphNode {
            id: tool_id(&tool),
            label: tool.clone(),
            node_type: NODE_TOOL.to_string(),
            properties: HashMap::from([("calls".to_string(), count.to_string())]),
        });
    }
    if other_calls > 0 {
        nodes.push(GraphNode {
            id: "tool-other".to_string(),
            label: "other tools".to_string(),
            node_type: NODE_TOOL.to_string(),
            properties: HashMap::from([("calls".to_string(), other_calls.to_string())]),
        });
    }
    let mut edge_budget = MAX_TRIGGERED_EDGES;
    for run in runs {
        if run.use_case != use_case {
            continue;
        }
        if let Some(pid) = project_id {
            if run.project_id.as_deref() != Some(pid) {
                continue;
            }
        }
        for tool in &run.tool_names {
            if edge_budget == 0 {
                break;
            }
            if !drawn.contains(tool) {
                continue;
            }
            edge_budget -= 1;
            let count = tool_counts.get(tool).copied().unwrap_or_default();
            edges.push(GraphEdge {
                source: run_id_node(run),
                target: tool_id(tool),
                relationship: REL_TRIGGERED.to_string(),
                weight: weight_for(count),
            });
        }
    }

    sort_graph(&mut nodes, &mut edges);
    KnowledgeGraph { nodes, edges }
}

/// Builds a graph scoped to a single run: the run node plus the tools it
/// triggered. Returns an error when no run matches the requested ID.
pub fn build_run_graph(run_id: &str, runs: &[RunNodeInfo]) -> Result<KnowledgeGraph, String> {
    let run = runs
        .iter()
        .find(|r| r.run_id == run_id)
        .ok_or_else(|| format!("Run {run_id} not found"))?;

    let tool_counts: HashMap<String, u32> =
        run.tool_names.iter().fold(HashMap::new(), |mut acc, t| {
            *acc.entry(t.clone()).or_default() += 1;
            acc
        });
    let mut tool_names: Vec<String> = tool_counts.keys().cloned().collect();
    tool_names.sort();

    let mut nodes = vec![GraphNode {
        id: run_id_node(run),
        label: format!("Run {}", run.run_id),
        node_type: NODE_RUN.to_string(),
        properties: HashMap::from([
            ("state".to_string(), run.state.clone()),
            ("intent".to_string(), run.intent.clone()),
            ("use_case".to_string(), run.use_case.clone()),
        ]),
    }];
    let mut edges: Vec<GraphEdge> = Vec::new();

    for tool in tool_names {
        let count = tool_counts.get(&tool).copied().unwrap_or_default();
        nodes.push(GraphNode {
            id: tool_id(&tool),
            label: tool.clone(),
            node_type: NODE_TOOL.to_string(),
            properties: HashMap::from([("calls".to_string(), count.to_string())]),
        });
        edges.push(GraphEdge {
            source: run_id_node(run),
            target: tool_id(&tool),
            relationship: REL_TRIGGERED.to_string(),
            weight: weight_for(count),
        });
    }

    let graph = KnowledgeGraph { nodes, edges };
    Ok(graph)
}

fn sort_graph(nodes: &mut [GraphNode], edges: &mut [GraphEdge]) {
    nodes.sort_by(|a, b| a.id.cmp(&b.id));
    edges.sort_by_key(|e| (e.source.clone(), e.target.clone(), e.relationship.clone()));
}

/// Optional query params for the use-case graph.
#[derive(Debug, Deserialize)]
pub struct GraphQuery {
    /// Scope the graph to a single Vibe project (its UUID); when absent the
    /// graph shows every run for the use case across projects.
    pub project_id: Option<String>,
    /// Maximum number of runs to include (newest first). Defaults to 20 so
    /// a long-lived workspace does not produce an unreadable graph.
    pub limit: Option<usize>,
}

/// Returns a knowledge graph for the given use case, built from live runs.
/// Optionally scoped to a project and capped to the most recent `limit` runs.
pub(crate) async fn get_knowledge_graph(
    Extension(api): Extension<Arc<VibeApiInner>>,
    Path(use_case): Path<String>,
    Query(query): Query<GraphQuery>,
) -> Json<GraphResponse> {
    let mut runs = api.snapshot_runs().await;
    let project_id = query.project_id.as_deref();
    if let Some(pid) = project_id {
        runs.retain(|r| r.project_id.as_deref() == Some(pid));
    }
    // #1446 — newest first by created_at (a lexical run_id sort is
    // meaningless for UUID v4), then cap so the canvas stays readable.
    runs.sort_by_key(|a| std::cmp::Reverse(a.created_at));
    let limit = query.limit.unwrap_or(20).min(100);
    if runs.len() > limit {
        runs.truncate(limit);
    }
    Json(GraphResponse {
        success: true,
        graph: Some(build_use_case_graph(&use_case, &runs, project_id)),
        error: None,
    })
}

/// Returns a knowledge graph scoped to a specific Vibe run.
pub(crate) async fn get_run_graph(
    Extension(api): Extension<Arc<VibeApiInner>>,
    Path(run_id): Path<String>,
) -> Json<GraphResponse> {
    let runs = api.snapshot_runs().await;
    match build_run_graph(&run_id, &runs) {
        Ok(graph) => Json(GraphResponse {
            success: true,
            graph: Some(graph),
            error: None,
        }),
        Err(error) => Json(GraphResponse {
            success: false,
            graph: None,
            error: Some(error),
        }),
    }
}