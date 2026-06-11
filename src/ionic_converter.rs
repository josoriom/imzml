use std::{
    collections::HashMap,
    error::Error,
    fs::File,
    io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use ionic::{
    ion::{
        encoder::{
            encode::{WriteOptions, DEFAULT_TARGET_SEGMENT_BYTES},
            scan_stream::ScanStream,
        },
        FileWriter, IonError, IonReader, IonResult, ReadOptions, SectionStorage, TempFile,
    },
    mzml::{
        structs::{
            BinaryData, BinaryDataArray, Chromatogram, CvParam, MzML, NumericType,
            ReferenceableParamGroup, Spectrum,
        },
        MzmlReader,
    },
    IonWriter,
};

const FLOAT_64_BIT: &str = "MS:1000523";
const FLOAT_32_BIT: &str = "MS:1000521";
const EXTERNAL_DATA: &str = "IMS:1000101";
const EXTERNAL_OFFSET: &str = "IMS:1000102";
const EXTERNAL_ARRAY_LENGTH: &str = "IMS:1000103";
const EMPTY_BINARY_TAG: &[u8] = b"<binary/>";
const NORMALIZED_BINARY_TAG: &[u8] = b"<binary></binary>";
const MEMORY_LOG_SECONDS: u64 = 2;
const DEFAULT_BLOCK_SIZE: usize = 1024 * 1024;

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

pub struct Imzml {
    normalized_file: TempFile,
    reader: ImzmlReader,
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            log_memory: false,
            block_size: DEFAULT_BLOCK_SIZE,
        }
    }
}

impl Imzml {
    pub fn get_metadata(&mut self) -> IonResult<MzML> {
        self.reader.metadata()
    }

    pub fn get_next_spectrum(&mut self) -> IonResult<Option<Spectrum>> {
        self.reader.next_spectrum()
    }

    pub fn get_next_chromatogram(&mut self) -> IonResult<Option<Chromatogram>> {
        self.reader.next_chromatogram()
    }

    pub fn normalized_path(&self) -> &Path {
        self.normalized_file.path()
    }

    fn summary(&self) -> ConversionSummary {
        self.reader.summary()
    }

    fn write_memory_now(&mut self, step: &str) {
        self.reader.write_memory_now(step);
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

trait BinarySource {
    fn read_bytes(&mut self, offset: u64, count: usize) -> io::Result<Vec<u8>>;
}

struct IbdFile {
    file: File,
    byte_count: u64,
}

impl IbdFile {
    pub fn open(path: &Path) -> io::Result<Self> {
        let byte_count = std::fs::metadata(path)?.len();
        Ok(Self {
            file: File::open(path)?,
            byte_count,
        })
    }
}

impl BinarySource for IbdFile {
    fn read_bytes(&mut self, offset: u64, count: usize) -> io::Result<Vec<u8>> {
        let end = offset
            .checked_add(count as u64)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "ibd byte range overflow"))?;
        if end > self.byte_count {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!(
                    "ibd file is too small or does not match imzML: need bytes {}..{}, file has {} bytes",
                    offset, end, self.byte_count
                ),
            ));
        }
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
) -> Result<ConversionSummary, Box<dyn Error>> {
    convert_imzml_to_ion_with_options(imzml_path, ibd_path, ion_path, ConversionOptions::default())
}

pub fn convert_imzml_to_ion_with_options(
    imzml_path: &Path,
    ibd_path: &Path,
    ion_path: &Path,
    options: ConversionOptions,
) -> Result<ConversionSummary, Box<dyn Error>> {
    let imzml = parse_imzml_with_options(imzml_path, ibd_path, options)?;
    write_ion_file(imzml, ion_path, options)
}

pub fn parse_imzml(imzml_path: &Path, ibd_path: &Path) -> Result<Imzml, Box<dyn Error>> {
    parse_imzml_with_options(imzml_path, ibd_path, ConversionOptions::default())
}

pub fn parse_imzml_with_options(
    imzml_path: &Path,
    ibd_path: &Path,
    options: ConversionOptions,
) -> Result<Imzml, Box<dyn Error>> {
    let memory_log = MemoryLog::new(options.log_memory);
    memory_log.write_step("start", 0, 0);

    let normalized_file = TempFile::new(imzml_path)?;
    normalize_imzml_file(imzml_path, normalized_file.path())?;
    memory_log.write_step("xml ready", 0, 0);

    let reader = ImzmlReader::open(normalized_file.path(), ibd_path, memory_log)?;
    let mut imzml = Imzml {
        normalized_file,
        reader,
    };
    imzml.write_memory_now("reader ready");
    Ok(imzml)
}

