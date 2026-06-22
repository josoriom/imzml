use std::path::{Path, PathBuf};

use imzml::{parse_imzml_with_options, read_spectrum_from_ion, write_ion_file, ConversionOptions};
use ionic::mzml::structs::NumericArray;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let imzml_path = PathBuf::from("data/200TopL, 170TopR, 190BottomL, 180BottomR-centroid.imzML");
    let ibd_path = PathBuf::from("data/200TopL, 170TopR, 190BottomL, 180BottomR-centroid.ibd");
    let ion_path = PathBuf::from("data/200TopL, 170TopR, 190BottomL, 180BottomR-centroid.ion");
    let log_memory = true;
    let options = ConversionOptions {
        log_memory,
        ..ConversionOptions::default()
    };

    println!("Converting imzML to ion using streaming writer...");
    let imzml = parse_imzml_with_options(&imzml_path, &ibd_path, options)?;
    let summary = write_ion_file(imzml, &ion_path, options)?;

    println!("Spectra streamed from imzML: {}", summary.spectra_count);

    let ion_size = std::fs::metadata(&ion_path)?.len();
    println!("Ion file size: {:.2} MB", ion_size as f64 / 1024.0 / 1024.0);

    print_first_array_from_ion(&ion_path)?;

    Ok(())
}

fn print_first_array_from_ion(ion_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
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
    if let Some(NumericArray::F64(values)) = &array.binary {
        println!("  {:?}", &values[..values.len().min(5)]);
    }
    Ok(())
}
