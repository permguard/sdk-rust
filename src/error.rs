// Copyright (c) 2022 Nitro Agility S.r.l.
// SPDX-License-Identifier: Apache-2.0

use serde::Deserialize;
use thiserror::Error;

/// A structured PDP error. A deny is a successful response, not a refusal.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Error)]
#[error("{code}: {message}")]
pub struct Refusal {
    #[serde(rename = "class")]
    pub error_class: String,
    pub code: String,
    pub message: String,
    #[serde(skip)]
    pub http_status: Option<u16>,
    #[serde(skip)]
    pub grpc_status: Option<tonic::Code>,
}

/// Error returned by the SDK.
#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid Permguard endpoint: {0}")]
    InvalidEndpoint(String),
    #[error("invalid client option: {0}")]
    InvalidOption(String),
    #[error("invalid JSON value for gRPC: {0}")]
    InvalidValue(String),
    #[error("Permguard HTTP transport failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("invalid Permguard JSON response: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Permguard gRPC transport failed: {0}")]
    GrpcTransport(#[from] tonic::transport::Error),
    #[error(transparent)]
    Refusal(#[from] Refusal),
}
