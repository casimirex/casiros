//! Save/load support for causality graphs.
//!
//! A [`CausalityEngine`] can be serialized to a stable, name-based snapshot and
//! reconstructed later. Snapshots are intended for model persistence, versioning,
//! and interchange between the API, CLI, and future storage backends.

use serde::{Deserialize, Serialize};

use crate::error::DagError;
use crate::graph::{CausalityEngine, FormulaKind, NodeKind};

/// A serializable, name-stable representation of a causality graph.
///
/// The snapshot stores only node names/kinds and named edges, making it immune
/// to internal identifier renumbering.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct EngineSnapshot {
    /// Nodes in the graph, in insertion order.
    pub nodes: Vec<SnapshotNode>,

    /// Directed edges, each expressed as `(dependency, dependent)` node names.
    pub edges: Vec<(String, String)>,
}

/// A single node inside an [`EngineSnapshot`].
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SnapshotNode {
    /// Human-readable node name.
    pub name: String,

    /// Computational kind of the node.
    #[serde(flatten)]
    pub kind: SnapshotNodeKind,
}

/// The computational kind of a snapshot node.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum SnapshotNodeKind {
    /// A raw numeric input provided at evaluation time.
    Input,
    /// A formula from the CASIROS core catalog.
    Formula {
        /// The specific formula and its port bindings.
        formula: FormulaKind,
    },
}

impl CausalityEngine {
    /// Serializes the engine to a stable [`EngineSnapshot`].
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_dag::graph::{CausalityEngine, FormulaKind, Port};
    /// use rust_decimal_macros::dec;
    ///
    /// let mut engine = CausalityEngine::new();
    /// engine.add_input("principal");
    /// engine.add_formula(
    ///     "fv",
    ///     FormulaKind::FutureValue {
    ///         present_value: Port::Constant(dec!(100.0)),
    ///         rate: Port::Constant(dec!(0.05)),
    ///         periods: Port::Constant(dec!(10)),
    ///     },
    /// );
    ///
    /// let snapshot = engine.to_snapshot();
    /// assert_eq!(snapshot.nodes.len(), 2);
    /// ```
    #[must_use]
    pub fn to_snapshot(&self) -> EngineSnapshot {
        let mut nodes = Vec::with_capacity(self.len());
        for node in self.nodes() {
            let kind = match node.kind() {
                NodeKind::Input => SnapshotNodeKind::Input,
                NodeKind::Formula(formula) => SnapshotNodeKind::Formula {
                    formula: formula.clone(),
                },
            };
            nodes.push(SnapshotNode {
                name: node.name().to_string(),
                kind,
            });
        }

        let mut edges = Vec::new();
        for (dependency, dependent) in self.edges() {
            edges.push((dependency.to_string(), dependent.to_string()));
        }

        return EngineSnapshot { nodes, edges };
    }

    /// Reconstructs an engine from a stable [`EngineSnapshot`].
    ///
    /// # Errors
    ///
    /// Returns [`DagError::DuplicateNodeName`] if a name is reused, or
    /// [`DagError::UnknownNodeName`] if an edge references a missing name.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_dag::graph::{CausalityEngine, FormulaKind, Port};
    /// use rust_decimal_macros::dec;
    ///
    /// let mut engine = CausalityEngine::new();
    /// engine.add_input("principal");
    /// engine.add_formula(
    ///     "fv",
    ///     FormulaKind::FutureValue {
    ///         present_value: Port::Constant(dec!(100.0)),
    ///         rate: Port::Constant(dec!(0.05)),
    ///         periods: Port::Constant(dec!(10)),
    ///     },
    /// );
    ///
    /// let snapshot = engine.to_snapshot();
    /// let restored = CausalityEngine::from_snapshot(&snapshot).unwrap();
    /// assert_eq!(restored.len(), 2);
    /// ```
    pub fn from_snapshot(snapshot: &EngineSnapshot) -> Result<Self, DagError> {
        let mut engine = CausalityEngine::new();
        let mut name_to_id: std::collections::HashMap<String, crate::graph::NodeId> =
            std::collections::HashMap::with_capacity(snapshot.nodes.len());

        for node in &snapshot.nodes {
            if name_to_id.contains_key(&node.name) {
                return Err(DagError::DuplicateNodeName {
                    name: node.name.clone(),
                });
            }

            let id = match &node.kind {
                SnapshotNodeKind::Input => engine.add_input(&node.name),
                SnapshotNodeKind::Formula { formula } => {
                    engine.add_formula(&node.name, formula.clone())
                }
            };
            name_to_id.insert(node.name.clone(), id);
        }

        for (dependency_name, dependent_name) in &snapshot.edges {
            let dependency =
                *name_to_id
                    .get(dependency_name)
                    .ok_or_else(|| DagError::UnknownNodeName {
                        name: dependency_name.clone(),
                    })?;
            let dependent =
                *name_to_id
                    .get(dependent_name)
                    .ok_or_else(|| DagError::UnknownNodeName {
                        name: dependent_name.clone(),
                    })?;
            engine.add_edge(dependency, dependent)?;
        }

        return Ok(engine);
    }
}
