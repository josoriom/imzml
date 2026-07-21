use std::path::Path;

use ionic::ion::IonResult;
use ionic::ion::ScanStream;
use ionic::mzml::structs::{Chromatogram, MzML, Spectrum};

use crate::error::ImzmlError;
use crate::options::{ConversionOptions, ConversionSummary};
use crate::reader::ImzmlReader;
use crate::utilities::{normalize_imzml_file, MemoryLog, TempFile};

pub struct Imzml {
    normalized_file: TempFile,
    reader: ImzmlReader,
}

impl Imzml {
    pub(crate) fn open(
        imzml_path: &Path,
        ibd_path: &Path,
        options: ConversionOptions,
    ) -> Result<Self, ImzmlError> {
        let memory_log = MemoryLog::new(options.log_memory);
        memory_log.write_step("start", 0, 0);

        let normalized_file =
            TempFile::new(imzml_path).map_err(ImzmlError::io("cannot create temporary file"))?;
        normalize_imzml_file(imzml_path, normalized_file.path())
            .map_err(ImzmlError::io("cannot normalize imzML file"))?;
        memory_log.write_step("xml ready", 0, 0);

        let reader = ImzmlReader::open(normalized_file.path(), ibd_path, memory_log)?;
        let mut imzml = Self {
            normalized_file,
            reader,
        };
        imzml.write_memory_now("reader ready");
        Ok(imzml)
    }

    pub fn get_metadata(&mut self) -> Result<MzML, ImzmlError> {
        self.reader.read_metadata()
    }

    pub fn get_next_spectrum(&mut self) -> Result<Option<Spectrum>, ImzmlError> {
        self.reader.read_next_spectrum()
    }

    pub fn get_next_chromatogram(&mut self) -> Result<Option<Chromatogram>, ImzmlError> {
        self.reader.read_next_chromatogram()
    }

    pub fn normalized_path(&self) -> &Path {
        self.normalized_file.path()
    }

    pub(crate) fn summary(&self) -> ConversionSummary {
        self.reader.summary()
    }

    pub(crate) fn write_memory_now(&mut self, step: &str) {
        self.reader.write_memory_now(step);
    }

    pub(crate) fn set_output_size(&self, output_bytes: u64) {
        self.reader.set_output_size(output_bytes);
    }
}

impl ScanStream for Imzml {
    fn metadata(&mut self) -> IonResult<MzML> {
        self.reader.metadata()
    }

    fn next_spectrum(&mut self) -> IonResult<Option<Spectrum>> {
        self.reader.next_spectrum()
    }

    fn next_chromatogram(&mut self) -> IonResult<Option<Chromatogram>> {
        self.reader.next_chromatogram()
    }
}
