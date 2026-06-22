mod common;

use common::{dataset_two_arrays, decoded, stream_all, write_pair, Workspace};
use imzml::{convert_imzml_to_ion, read_spectrum_from_ion, NumericArray};

#[test]
fn streams_two_arrays_and_decodes_values() {
    let workspace = Workspace::new("two_arrays");
    let (xml, ibd) = dataset_two_arrays(&[
        (vec![100.0, 200.5, 300.25], vec![1.0, 2.0, 3.0]),
        (vec![110.0, 220.0], vec![9.0, 8.0]),
    ]);
    let (imzml_path, ibd_path) = write_pair(&workspace, &xml, &ibd);

    let spectra = stream_all(&imzml_path, &ibd_path).expect("stream spectra");

    assert_eq!(spectra.len(), 2);
    match decoded(&spectra[0], 0) {
        NumericArray::F64(values) => assert_eq!(values, &vec![100.0, 200.5, 300.25]),
        other => panic!("expected f64 m/z, got {other:?}"),
    }
    match decoded(&spectra[0], 1) {
        NumericArray::F32(values) => assert_eq!(values, &vec![1.0, 2.0, 3.0]),
        other => panic!("expected f32 intensity, got {other:?}"),
    }
}

#[test]
fn converts_to_ion_and_reads_back_matching_values() {
    let workspace = Workspace::new("roundtrip");
    let (xml, ibd) = dataset_two_arrays(&[
        (vec![100.0, 200.0, 300.0], vec![5.0, 6.0, 7.0]),
        (vec![150.0, 250.0, 350.0], vec![1.5, 2.5, 3.5]),
    ]);
    let (imzml_path, ibd_path) = write_pair(&workspace, &xml, &ibd);
    let ion_path = workspace.path("out.ion");

    let summary = convert_imzml_to_ion(&imzml_path, &ibd_path, &ion_path).expect("convert");
    assert_eq!(summary.spectra_count, 2);

    let spectrum = read_spectrum_from_ion(&ion_path, 1)
        .expect("read ion")
        .expect("spectrum exists");
    match decoded(&spectrum, 0) {
        NumericArray::F64(values) => assert_eq!(values, &vec![150.0, 250.0, 350.0]),
        other => panic!("expected f64 m/z, got {other:?}"),
    }
}
