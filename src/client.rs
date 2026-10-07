// Copyright (c) 2022 Nitro Agility S.r.l.
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use tonic::metadata::{Ascii, MetadataKey, MetadataValue};
use tonic::transport::{Channel, Endpoint};

use crate::error::{Error, Refusal};
use crate::mapper::{configuration_from_proto, request_to_proto, response_from_proto};
use crate::model::{Configuration, EvaluateRequest, EvaluateResponse};
use crate::proto;

const EVALUATION_PATH: &str = "/access/v1/evaluation";
const EVALUATIONS_PATH: &str = "/access/v1/evaluations";
const CONFIGURATION_PATH: &str = "/.well-known/permguard-pdp-v1-configuration";
const MAXIMUM_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

type GrpcHeader = (MetadataKey<Ascii>, MetadataValue<Ascii>);

/// Options shared by the HTTP and gRPC transports.
#[derive(Debug, Clone)]
pub struct ClientOptions {
    /// Per-call timeout.
    pub timeout: Duration,
    /// Static HTTP headers or ASCII gRPC metadata.
    pub headers: BTreeMap<String, String>,
    /// Optional preconfigured client used for HTTP endpoints.
    pub http_client: Option<reqwest::Client>,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            headers: BTreeMap::new(),
            http_client: None,
        }
    }
}

impl ClientOptions {
    /// Sets the per-call timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Adds a static header or metadata value.
    #[must_use]
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Supplies a preconfigured reqwest client for an HTTP endpoint.
    #[must_use]
    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }
}

#[derive(Clone)]
enum Transport {
    Http {
        base: String,
        client: reqwest::Client,
        headers: HeaderMap,
    },
    Grpc {
        channel: Channel,
        headers: Vec<GrpcHeader>,
    },
}

/// Client for the native, stateless Permguard PDP v1 interface.
#[derive(Clone)]
pub struct Client {
    transport: Transport,
    timeout: Duration,
}

impl Client {
    /// Creates a client. `http(s)://` selects JSON; `grpc(s)://` selects gRPC.
    pub fn new(endpoint: impl AsRef<str>) -> Result<Self, Error> {
        Self::with_options(endpoint, ClientOptions::default())
    }

    /// Creates a client with transport options.
    pub fn with_options(endpoint: impl AsRef<str>, options: ClientOptions) -> Result<Self, Error> {
        if options.timeout.is_zero() {
            return Err(Error::InvalidOption(
                "timeout must be greater than zero".to_owned(),
            ));
        }

        let parsed = reqwest::Url::parse(endpoint.as_ref())
            .map_err(|error| Error::InvalidEndpoint(error.to_string()))?;
        if parsed.host_str().is_none() {
            return Err(Error::InvalidEndpoint("host is required".to_owned()));
        }
        if !matches!(parsed.path(), "" | "/")
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(Error::InvalidEndpoint(
                "path, query, and fragment are not allowed".to_owned(),
            ));
        }
        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err(Error::InvalidEndpoint(
                "embedded credentials are not allowed".to_owned(),
            ));
        }

        let transport = match parsed.scheme() {
            "http" | "https" => Transport::Http {
                base: format!("{}://{}", parsed.scheme(), authority(&parsed)?),
                client: options.http_client.unwrap_or_default(),
                headers: http_headers(&options.headers)?,
            },
            "grpc" | "grpcs" => {
                let scheme = if parsed.scheme() == "grpcs" {
                    "https"
                } else {
                    "http"
                };
                let address = format!("{scheme}://{}", authority(&parsed)?);
                let channel = Endpoint::from_shared(address)?.connect_lazy();
                Transport::Grpc {
                    channel,
                    headers: grpc_headers(&options.headers)?,
                }
            }
            scheme => {
                return Err(Error::InvalidEndpoint(format!(
                    "unsupported scheme {scheme:?}"
                )));
            }
        };

        Ok(Self {
            transport,
            timeout: options.timeout,
        })
    }

    /// Evaluates one access request.
    pub async fn evaluate(&self, request: &EvaluateRequest) -> Result<EvaluateResponse, Error> {
        self.evaluate_inner(request, false).await
    }

    /// Evaluates a boxcarred request.
    pub async fn evaluate_many(
        &self,
        request: &EvaluateRequest,
    ) -> Result<EvaluateResponse, Error> {
        self.evaluate_inner(request, true).await
    }

    async fn evaluate_inner(
        &self,
        request: &EvaluateRequest,
        many: bool,
    ) -> Result<EvaluateResponse, Error> {
        match &self.transport {
            Transport::Http {
                base,
                client,
                headers,
            } => {
                let path = if many {
                    EVALUATIONS_PATH
                } else {
                    EVALUATION_PATH
                };
                let response = client
                    .post(format!("{base}{path}"))
                    .headers(headers.clone())
                    .timeout(self.timeout)
                    .json(request)
                    .send()
                    .await?;
                decode_http(response).await
            }
            Transport::Grpc { channel, headers } => {
                let mut client =
                    proto::policy_decision_point_client::PolicyDecisionPointClient::new(
                        channel.clone(),
                    );
                let request = grpc_request(request_to_proto(request)?, headers, self.timeout);
                let response = if many {
                    client.evaluate_many(request).await
                } else {
                    client.evaluate(request).await
                }
                .map_err(refusal_from_grpc)?
                .into_inner();
                Ok(response_from_proto(response))
            }
        }
    }

    /// Returns the native PDP v1 discovery document.
    pub async fn get_configuration(&self) -> Result<Configuration, Error> {
        match &self.transport {
            Transport::Http {
                base,
                client,
                headers,
            } => {
                let response = client
                    .get(format!("{base}{CONFIGURATION_PATH}"))
                    .headers(headers.clone())
                    .timeout(self.timeout)
                    .send()
                    .await?;
                decode_http(response).await
            }
            Transport::Grpc { channel, headers } => {
                let mut client =
                    proto::policy_decision_point_client::PolicyDecisionPointClient::new(
                        channel.clone(),
                    );
                let request =
                    grpc_request(proto::GetConfigurationRequest {}, headers, self.timeout);
                let response = client
                    .get_configuration(request)
                    .await
                    .map_err(refusal_from_grpc)?
                    .into_inner();
                Ok(configuration_from_proto(response))
            }
        }
    }
}

