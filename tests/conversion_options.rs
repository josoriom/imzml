mod common;

use common::{dataset_two_arrays, write_pair, Workspace};
use imzml::{
    convert_imzml_to_ion, convert_imzml_to_ion_with_options, read_spectrum_from_ion,
    ConversionOptions,
};

fn sample() -> (String, Vec<u8>) {
    dataset_two_arrays(&[
        (vec![100.0, 200.0, 300.0], vec![1.0, 2.0, 3.0]),
        (vec![150.0, 250.0, 350.0], vec![4.0, 5.0, 6.0]),
    ])
}

#[test]
fn zero_block_size_falls_back_to_default() {
    let workspace = Workspace::new("block_zero");
    let (xml, ibd) = sample();
    let (imzml_path, ibd_path) = write_pair(&workspace, &xml, &ibd);
    let ion_path = workspace.path("out.ion");
    let options = ConversionOptions {
        log_memory: false,
        block_size: 0,
    };

    let summary = convert_imzml_to_ion_with_options(&imzml_path, &ibd_path, &ion_path, options)
        .expect("convert");
    assert_eq!(summary.spectra_count, 2);
}

#[test]
fn small_block_size_still_converts() {
    let workspace = Workspace::new("block_small");
    let (xml, ibd) = sample();
    let (imzml_path, ibd_path) = write_pair(&workspace, &xml, &ibd);
    let ion_path = workspace.path("out.ion");
    let options = ConversionOptions {
        log_memory: false,
        block_size: 1024,
    };

    let summary = convert_imzml_to_ion_with_options(&imzml_path, &ibd_path, &ion_path, options)
        .expect("convert");
    assert_eq!(summary.spectra_count, 2);
}

#[test]
fn memory_logging_enabled_still_converts() {
    let workspace = Workspace::new("with_memory_log");
    let (xml, ibd) = sample();
    let (imzml_path, ibd_path) = write_pair(&workspace, &xml, &ibd);
    let ion_path = workspace.path("out.ion");
    let options = ConversionOptions {
        log_memory: true,
        block_size: 0,
    };

    let summary = convert_imzml_to_ion_with_options(&imzml_path, &ibd_path, &ion_path, options)
        .expect("convert");
    assert_eq!(summary.spectra_count, 2);
}

#[test]
fn reading_out_of_range_index_does_not_panic() {
    let workspace = Workspace::new("out_of_range");
    let (xml, ibd) = sample();
    let (imzml_path, ibd_path) = write_pair(&workspace, &xml, &ibd);
    let ion_path = workspace.path("out.ion");
    convert_imzml_to_ion(&imzml_path, &ibd_path, &ion_path).expect("convert");

    let result = read_spectrum_from_ion(&ion_path, 99);
    assert!(matches!(result, Ok(None) | Err(_)));
}
