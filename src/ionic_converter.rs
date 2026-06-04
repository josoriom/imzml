use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use ionic::ion::encoder::encode::EncodingConfig;
use ionic::ion::encoder::ion_writer::write_mzml_to_ion;
use ionic::ion::encoder::utilities::SectionChunkMode;
use ionic::ion::FileEncoderOutput;
use ionic::mzml::parse_mzml;
use ionic::mzml::structs::{
    BinaryData, BinaryDataArray, CvParam, MzML, NumericType, ReferenceableParamGroup,
};

const FLOAT_64_BIT: &str = "MS:1000523";
const FLOAT_32_BIT: &str = "MS:1000521";
const EXTERNAL_DATA: &str = "IMS:1000101";
const EXTERNAL_OFFSET: &str = "IMS:1000102";
const EXTERNAL_ARRAY_LENGTH: &str = "IMS:1000103";

pub trait BinarySource {
    fn read_bytes(&mut self, offset: u64, count: usize) -> io::Result<Vec<u8>>;
}

pub struct IbdFile {
    file: File,
}

impl IbdFile {
    pub fn open(path: &Path) -> io::Result<Self> {
        Ok(Self {
            file: File::open(path)?,
        })
    }
}

impl BinarySource for IbdFile {
    fn read_bytes(&mut self, offset: u64, count: usize) -> io::Result<Vec<u8>> {
        self.file.seek(SeekFrom::Start(offset))?;
        let mut bytes = vec![0u8; count];
        self.file.read_exact(&mut bytes)?;
        Ok(bytes)
    }
}

pub fn convert_imzml_to_ion(
    imzml_path: &Path,
    ibd_path: &Path,
    ion_path: &Path,
) -> Result<MzML, Box<dyn Error>> {
    let mzml = load_imzml_with_binary(imzml_path, ibd_path)?;
    write_ion_file(&mzml, ion_path)?;
    Ok(mzml)
}

pub fn load_imzml_with_binary(imzml_path: &Path, ibd_path: &Path) -> Result<MzML, Box<dyn Error>> {
    let mut mzml = read_imzml(imzml_path)?;
    let mut source = IbdFile::open(ibd_path)?;
    embed_external_arrays(&mut mzml, &mut source)?;
    Ok(mzml)
}

fn read_imzml(path: &Path) -> Result<MzML, Box<dyn Error>> {
    let raw = std::fs::read(path)?;
    let normalized = normalize_empty_binary_tags(&raw);
    let mzml = parse_mzml(&normalized).map_err(|e| format!("cannot parse imzML: {e:?}"))?;
    Ok(mzml)
}

fn normalize_empty_binary_tags(raw: &[u8]) -> Vec<u8> {
    replace_bytes(raw, b"<binary/>", b"<binary></binary>")
}

fn replace_bytes(source: &[u8], target: &[u8], replacement: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(source.len());
    let mut position = 0;
    while position < source.len() {
        if source[position..].starts_with(target) {
            output.extend_from_slice(replacement);
            position += target.len();
        } else {
            output.push(source[position]);
            position += 1;
        }
    }
    output
}

struct ArrayGroup {
    float_type: NumericType,
    inline_params: Vec<CvParam>,
}

fn embed_external_arrays(mzml: &mut MzML, source: &mut dyn BinarySource) -> Result<(), Box<dyn Error>> {
    let groups = collect_array_groups(mzml);
    let Some(spectrum_list) = mzml.run.spectrum_list.as_mut() else {
        return Ok(());
    };
    for spectrum in &mut spectrum_list.spectra {
        let Some(array_list) = spectrum.binary_data_array_list.as_mut() else {
            continue;
        };
        for array in &mut array_list.binary_data_arrays {
            embed_one_array(array, &groups, source)?;
        }
    }
    Ok(())
}

