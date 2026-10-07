// Copyright (c) 2022 Nitro Agility S.r.l.
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Request, Response, Status};

use crate::proto;
use crate::{
    Action, Client, ClientOptions, Entity, Error, EvaluateRequest, Evaluation, EvaluationOptions,
    EvaluationsSemantic,
};

#[derive(Clone, Default)]
struct HttpState {
    paths: Arc<Mutex<Vec<String>>>,
    body: Arc<Mutex<Option<Value>>>,
    tenant: Arc<Mutex<Option<String>>>,
}

#[tokio::test]
async fn http_uses_native_paths_and_preserves_empty_partition_inputs() {
    let state = HttpState::default();
    let router = Router::new()
        .route("/access/v1/evaluations", post(http_evaluate))
        .route(
            "/.well-known/permguard-pdp-v1-configuration",
            get(http_configuration),
        )
        .with_state(state.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
    });

    let client = Client::with_options(
        format!("http://{address}"),
        ClientOptions::default().with_header("x-tenant", "acme"),
    )
    .unwrap();
    let response = client.evaluate_many(&sample_request()).await.unwrap();
    let configuration = client.get_configuration().await.unwrap();

    assert!(response.decision);
    assert_eq!(response.request_id.as_deref(), Some("req-1"));
    assert_eq!(configuration.interface, "permguard.api.pdp.native.v1");
    assert_eq!(
        *state.paths.lock().unwrap(),
        [
            "/access/v1/evaluations",
            "/.well-known/permguard-pdp-v1-configuration"
        ]
    );
    assert_eq!(state.tenant.lock().unwrap().as_deref(), Some("acme"));
    assert_eq!(
        state.body.lock().unwrap().as_ref().unwrap()["evaluations"][0]["partition_inputs"],
        json!({})
    );
    assert_eq!(
        state.body.lock().unwrap().as_ref().unwrap()["options"]["evaluations_semantic"],
        "execute_all"
    );

    let mut refused = sample_request();
    refused.ledger = "refuse".to_owned();
    let Error::Refusal(refusal) = client.evaluate_many(&refused).await.unwrap_err() else {
        panic!("expected refusal");
    };
    assert_eq!(refusal.error_class, "validation");
    assert_eq!(refusal.code, "invalid_ledger");
    assert_eq!(refusal.http_status, Some(422));

    let mut conflict = sample_request();
    conflict.ledger = "conflict".to_owned();
    let Error::Refusal(refusal) = client.evaluate_many(&conflict).await.unwrap_err() else {
        panic!("expected conflict refusal");
    };
    assert_eq!(refusal.error_class, "conflict");
    assert_eq!(refusal.http_status, Some(409));

    let _ = shutdown_tx.send(());
    server.await.unwrap();
}

async fn http_evaluate(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    state
        .paths
        .lock()
        .unwrap()
        .push("/access/v1/evaluations".to_owned());
    let refuse = body["ledger"] == "refuse";
    let conflict = body["ledger"] == "conflict";
    *state.body.lock().unwrap() = Some(body);
    *state.tenant.lock().unwrap() = headers
        .get("x-tenant")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    if refuse {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "class": "validation",
                "code": "invalid_ledger",
                "message": "bad ledger"
            })),
        );
    }
    if conflict {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "code": "ledger_conflict", "message": "ledger changed" })),
        );
    }
    (
        StatusCode::OK,
        Json(json!({
            "decision": true,
            "request_id": "req-1",
            "evaluations": [{ "decision": true, "request_id": "item-1" }]
        })),
    )
}

async fn http_configuration(State(state): State<HttpState>) -> Json<Value> {
    state
        .paths
        .lock()
        .unwrap()
        .push("/.well-known/permguard-pdp-v1-configuration".to_owned());
    Json(configuration_json())
}