pub fn write_ion_file(
    mut imzml: Imzml,
    ion_path: &Path,
    options: ConversionOptions,
) -> Result<ConversionSummary, Box<dyn Error>> {
    let temp_output = TempFile::new(ion_path)?;
    {
        let mut output = FileWriter::open_path(temp_output.path())
            .map_err(|e| format!("cannot create ion file: {e}"))?;
        let mut writer = IonWriter::begin(&mut output, write_options(options))
            .map_err(|e| format!("cannot start ion writer: {e}"))?;
        writer
            .write_stream(&mut imzml)
            .map_err(|e| format!("cannot write ion file: {e}"))?;
        drop(writer);
        output
            .flush()
            .map_err(|e| format!("cannot flush ion file: {e}"))?;
    }
    imzml.write_memory_now("ion written");
    temp_output
        .move_to(ion_path)
        .map_err(|e| format!("cannot move ion file into place: {e}"))?;
    imzml.write_memory_now("done");
    Ok(imzml.summary())
}

struct ArrayGroup {
    float_type: NumericType,
    inline_params: Vec<CvParam>,
}

#[derive(Clone, Copy)]
struct MemoryStatus {
    current_bytes: u64,
    peak_bytes: u64,
}

struct MemoryLog {
    data: Option<Arc<MemoryLogData>>,
    stop_sender: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

struct MemoryLogData {
    start_time: Instant,
    step: Mutex<String>,
    spectra_count: AtomicUsize,
    total_spectra: AtomicUsize,
    chromatogram_count: AtomicUsize,
}

impl MemoryLog {
    fn new(allow_log: bool) -> Self {
        if !allow_log {
            return Self {
                data: None,
                stop_sender: None,
                thread: None,
            };
        }

        let data = Arc::new(MemoryLogData::new());
        let thread_data = Arc::clone(&data);
        let (stop_sender, stop_receiver) = mpsc::channel();
        let thread = thread::spawn(move || run_memory_log(thread_data, stop_receiver));

        Self {
            data: Some(data),
            stop_sender: Some(stop_sender),
            thread: Some(thread),
        }
    }

    fn write_step(&self, step: &str, spectra_count: usize, chromatogram_count: usize) {
        self.set_step(step);
        self.set_counts(spectra_count, chromatogram_count);
        self.write_now();
    }

    fn set_step(&self, step: &str) {
        let Some(data) = &self.data else {
            return;
        };
        if let Ok(mut current_step) = data.step.lock() {
            *current_step = step.to_owned();
        }
    }

    fn set_counts(&self, spectra_count: usize, chromatogram_count: usize) {
        let Some(data) = &self.data else {
            return;
        };
        data.spectra_count.store(spectra_count, Ordering::Relaxed);
        data.chromatogram_count
            .store(chromatogram_count, Ordering::Relaxed);
    }

    fn set_total_spectra(&self, total_spectra: usize) {
        if let Some(data) = &self.data {
            data.total_spectra.store(total_spectra, Ordering::Relaxed);
        }
    }

    fn write_now(&self) {
        if let Some(data) = &self.data {
            write_memory_line(data);
        }
    }
}

impl Drop for MemoryLog {
    fn drop(&mut self) {
        if let Some(stop_sender) = self.stop_sender.take() {
            let _ = stop_sender.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl MemoryLogData {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            step: Mutex::new("start".to_owned()),
            spectra_count: AtomicUsize::new(0),
            total_spectra: AtomicUsize::new(0),
            chromatogram_count: AtomicUsize::new(0),
        }
    }
}

fn run_memory_log(data: Arc<MemoryLogData>, stop_receiver: mpsc::Receiver<()>) {
    loop {
        match stop_receiver.recv_timeout(Duration::from_secs(MEMORY_LOG_SECONDS)) {
            Ok(_) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => write_memory_line(&data),
        }
    }
}

fn write_memory_line(data: &MemoryLogData) {
    let step = data
        .step
        .lock()
        .map(|step| step.clone())
        .unwrap_or_else(|_| "unknown".to_owned());
    let spectra_count = data.spectra_count.load(Ordering::Relaxed);
    let total_spectra = data.total_spectra.load(Ordering::Relaxed);
    let chromatogram_count = data.chromatogram_count.load(Ordering::Relaxed);
    let elapsed_seconds = data.start_time.elapsed().as_secs_f64();
    let percent = format_percent(spectra_count, total_spectra);
    match get_memory_status() {
        Some(memory) => eprintln!(
            "memory status: step={} spectra={} total={} percent={} chromatograms={} current={} peak={} time={:.1}s",
            step,
            spectra_count,
            total_spectra,
            percent,
            chromatogram_count,
            format_bytes(memory.current_bytes),
            format_bytes(memory.peak_bytes),
            elapsed_seconds
        ),
        None => eprintln!(
            "memory status: step={} spectra={} total={} percent={} chromatograms={} current=unknown peak=unknown time={:.1}s",
            step, spectra_count, total_spectra, percent, chromatogram_count, elapsed_seconds
        ),
    }
}

fn format_bytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1024.0 / 1024.0)
}

