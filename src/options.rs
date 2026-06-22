pub(crate) const DEFAULT_BLOCK_SIZE: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversionOptions {
    pub log_memory: bool,
    pub block_size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversionSummary {
    pub spectra_count: usize,
    pub chromatogram_count: usize,
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            log_memory: false,
            block_size: DEFAULT_BLOCK_SIZE,
        }
    }
}
