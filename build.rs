// Copyright (c) 2022 Nitro Agility S.r.l.
// SPDX-License-Identifier: Apache-2.0

//! The gRPC bindings are committed under `src/generated/`, so a project depending on the crate
//! builds without `protoc` (BUG-0283). `PERMGUARD_REGENERATE_PROTO=1 cargo build` regenerates
//! them from `proto/pdp.proto`, which needs `protoc`; CI checks the committed file is current.

const GENERATED: &str = "src/generated";
const FILE: &str = "permguard.data.v1.rs";
const HEADER: &str = "// Copyright (c) 2022 Nitro Agility S.r.l.\n// SPDX-License-Identifier: Apache-2.0\n\n// Generated from proto/pdp.proto by build.rs with PERMGUARD_REGENERATE_PROTO=1: do not edit.\n\n";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-env-changed=PERMGUARD_REGENERATE_PROTO");
    if std::env::var_os("PERMGUARD_REGENERATE_PROTO").is_none() {
        return Ok(());
    }
    println!("cargo:rerun-if-changed=proto/pdp.proto");
    tonic_prost_build::configure()
        .server_mod_attribute(".", "#[cfg(test)]")
        .out_dir(GENERATED)
        .compile_protos(&["proto/pdp.proto"], &["proto"])?;
    let path = std::path::Path::new(GENERATED).join(FILE);
    let generated = std::fs::read_to_string(&path)?;
    std::fs::write(&path, format!("{HEADER}{generated}"))?;
    Ok(())
}
