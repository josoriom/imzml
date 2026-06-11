use std::path::PathBuf;

use imzml::{convert_imzml_to_ion, parse_imzml, read_spectrum_from_ion};
use ionic::mzml::structs::{BinaryData, Spectrum};

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

#[test]
fn ion_file_matches_imzml_spectra() {
    let expected_spectra = read_sample_spectra().expect("read imzML");
    let summary = convert_imzml_to_ion(&imzml_path(), &ibd_path(), &ion_path()).expect("write ion");

    assert_eq!(summary.spectra_count, 16632);

    for (index, expected) in expected_spectra {
        let actual = read_spectrum_from_ion(&ion_path(), index)
            .expect("read ion spectrum")
            .expect("spectrum exists");

        assert_eq!(expected.id, actual.id, "id mismatch at {index}");
        assert_arrays_match(array_values(&expected, 0), array_values(&actual, 0));
        assert_arrays_match(array_values(&expected, 1), array_values(&actual, 1));
    }
}

fn read_sample_spectra() -> Result<Vec<(usize, Spectrum)>, Box<dyn std::error::Error>> {
    let mut imzml = parse_imzml(&imzml_path(), &ibd_path())?;
    let metadata = imzml.get_metadata()?;
    let spectra_count = metadata
        .run
        .spectrum_list
        .as_ref()
        .and_then(|list| list.count)
        .unwrap_or(0);
    assert_eq!(spectra_count, 16632);
    let indexes = sample_indexes(spectra_count);
    let mut spectra = Vec::new();

    for index in 0..spectra_count {
        let Some(spectrum) = imzml.get_next_spectrum()? else {
            break;
        };
        if indexes.contains(&index) {
            spectra.push((index, spectrum));
        }
    }

    Ok(spectra)
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
