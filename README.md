# imzml

A streaming reader and converter for imzML imaging mass spectrometry files. It reads an `.imzML` metadata file plus its `.ibd` binary file and writes a single `.ion` file, one spectrum at a time, with constant memory.

## Layout

- `src/convert.rs` — public functions: `convert_imzml_to_ion`, `parse_imzml`,
  `write_ion_file`, `read_spectrum_from_ion`.
- `src/imzml.rs` — `Imzml`, the streaming handle.
- `src/reader.rs` — `ImzmlReader`, which streams spectra and fills their arrays.
- `src/error.rs` — `ImzmlError`, the one error type the public API returns.
- `src/options.rs` — `ConversionOptions` and `ConversionSummary`.
- `src/utilities/` — one job per file: decoding, array groups, the `.ibd` byte
  source, the `<binary/>` normalizer, the memory logger, and the `libc` wrapper.

## Dependencies

| Crate         | Third-party | Used for                                   | Wrapper function         |
| ------------- | ----------- | ------------------------------------------ | ------------------------ |
| `ionic`       | no          | mzML parser, ion reader/writer, data types | used directly            |
| `libc`        | yes         | read process memory on macOS               | `read_resident_memory()` |
| `windows-sys` | yes         | read process memory on Windows             | `read_resident_memory()` |
