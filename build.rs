// Copyright (c) 2022 Nitro Agility S.r.l.
// SPDX-License-Identifier: Apache-2.0

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/pdp.proto");
    tonic_prost_build::configure()
        .server_mod_attribute(".", "#[cfg(test)]")
        .compile_protos(&["proto/pdp.proto"], &["proto"])?;
    Ok(())
}
