mod common;

use common::{dataset_two_arrays, group_xml, spectrum_xml, wrap_mzml, write_pair, Workspace};
use imzml::parse_imzml;

#[test]
fn metadata_reports_spectrum_count_before_streaming() {
    let workspace = Workspace::new("meta_count");
    let (xml, ibd) = dataset_two_arrays(&[
        (vec![100.0], vec![1.0]),
        (vec![200.0], vec![2.0]),
        (vec![300.0], vec![3.0]),
    ]);
    let (imzml_path, ibd_path) = write_pair(&workspace, &xml, &ibd);

    let mut reader = parse_imzml(&imzml_path, &ibd_path).expect("open");
    let metadata = reader.get_metadata().expect("metadata");
    let count = metadata
        .run
        .spectrum_list
        .as_ref()
        .and_then(|list| list.count)
        .unwrap_or(0);
    assert_eq!(count, 3);
}

#[test]
fn streaming_stops_with_none_after_last_spectrum() {
    let workspace = Workspace::new("exhaust");
    let (xml, ibd) = dataset_two_arrays(&[(vec![100.0], vec![1.0]), (vec![200.0], vec![2.0])]);
    let (imzml_path, ibd_path) = write_pair(&workspace, &xml, &ibd);

    let mut reader = parse_imzml(&imzml_path, &ibd_path).expect("open");
    assert!(reader.get_next_spectrum().expect("first").is_some());
    assert!(reader.get_next_spectrum().expect("second").is_some());
    assert!(reader.get_next_spectrum().expect("third").is_none());
    assert!(reader.get_next_spectrum().expect("fourth").is_none());
}

#[test]
fn empty_spectrum_list_streams_nothing() {
    let workspace = Workspace::new("zero");
    let groups = group_xml("mzArray", "MS:1000514", "m/z array", "MS:1000523", "64-bit float");
    let xml = wrap_mzml(&groups, "", 0);
    let (imzml_path, ibd_path) = write_pair(&workspace, &xml, &[0u8; 16]);

    let mut reader = parse_imzml(&imzml_path, &ibd_path).expect("open");
    assert!(reader.get_next_spectrum().expect("none").is_none());
}

#[test]
fn array_without_external_offset_is_left_unfilled() {
    let workspace = Workspace::new("no_offset");
    let groups = group_xml("arr0", "MS:1000515", "intensity array", "MS:1000523", "64-bit float");
    let array = "          <binaryDataArray encodedLength=\"0\">\n\
                 \x20           <referenceableParamGroupRef ref=\"arr0\"/>\n\
                 \x20           <binary/>\n\
                 \x20         </binaryDataArray>\n"
        .to_string();
    let spectra = spectrum_xml(0, &[array]);
    let xml = wrap_mzml(&groups, &spectra, 1);
    let (imzml_path, ibd_path) = write_pair(&workspace, &xml, &[0u8; 16]);

    let mut reader = parse_imzml(&imzml_path, &ibd_path).expect("open");
    let spectrum = reader.get_next_spectrum().expect("spectrum").expect("present");
    let array = &spectrum
        .binary_data_array_list
        .as_ref()
        .expect("array list")
        .binary_data_arrays[0];
    assert!(array.binary.is_none());
}
