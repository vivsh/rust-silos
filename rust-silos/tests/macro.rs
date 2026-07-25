use rust_silos as runtime_alias;
use rust_silos_macros::embed_silo;
use std::io::Read;

/// Verifies that `crate = path` directs generated embedded code through a runtime alias.
#[test]
fn embed_silo_uses_runtime_alias() {
    let silo = embed_silo!("tests/data", crate = runtime_alias, force = true);
    let file = silo.get_file("alpha.txt").expect("fixture should be embedded");
    let mut contents = String::new();

    file.reader()
        .expect("embedded fixture should be readable")
        .read_to_string(&mut contents)
        .expect("embedded fixture should contain UTF-8 text");

    assert!(contents.contains("alpha file content"));
}
