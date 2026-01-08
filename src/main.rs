use clap::Parser;
use rayon::prelude::*;
use std::io::{self, BufRead};
use textplots::{Chart, Plot, Shape};

#[derive(Parser)]
#[command(about = "Summarizes numerical distributions", version)]
struct Args {
    /// Input file (stdin if not specified)
    input: Option<std::path::PathBuf>,

    /// Output file (stdout if not specified)
    output: Option<std::path::PathBuf>,

    /// Input unit
    #[arg(short, long)]
    unit: Option<Unit>,

    /// Output format
    #[arg(short, long)]
    fmt: Option<Format>,
}

#[derive(Clone, Copy, clap::ValueEnum)]
#[allow(non_camel_case_types)]
enum Unit {
    // Time units
    #[value(name = "ns")]
    Nanoseconds,
    #[value(name = "us")]
    Microseconds,
    #[value(name = "µs")]
    MicrosecondsMu,
    #[value(name = "ms")]
    Milliseconds,
    #[value(name = "s")]
    Seconds,

    // Byte units (decimal)
    #[value(name = "B")]
    Bytes,
    #[value(name = "KB")]
    Kilobytes,
    #[value(name = "MB")]
    Megabytes,
    #[value(name = "GB")]
    Gigabytes,
    #[value(name = "TB")]
    Terabytes,
    #[value(name = "PB")]
    Petabytes,

    // Byte units (binary)
    #[value(name = "KiB")]
    Kibibytes,
    #[value(name = "MiB")]
    Mebibytes,
    #[value(name = "GiB")]
    Gibibytes,
    #[value(name = "TiB")]
    Tebibytes,
    #[value(name = "PiB")]
    Pebibytes,
}

impl Unit {
    /// Get the scale factor to convert from this unit to base unit
    fn scale(&self) -> f64 {
        match self {
            // Time: base unit is nanoseconds
            Self::Nanoseconds => 1.0,
            Self::Microseconds | Self::MicrosecondsMu => 1e3,
            Self::Milliseconds => 1e6,
            Self::Seconds => 1e9,

            // Bytes: base unit is bytes
            Self::Bytes => 1.0,
            Self::Kilobytes => 1e3,
            Self::Megabytes => 1e6,
            Self::Gigabytes => 1e9,
            Self::Terabytes => 1e12,
            Self::Petabytes => 1e15,
            Self::Kibibytes => 1024.0,
            Self::Mebibytes => 1024.0_f64.powi(2),
            Self::Gibibytes => 1024.0_f64.powi(3),
            Self::Tebibytes => 1024.0_f64.powi(4),
            Self::Pebibytes => 1024.0_f64.powi(5),
        }
    }

    /// Get the default format for this unit
    fn default_format(&self) -> Format {
        match self {
            Self::Nanoseconds | Self::Microseconds | Self::MicrosecondsMu
            | Self::Milliseconds | Self::Seconds => Format::Time,
            _ => Format::Bytes,
        }
    }
}

#[derive(Clone, Copy, clap::ValueEnum)]
#[allow(non_camel_case_types)]
enum Format {
    #[value(name = "float")]
    Float,
    #[value(name = "hex")]
    Hex,
    #[value(name = "time")]
    Time,
    #[value(name = "bytes")]
    Bytes,
}

impl Format {
    fn format(&self, value: f64) -> String {
        match self {
            Format::Float => format!("{:.2}", value),
            Format::Hex => format!("0x{:x}", value as u64),
            Format::Time => format_duration(value),
            Format::Bytes => format_bytes(value),
        }
    }
}

/// Format a duration in nanoseconds as a human-readable string
fn format_duration(ns: f64) -> String {
    if ns < 1e3 {
        format!("{:.2}ns", ns)
    } else if ns < 1e6 {
        format!("{:.2}µs", ns / 1e3)
    } else if ns < 1e9 {
        format!("{:.2}ms", ns / 1e6)
    } else if ns < 60e9 {
        format!("{:.2}s", ns / 1e9)
    } else if ns < 3600e9 {
        let mins = (ns / 60e9).floor();
        let secs = (ns - mins * 60e9) / 1e9;
        format!("{}m{:.2}s", mins as i64, secs)
    } else {
        let hours = (ns / 3600e9).floor();
        let mins = ((ns - hours * 3600e9) / 60e9).floor();
        let secs = (ns - hours * 3600e9 - mins * 60e9) / 1e9;
        format!("{}h{}m{:.2}s", hours as i64, mins as i64, secs)
    }
}

