// Copyright (c) 2022 Nitro Agility S.r.l.
// SPDX-License-Identifier: Apache-2.0

use permguard::{Action, Client, Entity, EvaluateRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("PERMGUARD_PDP_URL").ok())
        .unwrap_or_else(|| "grpc://localhost:7443".to_owned());
    let client = Client::new(endpoint)?;

    let mut request = EvaluateRequest::new("acme", "main-ledger");
    request.profile = Some("gateway".to_owned());
    request.subject = Some(Entity::new("User", "alice"));
    request.resource = Some(Entity::new("Document", "budget-2026"));
    request.action = Some(Action::new("read"));
    request.request_id = Some("example-1".to_owned());

    let response = client.evaluate(&request).await?;
    println!("{}", if response.decision { "PERMIT" } else { "DENY" });
    Ok(())
}