fn format_percent(done: usize, total: usize) -> String {
    if total == 0 {
        return "unknown".to_owned();
    }
    format!("{:.1}%", done as f64 * 100.0 / total as f64)
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn get_memory_status() -> Option<MemoryStatus> {
    unsafe {
        let mut data: libc::mach_task_basic_info_data_t = std::mem::zeroed();
        let mut count = libc::MACH_TASK_BASIC_INFO_COUNT;
        let result = libc::task_info(
            libc::mach_task_self_,
            libc::MACH_TASK_BASIC_INFO,
            &mut data as *mut _ as libc::task_info_t,
            &mut count,
        );
        if result != libc::KERN_SUCCESS {
            return None;
        }
        Some(MemoryStatus {
            current_bytes: data.resident_size as u64,
            peak_bytes: data.resident_size_max as u64,
        })
    }
}

#[cfg(target_os = "linux")]
fn get_memory_status() -> Option<MemoryStatus> {
    let text = std::fs::read_to_string("/proc/self/status").ok()?;
    let current_bytes = read_status_kb(&text, "VmRSS:")? * 1024;
    let peak_bytes = read_status_kb(&text, "VmHWM:").unwrap_or(current_bytes / 1024) * 1024;
    Some(MemoryStatus {
        current_bytes,
        peak_bytes,
    })
}

#[cfg(target_os = "linux")]
fn read_status_kb(text: &str, key: &str) -> Option<u64> {
    text.lines()
        .find(|line| line.starts_with(key))
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse().ok())
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn get_memory_status() -> Option<MemoryStatus> {
    None
}

struct ImzmlReader {
    mzml: MzmlReader,
    source: IbdFile,
    groups: HashMap<String, ArrayGroup>,
    spectra_count: usize,
    chromatogram_count: usize,
    memory_log: MemoryLog,
}

impl ImzmlReader {
    fn open(
        imzml_path: &Path,
        ibd_path: &Path,
        memory_log: MemoryLog,
    ) -> Result<Self, Box<dyn Error>> {
        let mut mzml = MzmlReader::open(imzml_path)?;
        let metadata = mzml.metadata()?;
        let total_spectra = metadata
            .run
            .spectrum_list
            .as_ref()
            .and_then(|spectrum_list| spectrum_list.count)
            .unwrap_or(0);
        memory_log.set_total_spectra(total_spectra);
        let groups = collect_array_groups(&metadata);
        Ok(Self {
            mzml,
            source: IbdFile::open(ibd_path)?,
            groups,
            spectra_count: 0,
            chromatogram_count: 0,
            memory_log,
        })
    }

    fn summary(&self) -> ConversionSummary {
        ConversionSummary {
            spectra_count: self.spectra_count,
            chromatogram_count: self.chromatogram_count,
        }
    }

    fn write_memory_now(&mut self, step: &str) {
        self.memory_log
            .write_step(step, self.spectra_count, self.chromatogram_count);
    }

    fn set_memory_step(&self, step: &str) {
        self.memory_log.set_step(step);
    }

    fn set_memory_counts(&self) {
        self.memory_log
            .set_counts(self.spectra_count, self.chromatogram_count);
    }

    fn hydrate_spectrum(&mut self, spectrum: &mut Spectrum) -> IonResult<()> {
        let Some(array_list) = spectrum.binary_data_array_list.as_mut() else {
            return Ok(());
        };
        for array in &mut array_list.binary_data_arrays {
            embed_one_array(array, &self.groups, &mut self.source).map_err(|err| {
                IonError::from(format!(
                    "cannot read imzML spectrum data for index={} id={}: {err}",
                    spectrum.index.unwrap_or_default(),
                    spectrum.id
                ))
            })?;
        }
        Ok(())
    }

    fn hydrate_chromatogram(&mut self, chromatogram: &mut Chromatogram) -> IonResult<()> {
        let Some(array_list) = chromatogram.binary_data_array_list.as_mut() else {
            return Ok(());
        };
        for array in &mut array_list.binary_data_arrays {
            embed_one_array(array, &self.groups, &mut self.source).map_err(|err| {
                IonError::from(format!(
                    "cannot read imzML chromatogram data for index={} id={}: {err}",
                    chromatogram.index.unwrap_or_default(),
                    chromatogram.id
                ))
            })?;
        }
        Ok(())
    }
}

impl ScanStream for ImzmlReader {
    fn metadata(&mut self) -> IonResult<MzML> {
        self.mzml.metadata()
    }

    fn next_spectrum(&mut self) -> IonResult<Option<Spectrum>> {
        let Some(mut spectrum) = self.mzml.next_spectrum()? else {
            return Ok(None);
        };
        self.hydrate_spectrum(&mut spectrum)?;
        self.spectra_count += 1;
        self.set_memory_step("streaming spectra");
        self.set_memory_counts();
        Ok(Some(spectrum))
    }

    fn next_chromatogram(&mut self) -> IonResult<Option<Chromatogram>> {
        let Some(mut chromatogram) = self.mzml.next_chromatogram()? else {
            return Ok(None);
        };
        self.hydrate_chromatogram(&mut chromatogram)?;
        self.chromatogram_count += 1;
        self.set_memory_step("streaming chromatograms");
        self.set_memory_counts();
        Ok(Some(chromatogram))
    }
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
    let Some(offset) = read_cv_u64(array, EXTERNAL_OFFSET) else {
        return Ok(());
    };
    let Some(length) = read_cv_usize(array, EXTERNAL_ARRAY_LENGTH) else {
        return Ok(());
    };

    let byte_count = length
        .checked_mul(byte_width(group.float_type))
        .ok_or("external binary array byte count overflow")?;
    let bytes = source.read_bytes(offset, byte_count)?;
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

fn read_cv_u64(array: &BinaryDataArray, accession: &str) -> Option<u64> {
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

fn normalize_imzml_file(input_path: &Path, output_path: &Path) -> io::Result<()> {
    let input = File::open(input_path)?;
    let output = File::create(output_path)?;
    let mut reader = BufReader::with_capacity(1024 * 1024, input);
    let mut writer = BufWriter::with_capacity(1024 * 1024, output);
    let mut chunk = [0u8; 1024 * 1024];
    let mut pending = Vec::with_capacity(chunk.len() + EMPTY_BINARY_TAG.len());

    loop {
        let read = reader.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        pending.extend_from_slice(&chunk[..read]);
        write_normalized_binary_tags(&mut pending, &mut writer, false)?;
    }

    write_normalized_binary_tags(&mut pending, &mut writer, true)?;
    writer.flush()
}

fn write_normalized_binary_tags(
    pending: &mut Vec<u8>,
    writer: &mut dyn Write,
    final_chunk: bool,
) -> io::Result<()> {
    let mut position = 0;
    let mut write_start = 0;

    while position < pending.len() {
        if position + EMPTY_BINARY_TAG.len() <= pending.len() {
            if pending[position..].starts_with(EMPTY_BINARY_TAG) {
                writer.write_all(&pending[write_start..position])?;
                writer.write_all(NORMALIZED_BINARY_TAG)?;
                position += EMPTY_BINARY_TAG.len();
                write_start = position;
                continue;
            }
            position += 1;
        } else if final_chunk {
            position += 1;
        } else {
            break;
        }
    }

    writer.write_all(&pending[write_start..position])?;
    pending.drain(..position);
    Ok(())
}

fn write_options(options: ConversionOptions) -> WriteOptions {
    WriteOptions {
        compression_level: 18,
        force_f32: false,
        block_size: get_block_size(options),
        parallel: true,
        section_storage: SectionStorage::Disk,
        segment_size: DEFAULT_TARGET_SEGMENT_BYTES,
    }
}

fn get_block_size(options: ConversionOptions) -> usize {
    if options.block_size == 0 {
        DEFAULT_BLOCK_SIZE
    } else {
        options.block_size
    }
}

pub fn read_spectrum_from_ion(
    ion_path: &Path,
    index: usize,
) -> Result<Option<ionic::mzml::structs::Spectrum>, Box<dyn Error>> {
    let mut ion = IonReader::open_file(ion_path, ReadOptions::default())
        .map_err(|e| format!("cannot open ion file: {e}"))?;
    ion.get_spectrum(index)
        .map_err(|e| format!("cannot read spectrum: {e}").into())
}
