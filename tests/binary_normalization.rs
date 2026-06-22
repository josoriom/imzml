mod common;

use common::{dataset_two_arrays, decoded, stream_all, write_pair, Workspace};
use imzml::NumericArray;

#[test]
fn already_normalized_binary_tags_still_decode() {
    let workspace = Workspace::new("explicit_binary");
    let (xml, ibd) = dataset_two_arrays(&[(vec![100.0, 200.0], vec![1.0, 2.0])]);
    let explicit = xml.replace("<binary/>", "<binary></binary>");
    assert!(!explicit.contains("<binary/>"));
    let (imzml_path, ibd_path) = write_pair(&workspace, &explicit, &ibd);

    let spectra = stream_all(&imzml_path, &ibd_path).expect("stream");
    assert_eq!(spectra.len(), 1);
    match decoded(&spectra[0], 0) {
        NumericArray::F64(values) => assert_eq!(values, &vec![100.0, 200.0]),
        other => panic!("expected f64, got {other:?}"),
    }
}

#[test]
fn self_closing_binary_tags_decode() {
    let workspace = Workspace::new("self_closing_binary");
    let (xml, ibd) = dataset_two_arrays(&[(vec![100.0, 200.0], vec![1.0, 2.0])]);
    assert!(xml.contains("<binary/>"));
    let (imzml_path, ibd_path) = write_pair(&workspace, &xml, &ibd);

    let spectra = stream_all(&imzml_path, &ibd_path).expect("stream");
    match decoded(&spectra[0], 1) {
        NumericArray::F32(values) => assert_eq!(values, &vec![1.0, 2.0]),
        other => panic!("expected f32, got {other:?}"),
    }
}
