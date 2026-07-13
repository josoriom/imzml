use ionic::ion::encoder::encode::WriteOptions;
use ionic::ion::SectionStorage;

use crate::options::{ConversionOptions, DEFAULT_BLOCK_SIZE, DEFAULT_MZ_WINDOW};

pub(crate) fn get_write_options(options: ConversionOptions) -> WriteOptions {
    WriteOptions {
        compression_level: 18,
        force_f32: false,
        block_size: get_block_size(options),
        parallel: true,
        section_storage: SectionStorage::Disk,
        mz_window: get_mz_window(options),
    }
}

fn get_block_size(options: ConversionOptions) -> usize {
    if options.block_size == 0 {
        DEFAULT_BLOCK_SIZE
    } else {
        options.block_size
    }
}

fn get_mz_window(options: ConversionOptions) -> f64 {
    if options.mz_window.is_finite() && options.mz_window >= 0.0 {
        options.mz_window
    } else {
        DEFAULT_MZ_WINDOW
    }
}
