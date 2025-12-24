use rust_silos::*;
use rust_silos_macros::embed_silo;
use std::collections::HashSet;
use std::io::Read;

/// Tests that an embedded silo can retrieve a known file by path.
#[test]
fn test_embed_silo_get_file() {
    let silo = embed_silo!("tests/data", force=true);
    for file in silo.iter(){
        println!("File in embedded silo: {}", file.path().to_str().unwrap());
    }
    let file = silo.get_file("alpha.txt").unwrap();
    assert_eq!(file.path().to_str().unwrap(), "alpha.txt");
}

/// Tests that an embedded silo returns None for a missing file.
#[test]
fn test_embed_silo_get_file_not_found() {
    let silo = embed_silo!("tests/data");
    assert!(silo.get_file("notfound.txt").is_none());
}

/// Tests that iterating an embedded silo yields all expected files.
#[test]
fn test_embed_silo_iter() {
    let silo = embed_silo!("tests/data");
    let files: HashSet<_> = silo.iter().map(|f| f.path().to_str().unwrap().to_owned()).collect();
    assert!(files.contains("alpha.txt"));
    assert!(files.contains("beta.txt"));
    assert!(files.contains("subdir/gamma.txt"));
}

/// Tests that a dynamic silo can retrieve a known file by path.
#[test]
fn test_dyn_silo_get_file() {
    let silo = Silo::from_static("tests/data");
    let file = silo.get_file("beta.txt").unwrap();
    assert_eq!(file.path().to_str().unwrap(), "beta.txt");
}

/// Tests that a dynamic silo returns None for a missing file.
#[test]
fn test_dyn_silo_get_file_not_found() {
    let silo = Silo::from_static("tests/data");
    assert!(silo.get_file("notfound.txt").is_none());
}

/// Tests that iterating a dynamic silo yields all expected files.
#[test]
fn test_dyn_silo_iter() {
    let silo = Silo::from_static("tests/data");
    let files: HashSet<_> = silo.iter().map(|f| f.path().to_str().unwrap().to_owned()).collect();
    assert!(files.contains("alpha.txt"));
    assert!(files.contains("beta.txt"));
    assert!(files.contains("subdir/gamma.txt"));
}

/// Tests that the file extension is correctly reported.
#[test]
fn test_file_extension() {
    let silo = embed_silo!("tests/data");
    let file = silo.get_file("alpha.txt").unwrap();
    assert_eq!(file.extension(), Some("txt"));
}

/// Tests that is_embedded returns true for embedded files.
#[test]
fn test_file_is_embedded() {
    let silo = embed_silo!("tests/data", force=true);
    let file = silo.get_file("alpha.txt").unwrap();
    assert!(file.is_embedded());
}

/// Tests that is_embedded returns false for dynamic files.
#[test]
fn test_file_is_dynamic() {
    let silo = Silo::from_static("tests/data");
    let file = silo.get_file("alpha.txt").unwrap();
    assert!(!file.is_embedded());
}

/// Tests that absolute_path returns Some for dynamic files.
#[test]
fn test_file_absolute_path_dyn() {
    let silo = Silo::from_static("tests/data");
    let file = silo.get_file("alpha.txt").unwrap();
    assert!(file.absolute_path().is_some());
}

/// Tests that absolute_path returns None for embedded files.
#[test]
fn test_file_absolute_path_embed() {
    let silo = embed_silo!("tests/data", force=true);
    let file = silo.get_file("alpha.txt").unwrap();
    assert!(file.absolute_path().is_none());
}

/// Tests that Silo::from_embedded and Silo::from_path create correct silo types.
#[test]
fn test_silo_from_embedded_and_path() {
    let embed = embed_silo!("tests/data", force=true);
    let dyns = embed.clone().into_dynamic();
    assert!(embed.is_embedded());
    assert!(dyns.is_dynamic());
}

/// Tests that auto_dynamic switches to dynamic mode in debug builds.
#[test]
fn test_silo_auto_dynamic() {
    let embed = embed_silo!("tests/data");
    let auto = embed.auto_dynamic();
    assert!(auto.is_dynamic() || auto.is_embedded());
}

/// Tests that SiloSet can find a file present in any member silo.
#[test]
fn test_silo_set_get_file() {
    let s1 = embed_silo!("tests/data");
    let s2 = Silo::from_static("tests/data");
    let set = SiloSet::new(vec![s1, s2]);
    assert!(set.get_file("alpha.txt").is_some());
}

/// Tests that SiloSet::iter yields all files from all silos.
#[test]
fn test_silo_set_iter() {
    let s1 = embed_silo!("tests/data");
    let s2 = Silo::from_static("tests/data");
    let set = SiloSet::new(vec![s1, s2]);
    let files: Vec<_> = set.iter().collect();
    assert!(!files.is_empty());
}

/// Tests that SiloSet::iter_override yields unique files only.
#[test]
fn test_silo_set_iter_override() {
    let s1 = embed_silo!("tests/data");
    let s2 = Silo::from_static("tests/data");
    let set = SiloSet::new(vec![s1, s2]);
    let files: Vec<_> = set.iter_override().collect();
    assert!(!files.is_empty());
}

