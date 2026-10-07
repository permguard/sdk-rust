// Copyright (c) 2022 Nitro Agility S.r.l.
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Controls how a boxcarred request is executed and combined.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationsSemantic {
    /// Let the PDP apply its default semantic.
    #[default]
    Unspecified,
    /// Execute every evaluation.
    ExecuteAll,
    /// Stop after the first deny.
    DenyOnFirstDeny,
    /// Stop after the first permit.
    PermitOnFirstPermit,
}

/// Identifies a subject, resource, or caller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entity {
    pub r#type: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<Map<String, Value>>,
}

impl Entity {
    /// Creates an entity without properties.
    pub fn new(r#type: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            r#type: r#type.into(),
            id: id.into(),
            properties: None,
        }
    }
}

/// The operation being evaluated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Action {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<Map<String, Value>>,
}

impl Action {
    /// Creates an action without properties.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            properties: None,
        }
    }
}

/// Runtime data supplied to one named profile partition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionInput {
    pub r#type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl PartitionInput {
    /// Creates a partition input.
    pub fn new(r#type: impl Into<String>, data: impl Into<Value>) -> Self {
        Self {
            r#type: r#type.into(),
            data: Some(data.into()),
        }
    }
}

/// Overrides request defaults for one item in a boxcarred request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Evaluation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<Entity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<Entity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<Action>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<Map<String, Value>>,
    /// `None` inherits request inputs; `Some(empty)` explicitly replaces them with no inputs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition_inputs: Option<BTreeMap<String, PartitionInput>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// Boxcarred evaluation behavior.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationOptions {
    #[serde(default, skip_serializing_if = "is_unspecified")]
    pub evaluations_semantic: EvaluationsSemantic,
}

fn is_unspecified(value: &EvaluationsSemantic) -> bool {
    *value == EvaluationsSemantic::Unspecified
}

/// Request payload for the native PDP v1 interface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluateRequest {
    pub zone: String,
    pub ledger: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<Entity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<Entity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<Action>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<Map<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal: Option<Entity>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub partition_inputs: BTreeMap<String, PartitionInput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evaluations: Vec<Evaluation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<EvaluationOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl EvaluateRequest {
    /// Creates the minimal request that names a policy store.
    pub fn new(zone: impl Into<String>, ledger: impl Into<String>) -> Self {
        Self {
            zone: zone.into(),
            ledger: ledger.into(),
            profile: None,
            subject: None,
            resource: None,
            action: None,
            context: None,
            principal: None,
            partition_inputs: BTreeMap::new(),
            evaluations: Vec::new(),
            options: None,
            request_id: None,
        }
    }
}

/// Stable machine code and human-readable explanation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reason {
    pub code: String,
    pub message: String,
}

/// Evidence associated with a decision.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_admin: Option<Reason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_user: Option<Reason>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policies: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub absent_inputs: Vec<String>,
}

/// One item in a boxcarred response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    pub decision: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<DecisionContext>,
}

/// The PDP's decision response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluateResponse {
    pub decision: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<DecisionContext>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evaluations: Vec<Decision>,
}

/// HTTP bindings advertised by the PDP.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Endpoints {
    pub evaluation: String,
    pub evaluations: String,
}

/// Describes where and how a request names its policy store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreScope {
    pub r#in: String,
    pub zone: String,
    pub ledger: String,
    pub profile: String,
}

/// PDP native v1 discovery document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Configuration {
    pub interface: String,
    pub pdp: String,
    pub endpoints: Endpoints,
    pub capabilities: Vec<String>,
    pub store_scope: StoreScope,
}
