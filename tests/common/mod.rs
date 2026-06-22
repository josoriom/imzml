#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use imzml::{parse_imzml, NumericArray, Chromatogram, ImzmlError, Spectrum};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

pub struct Workspace {
    dir: PathBuf,
}

impl Workspace {
    pub fn new(tag: &str) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("imzml_test_{tag}_{}_{id}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        Self { dir }
    }

    pub fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.dir.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[derive(Clone)]
pub enum Array {
    F64(Vec<f64>),
    F32(Vec<f32>),
    I32(Vec<i32>),
    I64(Vec<i64>),
}

impl Array {
    pub fn to_le_bytes(&self) -> Vec<u8> {
        match self {
            Array::F64(values) => values.iter().flat_map(|v| v.to_le_bytes()).collect(),
            Array::F32(values) => values.iter().flat_map(|v| v.to_le_bytes()).collect(),
            Array::I32(values) => values.iter().flat_map(|v| v.to_le_bytes()).collect(),
            Array::I64(values) => values.iter().flat_map(|v| v.to_le_bytes()).collect(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Array::F64(values) => values.len(),
            Array::F32(values) => values.len(),
            Array::I32(values) => values.len(),
            Array::I64(values) => values.len(),
        }
    }

    pub fn type_accession(&self) -> (&'static str, &'static str) {
        match self {
            Array::F64(_) => ("MS:1000523", "64-bit float"),
            Array::F32(_) => ("MS:1000521", "32-bit float"),
            Array::I32(_) => ("MS:1000519", "32-bit integer"),
            Array::I64(_) => ("MS:1000522", "64-bit integer"),
        }
    }
}

pub fn group_xml(
    id: &str,
    array_accession: &str,
    array_name: &str,
    type_accession: &str,
    type_name: &str,
) -> String {
    format!(
        "    <referenceableParamGroup id=\"{id}\">\n\
         \x20     <cvParam cvRef=\"MS\" accession=\"MS:1000576\" name=\"no compression\" value=\"\"/>\n\
         \x20     <cvParam cvRef=\"MS\" accession=\"{array_accession}\" name=\"{array_name}\" value=\"\"/>\n\
         \x20     <cvParam cvRef=\"IMS\" accession=\"IMS:1000101\" name=\"external data\" value=\"true\"/>\n\
         \x20     <cvParam cvRef=\"MS\" accession=\"{type_accession}\" name=\"{type_name}\" value=\"\"/>\n\
         \x20   </referenceableParamGroup>\n"
    )
}

pub fn binary_array_xml(group_ref: &str, offset: usize, length: usize, encoded_length: usize) -> String {
    format!(
        "          <binaryDataArray encodedLength=\"0\">\n\
         \x20           <referenceableParamGroupRef ref=\"{group_ref}\"/>\n\
         \x20           <cvParam cvRef=\"IMS\" accession=\"IMS:1000103\" name=\"external array length\" value=\"{length}\"/>\n\
         \x20           <cvParam cvRef=\"IMS\" accession=\"IMS:1000102\" name=\"external offset\" value=\"{offset}\"/>\n\
         \x20           <cvParam cvRef=\"IMS\" accession=\"IMS:1000104\" name=\"external encoded length\" value=\"{encoded_length}\"/>\n\
         \x20           <binary/>\n\
         \x20         </binaryDataArray>\n"
    )
}

pub fn spectrum_xml(index: usize, arrays: &[String]) -> String {
    format!(
        "      <spectrum id=\"spectrum={index}\" defaultArrayLength=\"0\" index=\"{index}\">\n\
         \x20       <binaryDataArrayList count=\"{count}\">\n\
         {body}        </binaryDataArrayList>\n\
         \x20     </spectrum>\n",
        count = arrays.len(),
        body = arrays.concat()
    )
}

pub fn chromatogram_xml(index: usize, arrays: &[String]) -> String {
    format!(
        "      <chromatogram id=\"chromatogram={index}\" defaultArrayLength=\"0\" index=\"{index}\">\n\
         \x20       <binaryDataArrayList count=\"{count}\">\n\
         {body}        </binaryDataArrayList>\n\
         \x20     </chromatogram>\n",
        count = arrays.len(),
        body = arrays.concat()
    )
}

pub fn wrap_mzml(groups_xml: &str, spectra_xml: &str, spectrum_count: usize) -> String {
    wrap_mzml_with_chromatograms(groups_xml, spectra_xml, spectrum_count, "", 0)
}

pub fn wrap_mzml_with_chromatograms(
    groups_xml: &str,
    spectra_xml: &str,
    spectrum_count: usize,
    chromatograms_xml: &str,
    chromatogram_count: usize,
) -> String {
    let chromatogram_list = if chromatogram_count == 0 && chromatograms_xml.is_empty() {
        String::new()
    } else {
        format!(
            "    <chromatogramList count=\"{chromatogram_count}\" defaultDataProcessingRef=\"dp0\">\n\
             {chromatograms_xml}    </chromatogramList>\n"
        )
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>\n\
         <mzML xmlns=\"http://psi.hupo.org/ms/mzml\" version=\"1.1\">\n\
         \x20 <cvList count=\"3\">\n\
         \x20   <cv id=\"MS\" fullName=\"PSI MS\" version=\"1\" URI=\"x\"/>\n\
         \x20   <cv id=\"IMS\" fullName=\"Imaging MS\" version=\"1\" URI=\"x\"/>\n\
         \x20   <cv id=\"UO\" fullName=\"Unit\" version=\"1\" URI=\"x\"/>\n\
         \x20 </cvList>\n\
         \x20 <referenceableParamGroupList count=\"8\">\n\
         {groups_xml}  </referenceableParamGroupList>\n\
         \x20 <run id=\"run0\">\n\
         \x20   <spectrumList count=\"{spectrum_count}\" defaultDataProcessingRef=\"dp0\">\n\
         {spectra_xml}    </spectrumList>\n\
         {chromatogram_list}  </run>\n\
         </mzML>\n"
    )
}

pub fn dataset_two_arrays(spectra: &[(Vec<f64>, Vec<f32>)]) -> (String, Vec<u8>) {
    let mut ibd = vec![0u8; 16];
    let groups = format!(
        "{}{}",
        group_xml("mzArray", "MS:1000514", "m/z array", "MS:1000523", "64-bit float"),
        group_xml(
            "intensityArray",
            "MS:1000515",
            "intensity array",
            "MS:1000521",
            "32-bit float"
        ),
    );
    let mut spectra_xml = String::new();
    for (index, (mz, intensity)) in spectra.iter().enumerate() {
        let mz_bytes: Vec<u8> = mz.iter().flat_map(|v| v.to_le_bytes()).collect();
        let mz_offset = ibd.len();
        ibd.extend_from_slice(&mz_bytes);
        let intensity_bytes: Vec<u8> = intensity.iter().flat_map(|v| v.to_le_bytes()).collect();
        let intensity_offset = ibd.len();
        ibd.extend_from_slice(&intensity_bytes);
        let arrays = [
            binary_array_xml("mzArray", mz_offset, mz.len(), mz_bytes.len()),
            binary_array_xml("intensityArray", intensity_offset, intensity.len(), intensity_bytes.len()),
        ];
        spectra_xml.push_str(&spectrum_xml(index, &arrays));
    }
    (wrap_mzml(&groups, &spectra_xml, spectra.len()), ibd)
}

pub fn single_array_dataset(values: &Array) -> (String, Vec<u8>) {
    let mut ibd = vec![0u8; 16];
    let offset = ibd.len();
    let bytes = values.to_le_bytes();
    ibd.extend_from_slice(&bytes);
    let (type_accession, type_name) = values.type_accession();
    let groups = group_xml("arr0", "MS:1000515", "intensity array", type_accession, type_name);
    let array = binary_array_xml("arr0", offset, values.len(), bytes.len());
    let spectra = spectrum_xml(0, &[array]);
    (wrap_mzml(&groups, &spectra, 1), ibd)
}

fn append_bytes(ibd: &mut Vec<u8>, bytes: &[u8]) -> (usize, usize) {
    let offset = ibd.len();
    ibd.extend_from_slice(bytes);
    (offset, bytes.len())
}

pub fn dataset_with_chromatograms(
    spectra: &[(Vec<f64>, Vec<f32>)],
    chromatograms: &[(Vec<f64>, Vec<f32>)],
) -> (String, Vec<u8>) {
    let mut ibd = vec![0u8; 16];
    let groups = format!(
        "{}{}{}",
        group_xml("mzArray", "MS:1000514", "m/z array", "MS:1000523", "64-bit float"),
        group_xml(
            "intensityArray",
            "MS:1000515",
            "intensity array",
            "MS:1000521",
            "32-bit float"
        ),
        group_xml("timeArray", "MS:1000595", "time array", "MS:1000523", "64-bit float"),
    );

    let mut spectra_xml = String::new();
    for (index, (mz, intensity)) in spectra.iter().enumerate() {
        let mz_bytes: Vec<u8> = mz.iter().flat_map(|v| v.to_le_bytes()).collect();
        let (mz_offset, mz_len) = append_bytes(&mut ibd, &mz_bytes);
        let intensity_bytes: Vec<u8> = intensity.iter().flat_map(|v| v.to_le_bytes()).collect();
        let (intensity_offset, intensity_len) = append_bytes(&mut ibd, &intensity_bytes);
        let arrays = [
            binary_array_xml("mzArray", mz_offset, mz.len(), mz_len),
            binary_array_xml("intensityArray", intensity_offset, intensity.len(), intensity_len),
        ];
        spectra_xml.push_str(&spectrum_xml(index, &arrays));
    }

    let mut chromatograms_xml = String::new();
    for (index, (time, intensity)) in chromatograms.iter().enumerate() {
        let time_bytes: Vec<u8> = time.iter().flat_map(|v| v.to_le_bytes()).collect();
        let (time_offset, time_len) = append_bytes(&mut ibd, &time_bytes);
        let intensity_bytes: Vec<u8> = intensity.iter().flat_map(|v| v.to_le_bytes()).collect();
        let (intensity_offset, intensity_len) = append_bytes(&mut ibd, &intensity_bytes);
        let arrays = [
            binary_array_xml("timeArray", time_offset, time.len(), time_len),
            binary_array_xml("intensityArray", intensity_offset, intensity.len(), intensity_len),
        ];
        chromatograms_xml.push_str(&chromatogram_xml(index, &arrays));
    }

    (
        wrap_mzml_with_chromatograms(
            &groups,
            &spectra_xml,
            spectra.len(),
            &chromatograms_xml,
            chromatograms.len(),
        ),
        ibd,
    )
}

pub fn write_pair(workspace: &Workspace, xml: &str, ibd: &[u8]) -> (PathBuf, PathBuf) {
    let imzml_path = workspace.write("sample.imzML", xml.as_bytes());
    let ibd_path = workspace.write("sample.ibd", ibd);
    (imzml_path, ibd_path)
}

pub fn stream_all(imzml_path: &Path, ibd_path: &Path) -> Result<Vec<Spectrum>, ImzmlError> {
    let mut reader = parse_imzml(imzml_path, ibd_path)?;
    let mut spectra = Vec::new();
    while let Some(spectrum) = reader.get_next_spectrum()? {
        spectra.push(spectrum);
    }
    Ok(spectra)
}

pub fn stream_all_chromatograms(
    imzml_path: &Path,
    ibd_path: &Path,
) -> Result<Vec<Chromatogram>, ImzmlError> {
    let mut reader = parse_imzml(imzml_path, ibd_path)?;
    while reader.get_next_spectrum()?.is_some() {}
    let mut chromatograms = Vec::new();
    while let Some(chromatogram) = reader.get_next_chromatogram()? {
        chromatograms.push(chromatogram);
    }
    Ok(chromatograms)
}

pub fn spectrum_error_source(error: ImzmlError) -> ImzmlError {
    match error {
        ImzmlError::Spectrum { source, .. } => *source,
        other => panic!("expected a spectrum error, got: {other}"),
    }
}

pub fn chromatogram_error_source(error: ImzmlError) -> ImzmlError {
    match error {
        ImzmlError::Chromatogram { source, .. } => *source,
        other => panic!("expected a chromatogram error, got: {other}"),
    }
}

pub fn chromatogram_array(chromatogram: &Chromatogram, array_index: usize) -> &NumericArray {
    chromatogram
        .binary_data_array_list
        .as_ref()
        .expect("chromatogram has an array list")
        .binary_data_arrays
        .get(array_index)
        .expect("array exists")
        .binary
        .as_ref()
        .expect("array has decoded values")
}

pub fn decoded(spectrum: &Spectrum, array_index: usize) -> &NumericArray {
    spectrum
        .binary_data_array_list
        .as_ref()
        .expect("spectrum has an array list")
        .binary_data_arrays
        .get(array_index)
        .expect("array exists")
        .binary
        .as_ref()
        .expect("array has decoded values")
}
