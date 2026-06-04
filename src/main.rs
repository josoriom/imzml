use std::path::PathBuf;

use imzml::ionic_converter::convert_imzml_to_ion;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let imzml_path = PathBuf::from("data/200TopL, 170TopR, 190BottomL, 180BottomR-centroid.imzML");
    let ibd_path = PathBuf::from("data/200TopL, 170TopR, 190BottomL, 180BottomR-centroid.ibd");
    let ion_path = PathBuf::from("data/200TopL, 170TopR, 190BottomL, 180BottomR-centroid.ion");

    let mzml = convert_imzml_to_ion(&imzml_path, &ibd_path, &ion_path)?;

    let spectra_count = mzml
        .run
        .spectrum_list
        .as_ref()
        .map(|list| list.spectra.len())
        .unwrap_or(0);

    println!("Wrote {ion_path:?} with {spectra_count} spectra");
    Ok(())
}
