use std::path::{Path, PathBuf};
use std::time::Instant;

use imzml::{parse_imzml_with_options, write_ion_file, ConversionOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stem = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/200TopL, 170TopR, 190BottomL, 180BottomR-centroid".to_string());
    let imzml_path = PathBuf::from(format!("{stem}.imzML"));
    let ibd_path = PathBuf::from(format!("{stem}.ibd"));

    let ibd_size = std::fs::metadata(&ibd_path)?.len();
    println!("source: {}", imzml_path.display());
    println!("ibd size: {:.2} MB\n", mb(ibd_size));

    let mib = 1024 * 1024;
    let mz_windows = parse_list(std::env::args().nth(2), &[0.0_f64, 100.0, 250.0, 1000.0]);
    let block_sizes: Vec<usize> = parse_list(std::env::args().nth(3), &[1.0, 16.0, 64.0, 150.0])
        .iter()
        .map(|mb| (*mb as usize) * mib)
        .collect();

    println!(
        "{:>12} {:>12} {:>14} {:>10} {:>9}",
        "mz_window", "block(MB)", "ion size(MB)", "factor", "time(s)"
    );
    println!("{}", "-".repeat(62));

    let mut best: Option<(f64, usize, f64, u64)> = None;
    for &mz_window in &mz_windows {
        for &block_size in &block_sizes {
            let options = ConversionOptions {
                log_memory: false,
                block_size,
                mz_window,
            };
            let (ion_size, secs) = run_one(&imzml_path, &ibd_path, options)?;
            let factor = ibd_size as f64 / ion_size as f64;
            println!(
                "{:>12} {:>12} {:>14.2} {:>9.3}x {:>9.1}",
                fmt_window(mz_window),
                block_size / mib,
                mb(ion_size),
                factor,
                secs
            );
            if best.as_ref().is_none_or(|b| factor > b.0) {
                best = Some((factor, block_size, mz_window, ion_size));
            }
        }
    }

    if let Some((factor, block_size, mz_window, ion_size)) = best {
        println!(
            "\nbest: mz_window={} block_size={}MB -> {:.2} MB ({:.3}x)",
            fmt_window(mz_window),
            block_size / mib,
            mb(ion_size),
            factor
        );
    }
    Ok(())
}

fn run_one(
    imzml_path: &Path,
    ibd_path: &Path,
    options: ConversionOptions,
) -> Result<(u64, f64), Box<dyn std::error::Error>> {
    let out = std::env::temp_dir().join(format!(
        "bench.{}.{}.ion",
        fmt_window(options.mz_window),
        options.block_size
    ));
    let start = Instant::now();
    let imzml = parse_imzml_with_options(imzml_path, ibd_path, options)?;
    write_ion_file(imzml, &out, options)?;
    let secs = start.elapsed().as_secs_f64();
    let size = std::fs::metadata(&out)?.len();
    let _ = std::fs::remove_file(&out);
    Ok((size, secs))
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0
}

fn parse_list(arg: Option<String>, default: &[f64]) -> Vec<f64> {
    match arg {
        Some(text) => text
            .split(',')
            .filter_map(|part| part.trim().parse::<f64>().ok())
            .collect(),
        None => default.to_vec(),
    }
}

fn fmt_window(mz_window: f64) -> String {
    if mz_window == 0.0 {
        "off".to_string()
    } else {
        format!("{mz_window:.0}")
    }
}