#[tokio::test]
async fn grpc_uses_native_service_and_maps_refusal_metadata() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let incoming = TcpListenerStream::new(listener);
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                proto::policy_decision_point_server::PolicyDecisionPointServer::new(TestPdp),
            )
            .serve_with_incoming_shutdown(incoming, async move {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
    });

    let client = Client::new(format!("grpc://{address}")).unwrap();
    let response = client.evaluate_many(&sample_request()).await.unwrap();
    let configuration = client.get_configuration().await.unwrap();
    assert!(response.decision);
    assert_eq!(response.request_id.as_deref(), Some("req-1"));
    assert_eq!(configuration.interface, "permguard.api.pdp.native.v1");

    let mut refused = sample_request();
    refused.ledger = "refuse".to_owned();
    let error = client.evaluate(&refused).await.unwrap_err();
    let Error::Refusal(refusal) = error else {
        panic!("expected refusal");
    };
    assert_eq!(refusal.error_class, "validation");
    assert_eq!(refusal.code, "invalid_ledger");
    assert_eq!(refusal.grpc_status, Some(tonic::Code::InvalidArgument));

    let mut conflict = sample_request();
    conflict.ledger = "conflict".to_owned();
    let Error::Refusal(refusal) = client.evaluate(&conflict).await.unwrap_err() else {
        panic!("expected conflict refusal");
    };
    assert_eq!(refusal.error_class, "conflict");
    assert_eq!(refusal.grpc_status, Some(tonic::Code::FailedPrecondition));

    let _ = shutdown_tx.send(());
    server.await.unwrap();
}

#[derive(Debug)]
struct TestPdp;

#[tonic::async_trait]
impl proto::policy_decision_point_server::PolicyDecisionPoint for TestPdp {
    async fn evaluate(
        &self,
        request: Request<proto::EvaluateRequest>,
    ) -> Result<Response<proto::EvaluateResponse>, Status> {
        respond(request.into_inner())
    }

    async fn evaluate_many(
        &self,
        request: Request<proto::EvaluateRequest>,
    ) -> Result<Response<proto::EvaluateResponse>, Status> {
        respond(request.into_inner())
    }

    async fn get_configuration(
        &self,
        _request: Request<proto::GetConfigurationRequest>,
    ) -> Result<Response<proto::GetConfigurationResponse>, Status> {
        Ok(Response::new(proto::GetConfigurationResponse {
            interface: "permguard.api.pdp.native.v1".to_owned(),
            pdp: "test".to_owned(),
            endpoints: Some(proto::Endpoints {
                evaluation: "/access/v1/evaluation".to_owned(),
                evaluations: "/access/v1/evaluations".to_owned(),
            }),
            capabilities: vec!["http".to_owned(), "grpc".to_owned()],
            store_scope: Some(proto::StoreScope {
                r#in: "request".to_owned(),
                zone: "zone".to_owned(),
                ledger: "ledger".to_owned(),
                profile: "profile".to_owned(),
            }),
        }))
    }
}

fn respond(request: proto::EvaluateRequest) -> Result<Response<proto::EvaluateResponse>, Status> {
    assert_eq!(
        request.evaluations_semantic,
        proto::EvaluationsSemantic::ExecuteAll as i32
    );
    assert!(request.evaluations[0].partition_inputs.is_some());
    if request.ledger == "refuse" {
        let mut status = Status::invalid_argument("bad ledger");
        status
            .metadata_mut()
            .insert("permguard-error-class", "validation".parse().unwrap());
        status
            .metadata_mut()
            .insert("permguard-error-code", "invalid_ledger".parse().unwrap());
        return Err(status);
    }
    if request.ledger == "conflict" {
        return Err(Status::failed_precondition("ledger changed"));
    }
    Ok(Response::new(proto::EvaluateResponse {
        decision: true,
        request_id: request.request_id,
        context: None,
        evaluations: Vec::new(),
    }))
}

fn sample_request() -> EvaluateRequest {
    let mut request = EvaluateRequest::new("acme", "main");
    request.profile = Some("default".to_owned());
    request.subject = Some(Entity::new("user", "amy"));
    request.resource = Some(Entity::new("document", "report"));
    request.action = Some(Action::new("read"));
    request.request_id = Some("req-1".to_owned());
    request.evaluations.push(Evaluation {
        request_id: Some("item-1".to_owned()),
        partition_inputs: Some(BTreeMap::new()),
        ..Evaluation::default()
    });
    request.options = Some(EvaluationOptions {
        evaluations_semantic: EvaluationsSemantic::ExecuteAll,
    });
    request
}

fn configuration_json() -> Value {
    json!({
        "interface": "permguard.api.pdp.native.v1",
        "pdp": "test",
        "endpoints": {
            "evaluation": "/access/v1/evaluation",
            "evaluations": "/access/v1/evaluations"
        },
        "capabilities": ["http", "grpc"],
        "store_scope": {
            "in": "request",
            "zone": "zone",
            "ledger": "ledger",
            "profile": "profile"
        }
    })
}