/// Format bytes with IEC binary prefixes
fn format_bytes(bytes: f64) -> String {
    let units = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    let mut value = bytes;
    let mut unit_idx = 0;

    while value >= 1024.0 && unit_idx < units.len() - 1 {
        value /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{:.0}{}", value, units[unit_idx])
    } else {
        format!("{:.2}{}", value, units[unit_idx])
    }
}

fn main() {
    let args = Args::parse();

    let reader: Box<dyn BufRead> = match &args.input {
        Some(path) => {
            let file = std::fs::File::open(path).unwrap_or_else(|e| {
                eprintln!("error opening {}: {}", path.display(), e);
                std::process::exit(1);
            });
            Box::new(io::BufReader::new(file))
        }
        None => Box::new(io::stdin().lock()),
    };

    let data = read_input(reader, args.unit);

    if data.is_empty() {
        eprintln!("no input");
        return;
    }

    // Determine output format
    let format = args.fmt.or_else(|| args.unit.map(|u| u.default_format()))
        .unwrap_or(Format::Float);

    let stats = Stats::new(data);

    // For now, output to stdout (can extend to support output file later)
    print_stats_table(&stats, format);
    println!();
    plot_kde(&stats);
}

/// Read numbers from input, one per line
fn read_input(reader: Box<dyn BufRead>, unit: Option<Unit>) -> Vec<f64> {
    let scale = unit.map(|u| u.scale()).unwrap_or(1.0);
    let mut values = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("error reading input: {}", e);
                std::process::exit(1);
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let value = if let Some(hex) = trimmed.strip_prefix("0x") {
            match u64::from_str_radix(hex, 16) {
                Ok(v) => v as f64,
                Err(e) => {
                    eprintln!("error parsing hex '{}': {}", trimmed, e);
                    std::process::exit(1);
                }
            }
        } else {
            match trimmed.parse::<f64>() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("error parsing number '{}': {}", trimmed, e);
                    std::process::exit(1);
                }
            }
        };

        values.push(value * scale);
    }

    values
}

/// Statistics computed from a dataset
struct Stats {
    data: Vec<f64>,
    n: usize,
    sum: f64,
    mean: f64,
    geo_mean: f64,
    variance: f64,
    std_dev: f64,
}

impl Stats {
    fn new(mut data: Vec<f64>) -> Self {
        data.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let n = data.len();
        let sum: f64 = data.iter().sum();
        let mean = sum / n as f64;

        // Geometric mean (only for positive values)
        let geo_mean = if data.iter().all(|&x| x > 0.0) {
            let log_sum: f64 = data.iter().map(|x| x.ln()).sum();
            (log_sum / n as f64).exp()
        } else {
            f64::NAN
        };

        // Variance and standard deviation
        let variance = data.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / n as f64;
        let std_dev = variance.sqrt();

        Stats {
            data,
            n,
            sum,
            mean,
            geo_mean,
            variance,
            std_dev,
        }
    }

    /// Calculate quantile (0.0 = min, 0.5 = median, 1.0 = max)
    fn quantile(&self, q: f64) -> f64 {
        if self.data.is_empty() {
            return f64::NAN;
        }
        if q <= 0.0 {
            return self.data[0];
        }
        if q >= 1.0 {
            return self.data[self.n - 1];
        }

        // Linear interpolation between closest ranks
        let rank = q * (self.n - 1) as f64;
        let lower = rank.floor() as usize;
        let upper = rank.ceil() as usize;
        let fraction = rank - lower as f64;

        self.data[lower] * (1.0 - fraction) + self.data[upper] * fraction
    }
}

