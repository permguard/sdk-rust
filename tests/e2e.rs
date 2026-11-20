// Copyright (c) 2022 Nitro Agility S.r.l.
// SPDX-License-Identifier: Apache-2.0

//! Against a real data plane: `scripts/e2e-server.sh` starts it from the Permguard release
//! `PERMGUARD_CONTRACT` pins, serving a ledger `e2e/policies` that permits `User:alice` to `read`
//! (BUG-0077). Run with `cargo test --test e2e -- --ignored`.

use permguard::{Action, Client, Entity, Error, EvaluateRequest, Evaluation};

fn endpoints() -> [(&'static str, String); 2] {
    let read = |name: &str| {
        std::env::var(name).unwrap_or_else(|_| {
            panic!("E2E_HTTP and E2E_GRPC name the data plane scripts/e2e-server.sh started")
        })
    };
    [("HTTP", read("E2E_HTTP")), ("gRPC", read("E2E_GRPC"))]
}

fn ask(id: &str) -> EvaluateRequest {
    let mut request = EvaluateRequest::new("e2e", "policies");
    request.subject = Some(Entity::new("User", id));
    request.resource = Some(Entity::new("Document", "d"));
    request.action = Some(Action::new("read"));
    request
}

fn refusal_code(error: Error) -> String {
    match error {
        Error::Refusal(refusal) => refusal.code,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Over both transports: a permit with its decision evidence, a deny, a batch, a refusal, a
/// request for a signed response, and the discovery document (BUG-0077, BUG-0274).
#[tokio::test]
#[ignore = "needs the data plane scripts/e2e-server.sh starts"]
async fn against_a_real_data_plane() {
    for (name, endpoint) in endpoints() {
        let client = Client::new(endpoint).unwrap();

        let permit = client.evaluate(&ask("alice")).await.unwrap();
        assert!(permit.decision, "{name}: alice reads");
        let evidence = permit.context.and_then(|context| context.evidence);
        assert!(
            evidence.is_some_and(|evidence| !evidence.status.is_empty()),
            "{name}: the decision's evidence"
        );
        assert!(
            !client.evaluate(&ask("bob")).await.unwrap().decision,
            "{name}: bob reads"
        );

        let mut batch = ask("alice");
        batch.evaluations = ["alice", "bob"]
            .into_iter()
            .map(|id| Evaluation {
                subject: Some(Entity::new("User", id)),
                ..Evaluation::default()
            })
            .collect();
        let many = client.evaluate_many(&batch).await.unwrap();
        let decisions: Vec<bool> = many.evaluations.iter().map(|item| item.decision).collect();
        assert_eq!(decisions, [true, false], "{name}: the batch");

        let mut missing = ask("alice");
        missing.ledger = "nope".to_owned();
        let error = client.evaluate(&missing).await.unwrap_err();
        assert_eq!(
            refusal_code(error),
            "ledger_not_served",
            "{name}: an unknown ledger"
        );

        // The nonce reaches the plane: a signed response is addressed to an authenticated caller,
        // and this one presents no token, so the plane refuses rather than answer unsigned. The
        // attestation's mapping is the unit tests'.
        let mut signed = ask("alice");
        signed.request_nonce = Some("ZTJlLW5vbmNlLTE2LWJ5dGVz".to_owned());
        let error = client.evaluate(&signed).await.unwrap_err();
        assert_eq!(
            refusal_code(error),
            "unauthenticated",
            "{name}: a nonce without a credential"
        );

        let configuration = client.get_configuration().await.unwrap();
        assert!(
            !configuration.version.is_empty()
                && !configuration.stage.is_empty()
                && configuration
                    .limits
                    .get("max_evaluations")
                    .copied()
                    .unwrap_or(0)
                    > 0
                && !configuration.response_signing.state.is_empty(),
            "{name}: the discovery document {configuration:?}"
        );
    }
}
