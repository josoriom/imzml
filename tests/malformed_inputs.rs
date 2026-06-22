mod common;

use common::{
    binary_array_xml, group_xml, spectrum_error_source, spectrum_xml, stream_all, wrap_mzml,
    write_pair, Workspace,
};
use imzml::{parse_imzml, ImzmlError};

#[test]
fn missing_imzml_file_returns_error() {
    let workspace = Workspace::new("missing_imzml");
    let ibd_path = workspace.write("sample.ibd", &[0u8; 16]);
    let imzml_path = workspace.path("does_not_exist.imzML");

    let result = parse_imzml(&imzml_path, &ibd_path);
    assert!(result.is_err());
}

#[test]
fn missing_ibd_file_returns_error() {
    let workspace = Workspace::new("missing_ibd");
    let groups = group_xml("mzArray", "MS:1000514", "m/z array", "MS:1000523", "64-bit float");
    let xml = wrap_mzml(&groups, "", 0);
    let imzml_path = workspace.write("sample.imzML", xml.as_bytes());
    let ibd_path = workspace.path("does_not_exist.ibd");

    let result = parse_imzml(&imzml_path, &ibd_path);
    assert!(matches!(result, Err(ImzmlError::Io { .. })));
}

#[test]
fn garbage_input_does_not_panic_and_yields_no_spectra() {
    let workspace = Workspace::new("garbage");
    let (imzml_path, ibd_path) =
        write_pair(&workspace, "this is not xml at all <<< >>>", &[0u8; 16]);

    let spectra = stream_all(&imzml_path, &ibd_path).unwrap_or_default();
    assert!(spectra.is_empty());
}

#[test]
fn empty_imzml_file_does_not_panic() {
    let workspace = Workspace::new("empty_file");
    let (imzml_path, ibd_path) = write_pair(&workspace, "", &[0u8; 16]);

    let spectra = stream_all(&imzml_path, &ibd_path).unwrap_or_default();
    assert!(spectra.is_empty());
}

#[test]
fn unknown_data_type_is_rejected() {
    let workspace = Workspace::new("unknown_type");
    let groups = "    <referenceableParamGroup id=\"arr0\">\n\
                  \x20     <cvParam cvRef=\"IMS\" accession=\"IMS:1000101\" name=\"external data\" value=\"true\"/>\n\
                  \x20   </referenceableParamGroup>\n";
    let array = binary_array_xml("arr0", 16, 1, 8);
    let spectra = spectrum_xml(0, &[array]);
    let xml = wrap_mzml(groups, &spectra, 1);
    let (imzml_path, ibd_path) = write_pair(&workspace, &xml, &[0u8; 24]);

    let mut reader = parse_imzml(&imzml_path, &ibd_path).expect("open");
    let error = reader.get_next_spectrum().expect_err("should reject unknown type");
    assert!(matches!(
        spectrum_error_source(error),
        ImzmlError::UnknownDataType { .. }
    ));
}

#[test]
fn missing_array_length_is_rejected() {
    let workspace = Workspace::new("missing_length");
    let groups = group_xml("arr0", "MS:1000515", "intensity array", "MS:1000523", "64-bit float");
    let array = "          <binaryDataArray encodedLength=\"0\">\n\
                 \x20           <referenceableParamGroupRef ref=\"arr0\"/>\n\
                 \x20           <cvParam cvRef=\"IMS\" accession=\"IMS:1000102\" name=\"external offset\" value=\"16\"/>\n\
                 \x20           <binary/>\n\
                 \x20         </binaryDataArray>\n"
        .to_string();
    let spectra = spectrum_xml(0, &[array]);
    let xml = wrap_mzml(&groups, &spectra, 1);
    let (imzml_path, ibd_path) = write_pair(&workspace, &xml, &[0u8; 24]);

    let mut reader = parse_imzml(&imzml_path, &ibd_path).expect("open");
    let error = reader.get_next_spectrum().expect_err("should reject missing length");
    assert!(matches!(
        spectrum_error_source(error),
        ImzmlError::MissingArrayLength
    ));
}
