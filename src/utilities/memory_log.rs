use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::utilities::{format_bytes, format_percent, get_memory_status};

const MEMORY_LOG_SECONDS: u64 = 2;

pub(crate) struct MemoryLog {
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
    pub(crate) fn new(allow_log: bool) -> Self {
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

    pub(crate) fn write_step(&self, step: &str, spectra_count: usize, chromatogram_count: usize) {
        self.set_step(step);
        self.set_counts(spectra_count, chromatogram_count);
        self.write_now();
    }

    pub(crate) fn set_step(&self, step: &str) {
        let Some(data) = &self.data else {
            return;
        };
        if let Ok(mut current_step) = data.step.lock() {
            *current_step = step.to_owned();
        }
    }

    pub(crate) fn set_counts(&self, spectra_count: usize, chromatogram_count: usize) {
        let Some(data) = &self.data else {
            return;
        };
        data.spectra_count.store(spectra_count, Ordering::Relaxed);
        data.chromatogram_count
            .store(chromatogram_count, Ordering::Relaxed);
    }

    pub(crate) fn set_total_spectra(&self, total_spectra: usize) {
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
        Self {
            start_time: Instant::now(),
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
