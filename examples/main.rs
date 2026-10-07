// Copyright (c) 2022 Nitro Agility S.r.l.
// SPDX-License-Identifier: Apache-2.0

use permguard::{Action, Client, Entity, EvaluateRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://127.0.0.1:9094".to_owned());
    let client = Client::new(endpoint)?;

    let mut request = EvaluateRequest::new("acme", "main");
    request.profile = Some("default".to_owned());
    request.subject = Some(Entity::new("user", "amy"));
    request.resource = Some(Entity::new("document", "quarterly-report"));
    request.action = Some(Action::new("read"));
    request.request_id = Some("example-1".to_owned());

    let response = client.evaluate(&request).await?;
    println!("{}", if response.decision { "PERMIT" } else { "DENY" });
    Ok(())
}
