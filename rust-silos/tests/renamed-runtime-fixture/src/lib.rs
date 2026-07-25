//! Compile fixture proving that `embed_silo!` supports a renamed runtime dependency.

/// Assets embedded through the fixture's renamed `rust-silos` dependency.
pub static AUTO_RESOLVED_ASSETS: silo_runtime::Silo =
    silo_runtime::embed_silo!("assets", force = true);

/// Assets embedded through the fixture's explicit runtime-path override.
pub static OVERRIDDEN_ASSETS: silo_runtime::Silo =
    silo_runtime::embed_silo!("assets", crate = silo_runtime, force = true);

/// Verifies that macro output using the renamed runtime can access its embedded fixture data.
#[cfg(test)]
#[test]
fn renamed_runtime_embeds_assets() {
    assert!(AUTO_RESOLVED_ASSETS.get_file("fixture.txt").is_some());
    assert!(OVERRIDDEN_ASSETS.get_file("fixture.txt").is_some());
}
