// Copyright (c) 2022 Nitro Agility S.r.l.
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use permguard::az_client::*;
use permguard::az_req::action_builder::ActionBuilder;
use permguard::az_req::az_atomic_request_builder::AzAtomicRequestBuilder;
use permguard::az_req::az_request_builder::AzRequestBuilder;
use permguard::az_req::context_builder::ContextBuilder;
use permguard::az_req::evaluation_builder::EvaluationBuilder;
use permguard::az_req::model::AzRequest;
use permguard::az_req::principal_builder::PrincipalBuilder;
use permguard::az_req::resource_builder::ResourceBuilder;
use permguard::az_req::subject_builder::SubjectBuilder;
use permguard::config::*;
use serde_json::{json, Value};
use tokio::{fs};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = match std::env::args().nth(1).as_deref() {
        None | Some("atomic") => atomic_test().await,
        Some("json") => json_test().await,
        Some("first") => first_test().await,
        Some(example) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("unknown example '{example}'; expected one of: atomic, json, first"),
            )
            .into());
        }
    };

    match result {
        Ok(value) => value,
        Err(value) => return value,
    }
}

async fn atomic_test() -> Result<Result<(), Box<dyn Error>>, Result<(), Box<dyn Error>>>{
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
}


async fn json_test() -> Result<Result<(), Box<dyn Error>>, Result<(), Box<dyn Error>>>{
    let endpoint = AzEndpoint::new("http".to_string(), 9094, "localhost".to_string());
    let config = AzConfig::new().with_endpoint(Some(endpoint));
    let client = AzClient::new(config);

    let mut file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    file_path.push("./json/ok_onlyone.json");

    if !file_path.exists() {
        eprintln!("❌ Failed to load the JSON file");
        return Err(Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Failed to load the JSON file",
        ))));
    }

    let json_content = fs::read_to_string(&file_path).await;
    let request: AzRequest = match serde_json::from_str(&json_content.unwrap()) {
        Ok(req) => req,
        Err(e) => {
            eprintln!("❌ Failed to parse JSON: {}", e);
            return Err(Err(Box::new(e) as Box<dyn std::error::Error>));
        }
    };

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
}

async fn first_test() -> Result<Result<(), Box<dyn Error>>, Result<(), Box<dyn Error>>> {
    let endpoint = AzEndpoint::new("http".to_string(), 9094, "localhost".to_string());
    let config = AzConfig::new().with_endpoint(Some(endpoint));
    let client = AzClient::new(config);

    // Create the Principal
    let principal = PrincipalBuilder::new("amy.smith@acmecorp.com".to_string())
        .with_source("keycloak".to_string())
        .with_type("user".to_string())
        .build();

    // Create a new subject
    let subject = SubjectBuilder::new("platform-creator".to_string())
        .with_source("keycloak".to_string())
        .with_type("workload".to_string())
        .with_property("isSuperUser".to_string(), serde_json::json!(true))
        .build();

    // Create a new resource
    let resource = ResourceBuilder::new("PharmaAuthZFlow::Platform::Subscription".to_string())
        .with_id("e3a786fd07e24bfa95ba4341d3695ae8".to_string())
        .with_property("isEnabled".to_string(), serde_json::json!(true))
        .build();

    // Create actions
    let action_view = ActionBuilder::new("PharmaAuthZFlow::Platform::Action::create".to_string())
        .with_property("isEnabled".to_string(), serde_json::json!(true))
        .build();

    let action_create = ActionBuilder::new("PharmaAuthZFlow::Platform::Action::create".to_string())
        .with_property("isEnabled".to_string(), serde_json::json!(false))
        .build();

    // Create a new Context
    let context = ContextBuilder::new()
        .with_property("time".to_string(), serde_json::json!("2025-01-23T16:17:46+00:00"))
        .with_property("isSubscriptionActive".to_string(), serde_json::json!(true))
        .build();

    // Create evaluations
    let evaluation_view = EvaluationBuilder::new(Some(subject.clone()), Some(resource.clone()), Some(action_view.clone()))
        .with_request_id("134".to_string())
        .build();

    let evaluation_create = EvaluationBuilder::new(Some(subject.clone()), Some(resource.clone()), Some(action_create.clone()))
        .with_request_id("435".to_string())
        .build();

    // Create the entities
    let mut entity = HashMap::new();
    entity.insert(
        "uid".to_string(),
        serde_json::json!({
            "type": "PharmaAuthZFlow::Platform::BranchInfo",
            "id": "subscription"
        }),
    );
    entity.insert(
        "attrs".to_string(),
        serde_json::json!({
            "active": true
        }),
    );
    entity.insert("parents".to_string(), serde_json::json!([]));

    let entities = vec![Some(entity)];

    // Create a new authorization request
    let request = AzRequestBuilder::new(189106194833, "48335ae72b3b405eae9e4bd5b07732df".to_string())
        .with_request_id(Some("7567".to_string()))
        .with_subject(Some(subject))
        .with_principal(Some(principal))
        .with_entities_map("cedar".to_string(), entities)
        .with_context(Some(context))
        .with_evaluation(evaluation_view)
        .with_evaluation(evaluation_create)
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
}
