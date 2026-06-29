use btctax_adapters::price::BundledPrices;
use btctax_adapters::{ingest_files, AdapterError};

#[test]
fn unrecognized_file_is_a_typed_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mystery.csv");
    std::fs::write(&path, "foo,bar\n1,2\n").unwrap();
    let prices = BundledPrices::load().unwrap();
    let err = ingest_files(&[path], &prices).unwrap_err();
    assert!(matches!(err, AdapterError::UnknownSource { .. }));
}
