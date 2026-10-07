// Copyright (c) 2022 Nitro Agility S.r.l.
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use prost_types::{ListValue, Struct, Value, value::Kind};
use serde_json::{Map, Number};

use crate::error::Error;
use crate::model::{
    Action, Configuration, Decision, DecisionContext, Endpoints, Entity, EvaluateRequest,
    EvaluateResponse, Evaluation, EvaluationsSemantic, PartitionInput, Reason, StoreScope,
};
use crate::proto;

const MAXIMUM_EXACT_INTEGER: u64 = 9_007_199_254_740_991;

pub(crate) fn request_to_proto(source: &EvaluateRequest) -> Result<proto::EvaluateRequest, Error> {
    Ok(proto::EvaluateRequest {
        zone: source.zone.clone(),
        ledger: source.ledger.clone(),
        profile: source.profile.clone().unwrap_or_default(),
        subject: source.subject.as_ref().map(entity_to_proto).transpose()?,
        resource: source.resource.as_ref().map(entity_to_proto).transpose()?,
        action: source.action.as_ref().map(action_to_proto).transpose()?,
        context: source.context.as_ref().map(map_to_proto).transpose()?,
        principal: source.principal.as_ref().map(entity_to_proto).transpose()?,
        evaluations: source
            .evaluations
            .iter()
            .map(evaluation_to_proto)
            .collect::<Result<_, _>>()?,
        evaluations_semantic: semantic_to_proto(
            source
                .options
                .as_ref()
                .map_or(EvaluationsSemantic::Unspecified, |value| {
                    value.evaluations_semantic
                }),
        ),
        request_id: source.request_id.clone().unwrap_or_default(),
        partition_inputs: inputs_to_proto(&source.partition_inputs)?,
    })
}

fn entity_to_proto(source: &Entity) -> Result<proto::Entity, Error> {
    Ok(proto::Entity {
        r#type: source.r#type.clone(),
        id: source.id.clone(),
        properties: source.properties.as_ref().map(map_to_proto).transpose()?,
    })
}

fn action_to_proto(source: &Action) -> Result<proto::Action, Error> {
    Ok(proto::Action {
        name: source.name.clone(),
        properties: source.properties.as_ref().map(map_to_proto).transpose()?,
    })
}

fn evaluation_to_proto(source: &Evaluation) -> Result<proto::Evaluation, Error> {
    Ok(proto::Evaluation {
        subject: source.subject.as_ref().map(entity_to_proto).transpose()?,
        resource: source.resource.as_ref().map(entity_to_proto).transpose()?,
        action: source.action.as_ref().map(action_to_proto).transpose()?,
        context: source.context.as_ref().map(map_to_proto).transpose()?,
        request_id: source.request_id.clone().unwrap_or_default(),
        partition_inputs: source
            .partition_inputs
            .as_ref()
            .map(|inputs| -> Result<proto::PartitionInputs, Error> {
                Ok(proto::PartitionInputs {
                    inputs: inputs_to_proto(inputs)?,
                })
            })
            .transpose()?,
    })
}

fn inputs_to_proto(
    source: &BTreeMap<String, PartitionInput>,
) -> Result<std::collections::HashMap<String, proto::PartitionInput>, Error> {
    source
        .iter()
        .map(|(name, input)| {
            Ok((
                name.clone(),
                proto::PartitionInput {
                    r#type: input.r#type.clone(),
                    data: input.data.as_ref().map(value_to_proto).transpose()?,
                },
            ))
        })
        .collect()
}

fn map_to_proto(source: &Map<String, serde_json::Value>) -> Result<Struct, Error> {
    Ok(Struct {
        fields: source
            .iter()
            .map(|(name, value)| Ok((name.clone(), value_to_proto(value)?)))
            .collect::<Result<_, Error>>()?,
    })
}

fn value_to_proto(source: &serde_json::Value) -> Result<Value, Error> {
    let kind = match source {
        serde_json::Value::Null => Kind::NullValue(0),
        serde_json::Value::Bool(value) => Kind::BoolValue(*value),
        serde_json::Value::Number(value) => Kind::NumberValue(number_to_f64(value)?),
        serde_json::Value::String(value) => Kind::StringValue(value.clone()),
        serde_json::Value::Array(values) => Kind::ListValue(ListValue {
            values: values
                .iter()
                .map(value_to_proto)
                .collect::<Result<_, _>>()?,
        }),
        serde_json::Value::Object(values) => Kind::StructValue(map_to_proto(values)?),
    };
    Ok(Value { kind: Some(kind) })
}

fn number_to_f64(source: &Number) -> Result<f64, Error> {
    if let Some(value) = source.as_i64() {
        if value.unsigned_abs() > MAXIMUM_EXACT_INTEGER {
            return Err(Error::InvalidValue(format!(
                "integer {value} is not exactly representable by protobuf Value"
            )));
        }
    } else if let Some(value) = source.as_u64() {
        if value > MAXIMUM_EXACT_INTEGER {
            return Err(Error::InvalidValue(format!(
                "integer {value} is not exactly representable by protobuf Value"
            )));
        }
    }
    source
        .as_f64()
        .filter(|value| value.is_finite())
        .ok_or_else(|| Error::InvalidValue(format!("number {source} is not finite")))
}

fn semantic_to_proto(source: EvaluationsSemantic) -> i32 {
    match source {
        EvaluationsSemantic::Unspecified => proto::EvaluationsSemantic::Unspecified.into(),
        EvaluationsSemantic::ExecuteAll => proto::EvaluationsSemantic::ExecuteAll.into(),
        EvaluationsSemantic::DenyOnFirstDeny => proto::EvaluationsSemantic::DenyOnFirstDeny.into(),
        EvaluationsSemantic::PermitOnFirstPermit => {
            proto::EvaluationsSemantic::PermitOnFirstPermit.into()
        }
    }
}

pub(crate) fn response_from_proto(source: proto::EvaluateResponse) -> EvaluateResponse {
    EvaluateResponse {
        decision: source.decision,
        request_id: empty_to_none(source.request_id),
        context: source.context.map(context_from_proto),
        evaluations: source
            .evaluations
            .into_iter()
            .map(decision_from_proto)
            .collect(),
    }
}

fn decision_from_proto(source: proto::Decision) -> Decision {
    Decision {
        decision: source.decision,
        request_id: empty_to_none(source.request_id),
        context: source.context.map(context_from_proto),
    }
}

fn context_from_proto(source: proto::DecisionContext) -> DecisionContext {
    DecisionContext {
        id: empty_to_none(source.id),
        reason_admin: source.reason_admin.map(reason_from_proto),
        reason_user: source.reason_user.map(reason_from_proto),
        policies: source.policies,
        absent_inputs: source.absent_inputs,
    }
}

fn reason_from_proto(source: proto::Reason) -> Reason {
    Reason {
        code: source.code,
        message: source.message,
    }
}

pub(crate) fn configuration_from_proto(source: proto::GetConfigurationResponse) -> Configuration {
    let endpoints = source.endpoints.unwrap_or_default();
    let scope = source.store_scope.unwrap_or_default();
    Configuration {
        interface: source.interface,
        pdp: source.pdp,
        endpoints: Endpoints {
            evaluation: endpoints.evaluation,
            evaluations: endpoints.evaluations,
        },
        capabilities: source.capabilities,
        store_scope: StoreScope {
            r#in: scope.r#in,
            zone: scope.zone,
            ledger: scope.ledger,
            profile: scope.profile,
        },
    }
}

fn empty_to_none(source: String) -> Option<String> {
    (!source.is_empty()).then_some(source)
}