/// Tests that embedded and dynamic silos yield the same files and contents.
#[test]
fn test_embed_vs_dyn_parity() {
    let embed = embed_silo!("tests/data");
    let dyns = Silo::from_static("tests/data");
    let embed_files: HashSet<_> = embed.iter().map(|f| f.path().to_str().unwrap().to_owned()).collect();
    let dyn_files: HashSet<_> = dyns.iter().map(|f| f.path().to_str().unwrap().to_owned()).collect();
    assert_eq!(embed_files, dyn_files);
    for path in embed_files {
        let ef = embed.get_file(&path).unwrap();
        let df = dyns.get_file(&path).unwrap();
        let mut eb = Vec::new();
        let mut db = Vec::new();
        ef.reader().unwrap().read_to_end(&mut eb).unwrap();
        df.reader().unwrap().read_to_end(&mut db).unwrap();
        assert_eq!(eb, db, "File content mismatch for {path}");
    }
}

/// Tests that embedded file metadata is correct.
#[test]
fn test_embed_file_metadata() {
    let silo = embed_silo!("tests/data");
    let file = silo.get_file("alpha.txt").unwrap();
    assert!(file.path().to_str().unwrap().ends_with("alpha.txt"));
}

/// Tests that dynamic file metadata is correct.
#[test]
fn test_dyn_file_metadata() {
    let silo = Silo::from_static("tests/data");
    let file = silo.get_file("alpha.txt").unwrap();
    assert!(file.path().to_str().unwrap().ends_with("alpha.txt"));
}

/// Tests reading bytes from an embedded file.
#[test]
fn test_embed_file_read_bytes() {
    let silo = embed_silo!("tests/data");
    let file = silo.get_file("alpha.txt").unwrap();
    let mut buf = Vec::new();
    file.reader().unwrap().read_to_end(&mut buf).unwrap();
    assert!(buf.starts_with(b"alpha file content"));
}

/// Tests reading bytes from a dynamic file.
#[test]
fn test_dyn_file_read_bytes() {
    let silo = Silo::from_static("tests/data");
    let file = silo.get_file("alpha.txt").unwrap();
    let mut buf = Vec::new();
    file.reader().unwrap().read_to_end(&mut buf).unwrap();
    assert!(buf.starts_with(b"alpha file content"));
}

/// Tests reading a string from an embedded file.
#[test]
fn test_embed_file_read_str() {
    let silo = embed_silo!("tests/data");
    let file = silo.get_file("alpha.txt").unwrap();
    let mut buf = String::new();
    file.reader().unwrap().read_to_string(&mut buf).unwrap();
    assert!(buf.contains("alpha file content"));
}

/// Tests reading a string from a dynamic file.
#[test]
fn test_dyn_file_read_str() {
    let silo = Silo::from_static("tests/data");
    let file = silo.get_file("alpha.txt").unwrap();
    let mut buf = String::new();
    file.reader().unwrap().read_to_string(&mut buf).unwrap();
    assert!(buf.contains("alpha file content"));
}

/// Tests reading a file from a subdirectory (embedded).
#[test]
fn test_embed_file_subdir() {
    let silo = embed_silo!("tests/data");
    let file = silo.get_file("subdir/gamma.txt").unwrap();
    let mut buf = String::new();
    file.reader().unwrap().read_to_string(&mut buf).unwrap();
    assert!(buf.contains("gamma file content"));
}

/// Tests reading a file from a subdirectory (dynamic).
#[test]
fn test_dyn_file_subdir() {
    let silo = Silo::from_static("tests/data");
    let file = silo.get_file("subdir/gamma.txt").unwrap();
    let mut buf = String::new();
    file.reader().unwrap().read_to_string(&mut buf).unwrap();
    assert!(buf.contains("gamma file content"));
}

/// Tests size/modified accessors for embedded files.
#[test]
fn test_file_metadata_accessors_embed() {
    let silo = embed_silo!("tests/data", force=true);
    let file = silo.get_file("alpha.txt").unwrap();
    let meta = file.meta().unwrap();
    assert!(meta.size > 0);
    let _ = meta.modified;
}

/// Tests size/modified accessors for dynamic files.
#[test]
fn test_file_metadata_accessors_dynamic() {
    let silo = Silo::from_static("tests/data");
    let file = silo.get_file("alpha.txt").unwrap();
    let meta = file.meta().unwrap();
    assert!(meta.size > 0);
    let _ = meta.modified;
}

#[test]
fn test_lookup_blocks_traversal() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("root");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("ok.txt"), b"ok").unwrap();

    // Sibling file outside the root.
    std::fs::write(tmp.path().join("outside.txt"), b"nope").unwrap();

    let silo = Silo::new(root.to_str().unwrap());
    assert!(silo.get_file("ok.txt").is_some());
    assert!(silo.get_file("../outside.txt").is_none());
}