fn embed_one_array(
    array: &mut BinaryDataArray,
    groups: &HashMap<String, ArrayGroup>,
    source: &mut dyn BinarySource,
) -> Result<(), Box<dyn Error>> {
    let Some(group_id) = first_group_ref(array) else {
        return Ok(());
    };
    let Some(group) = groups.get(&group_id) else {
        return Ok(());
    };
    let Some(offset) = read_cv_usize(array, EXTERNAL_OFFSET) else {
        return Ok(());
    };
    let Some(length) = read_cv_usize(array, EXTERNAL_ARRAY_LENGTH) else {
        return Ok(());
    };

    let bytes = source.read_bytes(offset as u64, length * byte_width(group.float_type))?;
    let data = decode_floats(&bytes, group.float_type);

    array.cv_params = group.inline_params.clone();
    array.numeric_type = Some(group.float_type);
    array.array_length = Some(length);
    array.encoded_length = None;
    array.binary = Some(data);
    Ok(())
}

fn collect_array_groups(mzml: &MzML) -> HashMap<String, ArrayGroup> {
    let mut groups = HashMap::new();
    let Some(group_list) = mzml.referenceable_param_group_list.as_ref() else {
        return groups;
    };
    for group in &group_list.referenceable_param_groups {
        if let Some(float_type) = float_type_of(group) {
            groups.insert(
                group.id.clone(),
                ArrayGroup {
                    float_type,
                    inline_params: inline_params_of(group),
                },
            );
        }
    }
    groups
}

fn float_type_of(group: &ReferenceableParamGroup) -> Option<NumericType> {
    for param in &group.cv_params {
        match param.accession.as_deref() {
            Some(FLOAT_64_BIT) => return Some(NumericType::Float64),
            Some(FLOAT_32_BIT) => return Some(NumericType::Float32),
            _ => {}
        }
    }
    None
}

fn inline_params_of(group: &ReferenceableParamGroup) -> Vec<CvParam> {
    group
        .cv_params
        .iter()
        .filter(|param| param.accession.as_deref() != Some(EXTERNAL_DATA))
        .cloned()
        .collect()
}

fn first_group_ref(array: &BinaryDataArray) -> Option<String> {
    array
        .referenceable_param_group_refs
        .first()
        .map(|reference| reference.r#ref.clone())
}

fn read_cv_usize(array: &BinaryDataArray, accession: &str) -> Option<usize> {
    array
        .cv_params
        .iter()
        .find(|param| param.accession.as_deref() == Some(accession))
        .and_then(|param| param.value.as_deref())
        .and_then(|value| value.parse().ok())
}

fn byte_width(float_type: NumericType) -> usize {
    match float_type {
        NumericType::Float32 => 4,
        _ => 8,
    }
}

fn decode_floats(bytes: &[u8], float_type: NumericType) -> BinaryData {
    match float_type {
        NumericType::Float32 => BinaryData::F32(read_f32_values(bytes)),
        _ => BinaryData::F64(read_f64_values(bytes)),
    }
}

fn read_f64_values(bytes: &[u8]) -> Vec<f64> {
    bytes
        .chunks_exact(8)
        .map(|chunk| f64::from_le_bytes(chunk.try_into().unwrap()))
        .collect()
}

fn read_f32_values(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
        .collect()
}

fn write_ion_file(mzml: &MzML, ion_path: &Path) -> Result<(), Box<dyn Error>> {
    let mut output =
        FileEncoderOutput::open_path(ion_path).map_err(|e| format!("cannot create ion file: {e}"))?;
    write_mzml_to_ion(mzml, encoding_config(), &mut output)
        .map_err(|e| format!("cannot write ion file: {e}"))?;
    output.flush().map_err(|e| format!("cannot flush ion file: {e}"))?;
    Ok(())
}

fn encoding_config() -> EncodingConfig {
    EncodingConfig {
        compression_level: 18,
        force_f32: false,
        uncompressed_block_size: 64 * 1024 * 1024,
        parallel: true,
        section_chunk: SectionChunkMode::Memory,
    }
}

pub fn read_spectrum_from_ion(
    ion_path: &Path,
    index: usize,
) -> Result<Option<ionic::mzml::structs::Spectrum>, Box<dyn Error>> {
    use ionic::ion::{DecoderConfig, Ion};

    let mut ion = Ion::open_file(ion_path, DecoderConfig::default())
        .map_err(|e| format!("cannot open ion file: {e}"))?;
    ion.spectrum_at(index)
        .map_err(|e| format!("cannot read spectrum: {e}").into())
}
