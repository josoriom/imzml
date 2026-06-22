use ionic::ion::encoder::encode::{WriteOptions, DEFAULT_MZ_WINDOW};
use ionic::ion::SectionStorage;

use crate::options::{ConversionOptions, DEFAULT_BLOCK_SIZE};

pub(crate) fn get_write_options(options: ConversionOptions) -> WriteOptions {
    WriteOptions {
        compression_level: 18,
        force_f32: false,
        block_size: get_block_size(options),
        parallel: true,
        section_storage: SectionStorage::Disk,
        mz_window: DEFAULT_MZ_WINDOW,
    }
}

fn get_block_size(options: ConversionOptions) -> usize {
    if options.block_size == 0 {
        DEFAULT_BLOCK_SIZE
    } else {
        options.block_size
    }
}
