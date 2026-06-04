use std::path::PathBuf;

use imzml::ionic_converter::{convert_imzml_to_ion, read_spectrum_from_ion};
use ionic::mzml::structs::BinaryData;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let imzml_path = PathBuf::from("data/200TopL, 170TopR, 190BottomL, 180BottomR-centroid.imzML");
    let ibd_path = PathBuf::from("data/200TopL, 170TopR, 190BottomL, 180BottomR-centroid.ibd");
    let ion_path = PathBuf::from("data/200TopL, 170TopR, 190BottomL, 180BottomR-centroid.ion");

    println!("Converting imzML to ion using parse_mzml...");
    let mzml = convert_imzml_to_ion(&imzml_path, &ibd_path, &ion_path)?;

    let spectra_count = mzml
        .run
        .spectrum_list
        .as_ref()
        .map(|list| list.spectra.len())
        .unwrap_or(0);
    println!("Spectra parsed from imzML: {spectra_count}");

    let ion_size = std::fs::metadata(&ion_path)?.len();
    println!("Ion file size: {:.2} MB", ion_size as f64 / 1024.0 / 1024.0);

    print_first_array_from_imzml(&mzml);
    print_first_array_from_ion(&ion_path)?;

    Ok(())
}

fn print_first_array_from_imzml(mzml: &ionic::mzml::structs::MzML) {
    let Some(spectrum) = mzml.run.spectrum_list.as_ref().and_then(|l| l.spectra.first()) else {
        return;
    };
    let Some(array) = spectrum
        .binary_data_array_list
        .as_ref()
        .and_then(|l| l.binary_data_arrays.first())
    else {
        return;
    };
    println!("\nimzML first m/z values:");
    if let Some(BinaryData::F64(values)) = &array.binary {
        println!("  {:?}", &values[..values.len().min(5)]);
    }
}

fn print_first_array_from_ion(ion_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let Some(spectrum) = read_spectrum_from_ion(ion_path, 0)? else {
        return Ok(());
    };
    let Some(array) = spectrum
        .binary_data_array_list
        .as_ref()
        .and_then(|l| l.binary_data_arrays.first())
    else {
        return Ok(());
    };
    println!("\nion first m/z values:");
    if let Some(BinaryData::F64(values)) = &array.binary {
        println!("  {:?}", &values[..values.len().min(5)]);
    }
    Ok(())
}
