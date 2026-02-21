# The official Rust SDK for Permguard

[![GitHub License](https://img.shields.io/github/license/permguard/sdk-rust)](https://github.com/permguard/sdk-rust?tab=Apache-2.0-1-ov-file#readme)
[![X (formerly Twitter) Follow](https://img.shields.io/twitter/follow/permguard)](https://x.com/intent/follow?original_referer=https%3A%2F%2Fdeveloper.x.com%2F&ref_src=twsrc%5Etfw%7Ctwcamp%5Ebuttonembed%7Ctwterm%5Efollow%7Ctwgr%5ETwitterDev&screen_name=Permguard)

[![Documentation](https://img.shields.io/website?label=Docs&url=https%3A%2F%2Fwww.permguard.com%2F)](https://www.permguard.com/)
[![Build, test and publish the artifacts](https://github.com/permguard/sdk-rust/actions/workflows/sdk-rust-ci.yml/badge.svg)](https://github.com/permguard/sdk-rust/actions/workflows/sdk-rust-ci.yml)

[![Watch the video on YouTube](https://raw.githubusercontent.com/permguard/permguard-assets/refs/heads/main/video/permguard-thumbnail-preview.png)](https://youtu.be/cH_boKCpLQ8?si=i1fWFHT5kxQQJoYN)

[Watch the video on YouTube](https://youtu.be/cH_boKCpLQ8?si=i1fWFHT5kxQQJoYN)


The Permguard Rust SDK provides a simple and flexible client to perform authorization checks against a Permguard Policy Decision Point (PDP) service using gRPC.
Plase refer to the [Permguard Documentation](https://www.permguard.com/) for more information.

---

## Prerequisites

- **Rust Toolchain 1.89.0**

---

## Installation

Run the following command to install the SDK:

```bash
cargo add permguard
```

---

## Usage Example

Below is a sample Rust code demonstrating how to create a Permguard client, build an authorization request using a builder pattern, and process the authorization response:

```rust
let endpoint = AzEndpoint::new("http".to_string(), 9094, "localhost".to_string());
let config = AzConfig::new().with_endpoint(Some(endpoint));
let client = AzClient::new(config);

let principal = PrincipalBuilder::new("amy.smith@acmecorp.com")
    .with_source("keycloak")
    .with_type("user")
    .build();

let entity = {
    let mut map = HashMap::new();
    map.insert("uid".to_string(), json!({
        "type": "PharmaAuthZFlow::Platform::BranchInfo",
        "id": "subscription"
    }));
    map.insert("attrs".to_string(), json!({"active": true}));
    map.insert("parents".to_string(), json!([]));
    Some(map)
};

let entities = vec![entity];

let request = AzAtomicRequestBuilder::new(
    189106194833,
    "48335ae72b3b405eae9e4bd5b07732df",
    "platform-creator",
    "PharmaAuthZFlow::Platform::Subscription",
    "PharmaAuthZFlow::Platform::Action::create",
)
    .with_request_id("31243")
    .with_principal(principal)
    .with_subject_property("isSuperUser", Value::from(true))
    .with_subject_type("workload")
    .with_subject_source("keycloak")
    .with_resource_id("e3a786fd07e24bfa95ba4341d3695ae8")
    .with_resource_property("isEnabled", json!(true))
    .with_entities_map("cedar", entities)
    .with_action_property("isEnabled", json!(true))
    .with_context_property("isSubscriptionActive", json!(true))
    .with_context_property("time", json!("2025-01-23T16:17:46+00:00"))
    .build();

match client.check_auth(Some(request)).await {
    Ok(response) => {
        if response.decision {
            println!("✅ Authorization Permitted");
        } else {
            println!("❌ Authorization Denied");
            if let Some(ctx) = response.context {
                if let Some(admin) = ctx.reason_admin {
                    println!("-> Reason Admin: {}", admin.message);
                }
                if let Some(user) = ctx.reason_user {
                    println!("-> Reason User: {}", user.message);
                }
            }

            for eval in response.evaluations {
                if eval.decision {
                    println!("-> ✅ Authorization Permitted");
                }
                if let Some(ctx) = eval.context {
                    if let Some(admin) = ctx.reason_admin {
                        println!("-> Reason Admin: {}", admin.message);
                    }
                    if let Some(user) = ctx.reason_user {
                        println!("-> Reason User: {}", user.message);
                    }
                }
            }
        }
    }
    Err(e) => {
        eprintln!("❌ Failed to check auth: {}", e);
        return Err(Err(e.into()));
    }
}

Ok(Ok(()))
```

---

## Version Compatibility

Our SDK follows a versioning scheme aligned with the Permguard Server versions to ensure seamless integration. The versioning format is as follows:

**SDK Versioning Format:** `x.y.z`

- **x.y**: Indicates the compatible Permguard Server version.
- **z**: Represents the SDK's patch or minor updates specific to that server version.

**Compatibility Examples:**

- `SDK Version 1.3.0` is compatible with `Permguard Server 1.3`.
- `SDK Version 1.3.1` includes minor improvements or bug fixes for `Permguard Server 1.3`.

**Incompatibility Example:**

- `SDK Version 1.3.0` **may not be guaranteed** to be compatible with `Permguard Server 1.4` due to potential changes introduced in server version `1.4`.

**Important:** Ensure that the major and minor versions (`x.y`) of the SDK match those of your Permguard Server to maintain compatibility.

---

Created by [Nitro Agility](https://www.nitroagility.com/).