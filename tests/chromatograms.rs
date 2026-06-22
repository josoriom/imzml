mod common;

use common::{
    binary_array_xml, chromatogram_array, chromatogram_error_source, chromatogram_xml,
    dataset_with_chromatograms, stream_all_chromatograms, wrap_mzml_with_chromatograms, write_pair,
    Workspace,
};
use imzml::{convert_imzml_to_ion, parse_imzml, NumericArray, ImzmlError};

#[test]
fn streams_chromatogram_after_spectra_and_decodes_values() {
    let workspace = Workspace::new("chrom_after_spectra");
    let (xml, ibd) = dataset_with_chromatograms(
        &[(vec![100.0, 200.0], vec![1.0, 2.0])],
        &[(vec![0.0, 0.5, 1.0], vec![10.0, 20.0, 30.0])],
    );
    let (imzml_path, ibd_path) = write_pair(&workspace, &xml, &ibd);

    let chromatograms = stream_all_chromatograms(&imzml_path, &ibd_path).expect("stream");
    assert_eq!(chromatograms.len(), 1);
    match chromatogram_array(&chromatograms[0], 0) {
        NumericArray::F64(time) => assert_eq!(time, &vec![0.0, 0.5, 1.0]),
        other => panic!("expected f64 time, got {other:?}"),
    }
    match chromatogram_array(&chromatograms[0], 1) {
        NumericArray::F32(intensity) => assert_eq!(intensity, &vec![10.0, 20.0, 30.0]),
        other => panic!("expected f32 intensity, got {other:?}"),
    }
}

#[test]
fn streams_chromatogram_with_no_spectra() {
    let workspace = Workspace::new("chrom_only");
    let (xml, ibd) = dataset_with_chromatograms(&[], &[(vec![0.0, 1.0], vec![5.0, 6.0])]);
    let (imzml_path, ibd_path) = write_pair(&workspace, &xml, &ibd);

    let chromatograms = stream_all_chromatograms(&imzml_path, &ibd_path).expect("stream");
    assert_eq!(chromatograms.len(), 1);
    match chromatogram_array(&chromatograms[0], 1) {
        NumericArray::F32(intensity) => assert_eq!(intensity, &vec![5.0, 6.0]),
        other => panic!("expected f32 intensity, got {other:?}"),
    }
}

#[test]
fn convert_counts_spectra_and_chromatograms() {
    let workspace = Workspace::new("chrom_convert");
    let (xml, ibd) = dataset_with_chromatograms(
        &[(vec![100.0, 200.0, 300.0], vec![1.0, 2.0, 3.0])],
        &[(vec![0.0, 1.0, 2.0], vec![7.0, 8.0, 9.0])],
    );
    let (imzml_path, ibd_path) = write_pair(&workspace, &xml, &ibd);
    let ion_path = workspace.path("out.ion");

    let summary = convert_imzml_to_ion(&imzml_path, &ibd_path, &ion_path).expect("convert");
    assert_eq!(summary.spectra_count, 1);
    assert_eq!(summary.chromatogram_count, 1);
}

#[test]
fn malformed_chromatogram_array_is_reported() {
    let workspace = Workspace::new("chrom_bad");
    let groups = "    <referenceableParamGroup id=\"badArr\">\n\
                  \x20     <cvParam cvRef=\"IMS\" accession=\"IMS:1000101\" name=\"external data\" value=\"true\"/>\n\
                  \x20   </referenceableParamGroup>\n";
    let array = binary_array_xml("badArr", 16, 1, 8);
    let chromatogram = chromatogram_xml(0, &[array]);
    let xml = wrap_mzml_with_chromatograms(groups, "", 0, &chromatogram, 1);
    let (imzml_path, ibd_path) = write_pair(&workspace, &xml, &[0u8; 24]);

    let mut reader = parse_imzml(&imzml_path, &ibd_path).expect("open");
    while reader.get_next_spectrum().expect("spectrum").is_some() {}
    let error = reader.get_next_chromatogram().expect_err("should reject");
    assert!(matches!(
        chromatogram_error_source(error),
        ImzmlError::UnknownDataType { .. }
    ));
}
