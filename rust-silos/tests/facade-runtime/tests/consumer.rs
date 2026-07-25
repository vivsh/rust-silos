use facade_runtime::Silo;

const ASSETS: Silo = facade_runtime::embed_silo!("tests/assets", force = true);

/// Verifies that a downstream consumer needs only the facade runtime dependency.
#[test]
fn facade_embeds_assets_without_direct_rust_silos_dependency() {
    assert!(ASSETS.get_file("fixture.txt").is_some());
}
