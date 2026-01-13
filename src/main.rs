mod formatting;
mod kde;
mod parsing;
mod stats;
mod units;

use clap::Parser;
use formatting::{Format, get_display_scale};
use kde::KDE;
use rayon::prelude::*;
use stats::Stats;
use std::fs::File;
use std::io::{self, BufRead};
use textplots::{Chart, LabelBuilder, LabelFormat, Plot, Shape};
use units::Unit;

#[derive(Parser)]
#[command(about = "Summarizes numerical distributions", version)]
struct Args {
    /// Input file (stdin if not specified)
    input: Option<std::path::PathBuf>,

    /// Input unit
    #[arg(short, long)]
    unit: Option<Unit>,

    /// Output format
    #[arg(short, long)]
    fmt: Option<Format>,

    /// Skip KDE plotting
    #[arg(long)]
    no_plot: bool,
}

fn main() {
    let args = Args::parse();

    let data = match &args.input {
        Some(path) => {
            let file = File::open(path).unwrap_or_else(|e| {
                eprintln!("error opening {}: {}", path.display(), e);
                std::process::exit(1);
            });
            parsing::read_file_mmap(&file, args.unit)
        }
        None => {
            let reader = Box::new(io::stdin().lock());
            read_input(reader, args.unit)
        }
    };

    if data.is_empty() {
        eprintln!("no input");
        return;
    }

    let format = args
        .fmt
        .or_else(|| args.unit.map(|u| u.default_format()))
        .unwrap_or(Format::Float);

    let stats = Stats::new(data);

    // TODO if no_plot, we should probably just print lines instead of table.
    print_stats_table(&stats, format);
    if !args.no_plot {
        println!();
        plot_kde(&stats, format);
    }
}

/// Parses numeric input (decimal or hex with 0x prefix) from buffered reader.
/// All values are scaled to base units (nanoseconds for time, bytes for size).
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

fn print_stats_table(stats: &Stats, format: Format) {
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

    let max_rows = left_items.len().max(right_items.len());

    for i in 0..max_rows {
        if let Some((label, value)) = left_items.get(i) {
            print!("{:>8}  {:<20}", label, value);
        } else {
            print!("{:30}", "");
        }

        if let Some((label, value)) = right_items.get(i) {
            println!("{:>8}  {}", label, value);
        } else {
            println!();
        }
    }
}

fn plot_kde(stats: &Stats, format: Format) {
    let kde = KDE::new(&stats.data);
    let (min_x, max_x) = kde.bounds();

    let (scale, unit_label) = get_display_scale(max_x, format);

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
            ((x / scale) as f32, y as f32)
        })
        .collect();

    let label_formatter = if !unit_label.is_empty() {
        let unit = unit_label.to_string();
        LabelFormat::Custom(Box::new(move |v: f32| format!("{:.1}{}", v, unit)))
    } else {
        LabelFormat::Value
    };

    Chart::new(160, 40, (min_x / scale) as f32, (max_x / scale) as f32)
        .lineplot(&Shape::Lines(&points))
        .x_label_format(label_formatter)
        .y_label_format(LabelFormat::None)
        .nice();
}
