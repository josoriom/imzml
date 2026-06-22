mod common;

use common::{decoded, single_array_dataset, stream_all, write_pair, Array, Workspace};
use imzml::NumericArray;

fn stream_one(tag: &str, values: Array) -> NumericArray {
    let workspace = Workspace::new(tag);
    let (xml, ibd) = single_array_dataset(&values);
    let (imzml_path, ibd_path) = write_pair(&workspace, &xml, &ibd);
    let spectra = stream_all(&imzml_path, &ibd_path).expect("stream");
    assert_eq!(spectra.len(), 1);
    decoded(&spectra[0], 0).clone()
}

#[test]
fn decodes_float64_array() {
    match stream_one("f64", Array::F64(vec![0.0, -1.5, 123.4567890123, 1e300])) {
        NumericArray::F64(values) => {
            assert_eq!(values, vec![0.0, -1.5, 123.4567890123, 1e300])
        }
        other => panic!("expected f64, got {other:?}"),
    }
}

#[test]
fn decodes_float32_array() {
    match stream_one("f32", Array::F32(vec![0.0, -1.5, 2.5, 100.25])) {
        NumericArray::F32(values) => assert_eq!(values, vec![0.0, -1.5, 2.5, 100.25]),
        other => panic!("expected f32, got {other:?}"),
    }
}

#[test]
fn decodes_int32_array_with_negatives() {
    match stream_one("i32", Array::I32(vec![0, -1, i32::MIN, i32::MAX, 42])) {
        NumericArray::I32(values) => assert_eq!(values, vec![0, -1, i32::MIN, i32::MAX, 42]),
        other => panic!("expected i32, got {other:?}"),
    }
}

#[test]
fn decodes_int64_array_with_large_values() {
    match stream_one("i64", Array::I64(vec![0, -1, i64::MIN, i64::MAX])) {
        NumericArray::I64(values) => assert_eq!(values, vec![0, -1, i64::MIN, i64::MAX]),
        other => panic!("expected i64, got {other:?}"),
    }
}

#[test]
fn decodes_empty_array() {
    match stream_one("empty", Array::F64(vec![])) {
        NumericArray::F64(values) => assert!(values.is_empty()),
        other => panic!("expected empty f64, got {other:?}"),
    }
}
