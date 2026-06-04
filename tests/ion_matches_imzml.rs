use std::path::PathBuf;

use imzml::ionic_converter::{convert_imzml_to_ion, load_imzml_with_binary, read_spectrum_from_ion};
use ionic::mzml::structs::{BinaryData, MzML, Spectrum};

fn imzml_path() -> PathBuf {
    PathBuf::from("data/200TopL, 170TopR, 190BottomL, 180BottomR-centroid.imzML")
}

fn ibd_path() -> PathBuf {
    PathBuf::from("data/200TopL, 170TopR, 190BottomL, 180BottomR-centroid.ibd")
}

fn ion_path() -> PathBuf {
    PathBuf::from("data/test_output.ion")
}

fn array_values(spectrum: &Spectrum, index: usize) -> &BinaryData {
    spectrum
        .binary_data_array_list
        .as_ref()
        .expect("spectrum has binary arrays")
        .binary_data_arrays
        .get(index)
        .expect("array exists")
        .binary
        .as_ref()
        .expect("array has decoded values")
}

fn spectra_of(mzml: &MzML) -> &[Spectrum] {
    mzml.run
        .spectrum_list
        .as_ref()
        .map(|list| list.spectra.as_slice())
        .unwrap_or(&[])
}

#[test]
fn ion_file_matches_imzml_spectra() {
    let source = load_imzml_with_binary(&imzml_path(), &ibd_path()).expect("load imzML");
    convert_imzml_to_ion(&imzml_path(), &ibd_path(), &ion_path()).expect("write ion");

    let spectra = spectra_of(&source);
    assert_eq!(spectra.len(), 16632);

    for index in sample_indexes(spectra.len()) {
        let expected = &spectra[index];
        let actual = read_spectrum_from_ion(&ion_path(), index)
            .expect("read ion spectrum")
            .expect("spectrum exists");

        assert_eq!(expected.id, actual.id, "id mismatch at {index}");
        assert_arrays_match(array_values(expected, 0), array_values(&actual, 0));
        assert_arrays_match(array_values(expected, 1), array_values(&actual, 1));
    }
}

fn sample_indexes(count: usize) -> Vec<usize> {
    vec![0, 1, count / 4, count / 2, count - 2, count - 1]
}

fn assert_arrays_match(expected: &BinaryData, actual: &BinaryData) {
    match (expected, actual) {
        (BinaryData::F64(left), BinaryData::F64(right)) => assert_eq!(left, right),
        (BinaryData::F32(left), BinaryData::F32(right)) => assert_eq!(left, right),
        _ => panic!("binary data type mismatch"),
    }
}
