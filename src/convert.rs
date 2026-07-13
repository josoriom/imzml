use std::path::Path;

use ionic::ion::{FileWriter, IonReader, ReadOptions};
use ionic::mzml::structs::Spectrum;
use ionic::IonWriter;

use crate::error::ImzmlError;
use crate::imzml::Imzml;
use crate::options::{ConversionOptions, ConversionSummary};
use crate::utilities::{get_write_options, TempFile};

pub fn convert_imzml_to_ion(
    imzml_path: &Path,
    ibd_path: &Path,
    ion_path: &Path,
) -> Result<ConversionSummary, ImzmlError> {
    convert_imzml_to_ion_with_options(imzml_path, ibd_path, ion_path, ConversionOptions::default())
}

pub fn convert_imzml_to_ion_with_options(
    imzml_path: &Path,
    ibd_path: &Path,
    ion_path: &Path,
    options: ConversionOptions,
) -> Result<ConversionSummary, ImzmlError> {
    let imzml = parse_imzml_with_options(imzml_path, ibd_path, options)?;
    write_ion_file(imzml, ion_path, options)
}

pub fn parse_imzml(imzml_path: &Path, ibd_path: &Path) -> Result<Imzml, ImzmlError> {
    parse_imzml_with_options(imzml_path, ibd_path, ConversionOptions::default())
}

pub fn parse_imzml_with_options(
    imzml_path: &Path,
    ibd_path: &Path,
    options: ConversionOptions,
) -> Result<Imzml, ImzmlError> {
    Imzml::open(imzml_path, ibd_path, options)
}

pub fn write_ion_file(
    mut imzml: Imzml,
    ion_path: &Path,
    options: ConversionOptions,
) -> Result<ConversionSummary, ImzmlError> {
    let temp_output =
        TempFile::new(ion_path).map_err(ImzmlError::io("cannot create temporary file"))?;
    {
        let mut output = FileWriter::open_path(temp_output.path())
            .map_err(ImzmlError::ion("cannot create ion file"))?;
        let mut writer = IonWriter::create(&mut output, get_write_options(options))
            .map_err(ImzmlError::ion("cannot start ion writer"))?;
        writer
            .write_stream(&mut imzml)
            .map_err(ImzmlError::ion("cannot write ion file"))?;
        drop(writer);
        output
            .flush()
            .map_err(ImzmlError::ion("cannot flush ion file"))?;
    }
    imzml.write_memory_now("ion written");
    temp_output
        .move_to(ion_path)
        .map_err(ImzmlError::io("cannot move ion file into place"))?;
    let output_bytes = std::fs::metadata(ion_path)
        .map_err(ImzmlError::io("cannot read ion file size"))?
        .len();
    imzml.set_output_size(output_bytes);
    imzml.write_memory_now("done");
    Ok(imzml.summary())
}

pub fn read_spectrum_from_ion(
    ion_path: &Path,
    index: usize,
) -> Result<Option<Spectrum>, ImzmlError> {
    let mut ion = IonReader::open_file(ion_path, ReadOptions::default())
        .map_err(ImzmlError::ion("cannot open ion file"))?;
    ion.spectrum(index)
        .map_err(ImzmlError::ion("cannot read spectrum"))
}