fn print_stats_table(stats: &Stats, format: Format) {
    // Build summary stats (left column items)
    let mut left_items = vec![
        ("n", stats.n.to_string()),
        ("sum", format.format(stats.sum)),
        ("mean", format.format(stats.mean)),
    ];

    if !stats.geo_mean.is_nan() {
        left_items.push(("gmean", format.format(stats.geo_mean)));
    }

    left_items.push(("std dev", format.format(stats.std_dev)));
    left_items.push(("variance", format.format(stats.variance)));

    // Build percentiles (right column items)
    let percentiles = [
        (0.0, "min"),
        (0.01, "1%ile"),
        (0.05, "5%ile"),
        (0.25, "25%ile"),
        (0.50, "median"),
        (0.75, "75%ile"),
        (0.95, "95%ile"),
        (0.99, "99%ile"),
        (1.0, "max"),
    ];

    let right_items: Vec<(&str, String)> = percentiles
        .iter()
        .map(|(q, label)| (*label, format.format(stats.quantile(*q))))
        .collect();

    // Print two-column table
    let max_rows = left_items.len().max(right_items.len());

    for i in 0..max_rows {
        // Left column
        if let Some((label, value)) = left_items.get(i) {
            print!("{:>8}  {:<20}", label, value);
        } else {
            print!("{:30}", "");
        }

        // Right column
        if let Some((label, value)) = right_items.get(i) {
            println!("{:>8}  {}", label, value);
        } else {
            println!();
        }
    }
}

fn plot_kde(stats: &Stats) {
    let kde = KDE::new(stats.data.clone());
    let (min_x, max_x) = kde.bounds();

    // Pre-sample KDE in parallel at chart width points
    // This mimics what textplots does internally for Shape::Continuous,
    // but parallelizes the expensive kde.pdf() evaluations
    const CHART_WIDTH: usize = 160;
    let points: Vec<(f32, f32)> = (0..CHART_WIDTH)
        .into_par_iter()
        .map(|i| {
            // Map pixel coordinate to data coordinate (inv_linear)
            let x = min_x + (max_x - min_x) * (i as f64 / (CHART_WIDTH - 1) as f64);
            let y = kde.pdf(x);
            (x as f32, y as f32)
        })
        .collect();

    Chart::new(160, 40, min_x as f32, max_x as f32)
        .lineplot(&Shape::Lines(&points))
        .nice();
}

/// Simple Gaussian Kernel Density Estimator
struct KDE {
    data: Vec<f64>,
    bandwidth: f64,
}

impl KDE {
    /// Create a KDE with automatic bandwidth selection (Silverman's rule)
    fn new(mut data: Vec<f64>) -> Self {
        let n = data.len() as f64;

        // Calculate standard deviation for bandwidth
        let mean = data.iter().sum::<f64>() / n;
        let variance = data.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / n;
        let std_dev = variance.sqrt();

        // Silverman's rule of thumb: h ≈ 1.06 * σ * n^(-1/5)
        let bandwidth = 1.06 * std_dev * n.powf(-0.2);

        data.sort_by(|a, b| a.partial_cmp(b).unwrap());

        KDE { data, bandwidth }
    }

    /// Evaluate the probability density function at point x
    fn pdf(&self, x: f64) -> f64 {
        let n = self.data.len() as f64;
        let h = self.bandwidth;

        // Optimization: Only consider points within ~4 bandwidths
        // Beyond that, gaussian kernel contribution is < 0.00003 (negligible)
        let cutoff = 4.0 * h;
        let lower = x - cutoff;
        let upper = x + cutoff;

        // Binary search to find the range of relevant points (data is sorted)
        let start_idx = self.data.partition_point(|&xi| xi < lower);
        let end_idx = self.data.partition_point(|&xi| xi <= upper);

        // Only evaluate kernel for points in range
        let sum: f64 = self.data[start_idx..end_idx]
            .iter()
            .map(|&xi| gaussian_kernel((x - xi) / h))
            .sum();

        sum / (n * h)
    }

    /// Get bounds for plotting (data range + 10% padding)
    fn bounds(&self) -> (f64, f64) {
        let min = self.data.first().copied().unwrap_or(0.0);
        let max = self.data.last().copied().unwrap_or(1.0);
        let padding = (max - min) * 0.1;

        // Clamp lower bound to 0 if all data is non-negative
        let lower = if min >= 0.0 {
            (min - padding).max(0.0)
        } else {
            min - padding
        };

        (lower, max + padding)
    }
}

/// Standard Gaussian kernel: K(u) = (1/√(2π)) * e^(-u²/2)
fn gaussian_kernel(u: f64) -> f64 {
    const INV_SQRT_2PI: f64 = 0.3989422804014327;
    INV_SQRT_2PI * (-0.5 * u * u).exp()
}
