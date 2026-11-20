// Copyright (c) 2022 Nitro Agility S.r.l.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use crate::mapper::{configuration_from_proto, request_to_proto, response_from_proto};
use crate::proto;
use crate::{Configuration, EvaluateRequest, EvaluateResponse};

/// BUG-0274: a request asks for a signed response with its nonce, and the answer's attestation
/// and decision evidence reach the caller over both transports.
#[test]
fn the_nonce_the_attestation_and_the_evidence_travel_on_both_transports() {
    let mut request = EvaluateRequest::new("z", "l");
    request.request_nonce = Some("bm9uY2Utb2Ytc2l4dGVlbg".to_owned());
    let mapped = request_to_proto(&request).unwrap();
    assert_eq!(mapped.request_nonce, "bm9uY2Utb2Ytc2l4dGVlbg");
    let encoded = serde_json::to_value(&request).unwrap();
    assert_eq!(encoded["request_nonce"], "bm9uY2Utb2Ytc2l4dGVlbg");
    let plain = serde_json::to_value(EvaluateRequest::new("z", "l")).unwrap();
    assert!(plain.get("request_nonce").is_none());

    let from_grpc = response_from_proto(proto::EvaluateResponse {
        decision: true,
        context: Some(proto::DecisionContext {
            id: "d-1".to_owned(),
            evidence: Some(proto::Evidence {
                mode: "required".to_owned(),
                status: "recorded".to_owned(),
                record_id: Some("rec-1".to_owned()),
                position: Some(proto::EvidencePosition {
                    host_id: "h".to_owned(),
                    run: "r".to_owned(),
                    sequence: "7".to_owned(),
                }),
            }),
            ..proto::DecisionContext::default()
        }),
        attestation: Some(proto::Attestation {
            protected: "p".to_owned(),
            payload: "b".to_owned(),
            signature: "s".to_owned(),
        }),
        ..proto::EvaluateResponse::default()
    });
    let from_http: EvaluateResponse = serde_json::from_str(
        r#"{"decision":true,"context":{"id":"d-1","evidence":{"mode":"required","status":"recorded","record_id":"rec-1","position":{"host_id":"h","run":"r","sequence":"7"}}},"attestation":{"protected":"p","payload":"b","signature":"s"}}"#,
    )
    .unwrap();
    assert_eq!(from_grpc, from_http);
    let evidence = from_http.context.unwrap().evidence.unwrap();
    assert_eq!(
        (
            evidence.status.as_str(),
            evidence.record_id.as_deref(),
            evidence.position.map(|p| p.sequence)
        ),
        ("recorded", Some("rec-1"), Some("7".to_owned()))
    );
    assert_eq!(from_http.attestation.unwrap().signature, "s");
}

/// BUG-0274: the discovery document's version, lifecycle, limits and response signing reach the
/// caller over both transports.
#[test]
fn the_discovery_document_is_read_whole_on_both_transports() {
    let from_grpc = configuration_from_proto(proto::GetConfigurationResponse {
        interface: "permguard.api.pdp.native.v1".to_owned(),
        pdp: "permguard".to_owned(),
        endpoints: Some(proto::Endpoints {
            evaluation: "/access/v1/evaluation".to_owned(),
            evaluations: "/access/v1/evaluations".to_owned(),
        }),
        capabilities: vec!["http".to_owned()],
        store_scope: Some(proto::StoreScope {
            r#in: "request".to_owned(),
            zone: "zone".to_owned(),
            ledger: "ledger".to_owned(),
            profile: "profile".to_owned(),
        }),
        version: "0.1.33".to_owned(),
        component: "data-plane".to_owned(),
        state: "serving".to_owned(),
        stage: "v1".to_owned(),
        sunset: None,
        limits: HashMap::from([("max_evaluations".to_owned(), 64)]),
        response_signing: Some(proto::ResponseSigning {
            state: "optional".to_owned(),
            validity_seconds: 300,
            clock_skew_seconds: 30,
        }),
    });
    let from_http: Configuration = serde_json::from_str(
        r#"{"interface":"permguard.api.pdp.native.v1","pdp":"permguard","endpoints":{"evaluation":"/access/v1/evaluation","evaluations":"/access/v1/evaluations"},"capabilities":["http"],"store_scope":{"in":"request","zone":"zone","ledger":"ledger","profile":"profile"},"version":"0.1.33","component":"data-plane","state":"serving","stage":"v1","limits":{"max_evaluations":64},"response_signing":{"state":"optional","validity_seconds":300,"clock_skew_seconds":30}}"#,
    )
    .unwrap();
    assert_eq!(from_grpc, from_http);
    assert_eq!(
        (
            from_http.version.as_str(),
            from_http.state.as_str(),
            from_http.limits["max_evaluations"],
            from_http.response_signing.validity_seconds
        ),
        ("0.1.33", "serving", 64, 300)
    );
}
