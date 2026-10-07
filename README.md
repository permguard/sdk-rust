<!-- Copyright (c) 2022 Nitro Agility S.r.l. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Permguard Rust SDK

The official Rust client for the native, stateless Permguard PDP v1 interface. One async API
supports HTTP/JSON and gRPC; the endpoint scheme selects the transport.

## Install

```console
cargo add permguard
```

The crate requires Rust 1.85 or newer.

## Evaluate

```rust
use permguard::{Action, Client, Entity, EvaluateRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use http(s):// for JSON or grpc(s):// for gRPC.
    let client = Client::new("http://127.0.0.1:9094")?;

    let mut request = EvaluateRequest::new("acme", "main");
    request.profile = Some("default".to_owned());
    request.subject = Some(Entity::new("user", "amy"));
    request.resource = Some(Entity::new("document", "quarterly-report"));
    request.action = Some(Action::new("read"));
    request.request_id = Some("request-1".to_owned());

    let response = client.evaluate(&request).await?;
    println!("{}", if response.decision { "PERMIT" } else { "DENY" });
    Ok(())
}
```

For gRPC, only the endpoint changes:

```rust
let client = Client::new("grpc://127.0.0.1:9094")?;
```

`Client::evaluate_many` sends boxcarred evaluations to `/access/v1/evaluations` or the gRPC
`EvaluateMany` method. `Client::get_configuration` reads the PDP discovery document. The client is
cheap to clone and reuses its HTTP pool or gRPC channel.

Static headers, a timeout, and a preconfigured `reqwest::Client` can be supplied with
`ClientOptions`. HTTP refusals and gRPC status failures are returned as `Error::Refusal`, including
the stable Permguard error class and code. A normal deny remains a successful response whose
`decision` is `false`.

## Development

```console
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --locked
cargo package --locked
```

The published crate includes `LICENSE`, `NOTICE.md`, `THIRD_PARTY_NOTICES.md`, and `TRADEMARKS.md`.
