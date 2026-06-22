mod common;

use common::{
    binary_array_xml, group_xml, spectrum_error_source, spectrum_xml, wrap_mzml, write_pair,
    Workspace,
};
use imzml::{parse_imzml, NumericArray, ImzmlError};

fn float_group() -> String {
    group_xml("arr0", "MS:1000515", "intensity array", "MS:1000523", "64-bit float")
}

fn first_spectrum_error(tag: &str, array: String, ibd: &[u8]) -> ImzmlError {
    let workspace = Workspace::new(tag);
    let spectra = spectrum_xml(0, &[array]);
    let xml = wrap_mzml(&float_group(), &spectra, 1);
    let (imzml_path, ibd_path) = write_pair(&workspace, &xml, ibd);
    let mut reader = parse_imzml(&imzml_path, &ibd_path).expect("open");
    let error = reader.get_next_spectrum().expect_err("should fail");
    spectrum_error_source(error)
}

#[test]
fn huge_array_length_is_rejected_without_allocating() {
    let array = binary_array_xml("arr0", 16, 100_000_000_000, 8);
    let error = first_spectrum_error("huge_length", array, &[0u8; 24]);
    assert!(matches!(error, ImzmlError::Io { .. }));
}

#[test]
fn array_length_byte_count_overflow_is_rejected() {
    let array = binary_array_xml("arr0", 16, usize::MAX, 8);
    let error = first_spectrum_error("length_overflow", array, &[0u8; 24]);
    assert!(matches!(error, ImzmlError::ByteCountOverflow));
}

#[test]
fn offset_overflow_is_rejected() {
    let array = binary_array_xml("arr0", usize::MAX, 1, 8);
    let error = first_spectrum_error("offset_overflow", array, &[0u8; 24]);
    assert!(matches!(error, ImzmlError::Io { .. }));
}

#[test]
fn ibd_too_small_for_array_is_rejected() {
    let array = binary_array_xml("arr0", 16, 10, 80);
    let error = first_spectrum_error("ibd_too_small", array, &[0u8; 56]);
    assert!(matches!(error, ImzmlError::Io { .. }));
}

#[test]
fn many_spectra_stream_from_a_tiny_ibd() {
    let workspace = Workspace::new("many_spectra");
    let groups = format!(
        "{}{}",
        group_xml("mzArray", "MS:1000514", "m/z array", "MS:1000523", "64-bit float"),
        group_xml(
            "intensityArray",
            "MS:1000515",
            "intensity array",
            "MS:1000521",
            "32-bit float"
        ),
    );

    let mut ibd = vec![0u8; 16];
    ibd.extend([100.0f64, 200.0, 300.0].iter().flat_map(|v| v.to_le_bytes()));
    ibd.extend([1.0f32, 2.0, 3.0].iter().flat_map(|v| v.to_le_bytes()));
    assert_eq!(ibd.len(), 52);

    let spectrum_count = 5000;
    let mut spectra_xml = String::new();
    for index in 0..spectrum_count {
        let arrays = [
            binary_array_xml("mzArray", 16, 3, 24),
            binary_array_xml("intensityArray", 40, 3, 12),
        ];
        spectra_xml.push_str(&spectrum_xml(index, &arrays));
    }
    let xml = wrap_mzml(&groups, &spectra_xml, spectrum_count);
    let (imzml_path, ibd_path) = write_pair(&workspace, &xml, &ibd);

    let mut reader = parse_imzml(&imzml_path, &ibd_path).expect("open");
    let mut seen = 0;
    let mut first_values = None;
    while let Some(spectrum) = reader.get_next_spectrum().expect("spectrum") {
        if seen == 0 {
            if let NumericArray::F64(values) = common::decoded(&spectrum, 0) {
                first_values = Some(values.clone());
            }
        }
        seen += 1;
    }

    assert_eq!(seen, spectrum_count);
    assert_eq!(first_values, Some(vec![100.0, 200.0, 300.0]));
    assert_eq!(std::fs::metadata(&ibd_path).unwrap().len(), 52);
}
