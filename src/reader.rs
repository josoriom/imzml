use std::collections::HashMap;
use std::path::Path;

use ionic::ion::ScanStream;
use ionic::ion::{IonError, IonResult};
use ionic::mzml::structs::{Chromatogram, MzML, Spectrum};
use ionic::mzml::MzmlReader;

use crate::error::ImzmlError;
use crate::options::ConversionSummary;
use crate::utilities::{collect_array_groups, fill_array, ArrayGroup, IbdFile, MemoryLog};

pub(crate) struct ImzmlReader {
    mzml: MzmlReader,
    source: IbdFile,
    groups: HashMap<String, ArrayGroup>,
    spectra_count: usize,
    chromatogram_count: usize,
    memory_log: MemoryLog,
}

impl ImzmlReader {
    pub(crate) fn open(
        imzml_path: &Path,
        ibd_path: &Path,
        memory_log: MemoryLog,
    ) -> Result<Self, ImzmlError> {
        let mut mzml =
            MzmlReader::open(imzml_path).map_err(ImzmlError::ion("cannot open imzML metadata"))?;
        let metadata = mzml
            .metadata()
            .map_err(ImzmlError::ion("cannot read imzML metadata"))?;
        let total_spectra = metadata
            .run
            .spectrum_list
            .as_ref()
            .and_then(|spectrum_list| spectrum_list.count)
            .unwrap_or(0);
        memory_log.set_total_spectra(total_spectra);
        let groups = collect_array_groups(&metadata);
        let source = IbdFile::open(ibd_path).map_err(ImzmlError::io("cannot open ibd file"))?;
        memory_log.set_input_size(source.byte_count());
        Ok(Self {
            mzml,
            source,
            groups,
            spectra_count: 0,
            chromatogram_count: 0,
            memory_log,
        })
    }

    pub(crate) fn read_metadata(&mut self) -> Result<MzML, ImzmlError> {
        self.mzml
            .metadata()
            .map_err(ImzmlError::ion("cannot read imzML metadata"))
    }

    pub(crate) fn read_next_spectrum(&mut self) -> Result<Option<Spectrum>, ImzmlError> {
        let Some(mut spectrum) = self
            .mzml
            .next_spectrum()
            .map_err(ImzmlError::ion("cannot read imzML spectrum"))?
        else {
            return Ok(None);
        };
        self.fill_spectrum_arrays(&mut spectrum)?;
        self.spectra_count += 1;
        self.set_memory_step("streaming spectra");
        self.set_memory_counts();
        Ok(Some(spectrum))
    }

    pub(crate) fn read_next_chromatogram(&mut self) -> Result<Option<Chromatogram>, ImzmlError> {
        let Some(mut chromatogram) = self
            .mzml
            .next_chromatogram()
            .map_err(ImzmlError::ion("cannot read imzML chromatogram"))?
        else {
            return Ok(None);
        };
        self.fill_chromatogram_arrays(&mut chromatogram)?;
        self.chromatogram_count += 1;
        self.set_memory_step("streaming chromatograms");
        self.set_memory_counts();
        Ok(Some(chromatogram))
    }

    pub(crate) fn summary(&self) -> ConversionSummary {
        ConversionSummary {
            spectra_count: self.spectra_count,
            chromatogram_count: self.chromatogram_count,
        }
    }

    pub(crate) fn write_memory_now(&mut self, step: &str) {
        self.memory_log
            .write_step(step, self.spectra_count, self.chromatogram_count);
    }

    pub(crate) fn set_output_size(&self, output_bytes: u64) {
        self.memory_log.set_output_size(output_bytes);
    }

    fn set_memory_step(&self, step: &str) {
        self.memory_log.set_step(step);
    }

    fn set_memory_counts(&self) {
        self.memory_log
            .set_counts(self.spectra_count, self.chromatogram_count);
    }

    fn fill_spectrum_arrays(&mut self, spectrum: &mut Spectrum) -> Result<(), ImzmlError> {
        let Some(array_list) = spectrum.binary_data_array_list.as_mut() else {
            return Ok(());
        };
        for array in &mut array_list.binary_data_arrays {
            fill_array(array, &self.groups, &mut self.source).map_err(|source| {
                ImzmlError::Spectrum {
                    index: spectrum.index.unwrap_or_default(),
                    id: spectrum.id.clone(),
                    source: Box::new(source),
                }
            })?;
        }
        Ok(())
    }

    fn fill_chromatogram_arrays(
        &mut self,
        chromatogram: &mut Chromatogram,
    ) -> Result<(), ImzmlError> {
        let Some(array_list) = chromatogram.binary_data_array_list.as_mut() else {
            return Ok(());
        };
        for array in &mut array_list.binary_data_arrays {
            fill_array(array, &self.groups, &mut self.source).map_err(|source| {
                ImzmlError::Chromatogram {
                    index: chromatogram.index.unwrap_or_default(),
                    id: chromatogram.id.clone(),
                    source: Box::new(source),
                }
            })?;
        }
        Ok(())
    }
}

fn to_ion_error(error: ImzmlError) -> IonError {
    IonError::from(error.to_string())
}

impl ScanStream for ImzmlReader {
    fn metadata(&mut self) -> IonResult<MzML> {
        self.read_metadata().map_err(to_ion_error)
    }

    fn next_spectrum(&mut self) -> IonResult<Option<Spectrum>> {
        self.read_next_spectrum().map_err(to_ion_error)
    }

    fn next_chromatogram(&mut self) -> IonResult<Option<Chromatogram>> {
        self.read_next_chromatogram().map_err(to_ion_error)
    }
}
