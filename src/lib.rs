// Copyright (c) 2022 Nitro Agility S.r.l.
// SPDX-License-Identifier: Apache-2.0

//! Client for the native, stateless Permguard PDP v1 HTTP and gRPC interfaces.

mod client;
mod error;
mod mapper;
mod model;

mod proto {
    tonic::include_proto!("permguard.data.v1");
}

pub use client::{Client, ClientOptions};
pub use error::{Error, Refusal};
pub use model::{
    Action, Configuration, Decision, DecisionContext, Endpoints, Entity, EvaluateRequest,
    EvaluateResponse, Evaluation, EvaluationOptions, EvaluationsSemantic, PartitionInput, Reason,
    StoreScope,
};

#[cfg(test)]
mod tests;