fn authority(url: &reqwest::Url) -> Result<String, Error> {
    let host = url
        .host_str()
        .ok_or_else(|| Error::InvalidEndpoint("host is required".to_owned()))?;
    let host = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_owned()
    };
    Ok(url
        .port()
        .map_or(host.clone(), |port| format!("{host}:{port}")))
}

fn http_headers(source: &BTreeMap<String, String>) -> Result<HeaderMap, Error> {
    source
        .iter()
        .map(|(name, value)| {
            let name = HeaderName::from_bytes(name.as_bytes())
                .map_err(|error| Error::InvalidOption(error.to_string()))?;
            let value = HeaderValue::from_str(value)
                .map_err(|error| Error::InvalidOption(error.to_string()))?;
            Ok((name, value))
        })
        .collect()
}

fn grpc_headers(source: &BTreeMap<String, String>) -> Result<Vec<GrpcHeader>, Error> {
    source
        .iter()
        .map(|(name, value)| {
            let name = MetadataKey::from_bytes(name.as_bytes())
                .map_err(|error| Error::InvalidOption(error.to_string()))?;
            let value = MetadataValue::try_from(value.as_str())
                .map_err(|error| Error::InvalidOption(error.to_string()))?;
            Ok((name, value))
        })
        .collect()
}

fn grpc_request<T>(body: T, headers: &[GrpcHeader], timeout: Duration) -> tonic::Request<T> {
    let mut request = tonic::Request::new(body);
    request.set_timeout(timeout);
    for (name, value) in headers {
        request.metadata_mut().insert(name.clone(), value.clone());
    }
    request
}

async fn decode_http<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, Error> {
    let status = response.status();
    if response
        .content_length()
        .is_some_and(|length| length > MAXIMUM_RESPONSE_BYTES as u64)
    {
        return Err(Error::InvalidValue(format!(
            "HTTP response exceeds {MAXIMUM_RESPONSE_BYTES} bytes"
        )));
    }
    let bytes = response.bytes().await?;
    if bytes.len() > MAXIMUM_RESPONSE_BYTES {
        return Err(Error::InvalidValue(format!(
            "HTTP response exceeds {MAXIMUM_RESPONSE_BYTES} bytes"
        )));
    }
    if !status.is_success() {
        let mut refusal = serde_json::from_slice::<Refusal>(&bytes).unwrap_or_else(|_| Refusal {
            error_class: http_class(status.as_u16()).to_owned(),
            code: "http_status".to_owned(),
            message: String::from_utf8_lossy(&bytes).trim().to_owned(),
            http_status: None,
            grpc_status: None,
        });
        if refusal.error_class.is_empty() {
            refusal.error_class = http_class(status.as_u16()).to_owned();
        }
        if refusal.code.is_empty() {
            refusal.code = "http_status".to_owned();
        }
        if refusal.message.is_empty() {
            refusal.message = status
                .canonical_reason()
                .unwrap_or("Permguard HTTP request failed")
                .to_owned();
        }
        refusal.http_status = Some(status.as_u16());
        return Err(refusal.into());
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn refusal_from_grpc(status: tonic::Status) -> Error {
    let error_class = status
        .metadata()
        .get("permguard-error-class")
        .and_then(|value| value.to_str().ok())
        .map_or_else(|| grpc_class(status.code()).to_owned(), str::to_owned);
    let code = status
        .metadata()
        .get("permguard-error-code")
        .and_then(|value| value.to_str().ok())
        .map_or_else(
            || format!("{:?}", status.code()).to_lowercase(),
            str::to_owned,
        );
    Refusal {
        error_class,
        code,
        message: status.message().to_owned(),
        http_status: None,
        grpc_status: Some(status.code()),
    }
    .into()
}

fn http_class(status: u16) -> &'static str {
    match status {
        400 | 422 => "validation",
        409 => "conflict",
        401 | 403 => "authorization",
        404 => "not_found",
        503 | 504 => "unavailable",
        _ => "internal",
    }
}

fn grpc_class(status: tonic::Code) -> &'static str {
    match status {
        tonic::Code::InvalidArgument | tonic::Code::OutOfRange => "validation",
        tonic::Code::FailedPrecondition | tonic::Code::AlreadyExists | tonic::Code::Aborted => {
            "conflict"
        }
        tonic::Code::Unauthenticated | tonic::Code::PermissionDenied => "authorization",
        tonic::Code::NotFound => "not_found",
        tonic::Code::Unavailable | tonic::Code::DeadlineExceeded => "unavailable",
        _ => "internal",
    }
}
