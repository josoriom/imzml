use std::path::PathBuf;

use imzml::{parse_imzml_with_options, write_ion_file, ConversionOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let imzml_path = PathBuf::from("data/GastricMouseTumor+MALDI+FT-ICR.imzML");
    let ibd_path = PathBuf::from("data/GastricMouseTumor+MALDI+FT-ICR.ibd");
    let ion_path = PathBuf::from("data/GastricMouseTumor+MALDI+FT-ICR.ion");
    let log_memory = true;
    let options = ConversionOptions {
        log_memory,
        ..ConversionOptions::default()
    };

    let imzml = parse_imzml_with_options(&imzml_path, &ibd_path, options)?;
    let summary = write_ion_file(imzml, &ion_path, options)?;

    println!("Wrote {ion_path:?} with {} spectra", summary.spectra_count);
    Ok(())
}
