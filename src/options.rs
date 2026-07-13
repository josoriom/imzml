pub(crate) const DEFAULT_BLOCK_SIZE: usize = 1024 * 1024;
pub(crate) const DEFAULT_MZ_WINDOW: f64 = 1000.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConversionOptions {
    pub log_memory: bool,
    pub block_size: usize,
    pub mz_window: f64,
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
            mz_window: DEFAULT_MZ_WINDOW,
        }
    }
}
