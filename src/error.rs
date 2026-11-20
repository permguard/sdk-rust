// Copyright (c) 2022 Nitro Agility S.r.l.
// SPDX-License-Identifier: Apache-2.0

use serde::Deserialize;
use thiserror::Error;

/// A structured PDP error. A deny is a successful response, not a refusal.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Error)]
#[error("{code}: {message}")]
pub struct Refusal {
    /// The error class. A denial (`unauthenticated`, `forbidden`) carries none on the wire: it is
    /// filled from the HTTP status, and the body's `code` is kept.
    #[serde(rename = "class", default)]
    pub error_class: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub message: String,
    #[serde(skip)]
    pub http_status: Option<u16>,
    #[serde(skip)]
    pub grpc_status: Option<tonic::Code>,
}

impl Refusal {
    /// The class of a call that failed in the client, the same on HTTP and gRPC and in every
    /// Permguard SDK (BUG-0284).
    pub const UNAVAILABLE: &'static str = "unavailable";
    /// The code of a call whose timeout passed before the data plane answered.
    pub const TIMEOUT: &'static str = "timeout";
    /// The code of a call that could not reach the data plane.
    pub const UNREACHABLE: &'static str = "unreachable";

    pub(crate) fn client_failure(code: &str, message: String) -> Self {
        Self {
            error_class: Self::UNAVAILABLE.to_owned(),
            code: code.to_owned(),
            message,
            http_status: None,
            grpc_status: None,
        }
    }
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
