use memmap2::Mmap;
use rayon::prelude::*;
use std::fs::File;

use crate::Unit;

/// Parses file using mmap.
/// Much faster than sequential buffered I/O for large files.
pub fn read_file_mmap(file: &File, unit: Option<Unit>) -> Vec<f64> {
    let scale = unit.map(|u| u.scale()).unwrap_or(1.0);

    let mmap = unsafe {
        Mmap::map(file).unwrap_or_else(|e| {
            eprintln!("error mapping file: {}", e);
            std::process::exit(1);
        })
    };

    if mmap.is_empty() {
        return Vec::new();
    }

    let num_threads = rayon::current_num_threads();
    let chunk_size = mmap.len().div_ceil(num_threads);

    // Chunk boundaries must align to line breaks to avoid splitting numbers mid-parse
    let mut boundaries = vec![0];
    for i in 1..num_threads {
        let mut pos = i * chunk_size;
        if pos >= mmap.len() {
            break;
        }
        while pos < mmap.len() && mmap[pos] != b'\n' {
            pos += 1;
        }
        if pos < mmap.len() {
            boundaries.push(pos + 1); // Start after the newline
        }
    }
    boundaries.push(mmap.len());

    let chunks: Vec<_> = boundaries.windows(2).map(|w| (w[0], w[1])).collect();

    let results: Vec<Vec<f64>> = chunks
        .par_iter()
        .map(|&(start, end)| {
            let chunk = &mmap[start..end];
            parse_chunk(chunk, scale)
        })
        .collect();

    results.into_iter().flatten().collect()
}

/// Parses newline-delimited numbers from byte slice.
/// Returns values scaled to base units (ignores invalid lines silently).
fn parse_chunk(chunk: &[u8], scale: f64) -> Vec<f64> {
    let mut values = Vec::new();
    let mut start = 0;

    for (i, &byte) in chunk.iter().enumerate() {
        if byte == b'\n' {
            if i > start {
                let line = &chunk[start..i];
                if let Some(value) = parse_line(line, scale) {
                    values.push(value);
                }
            }
            start = i + 1;
        }
    }

    // Handle last line if no trailing newline
    if start < chunk.len() {
        let line = &chunk[start..];
        if let Some(value) = parse_line(line, scale) {
            values.push(value);
        }
    }

    values
}

/// Parses a single line as either decimal float or hex (0x prefix).
/// Returns None for invalid input rather than panicking (for robustness with untrusted input).
fn parse_line(line: &[u8], scale: f64) -> Option<f64> {
    let mut start = 0;
    let mut end = line.len();

    while start < end && line[start].is_ascii_whitespace() {
        start += 1;
    }
    while end > start && line[end - 1].is_ascii_whitespace() {
        end -= 1;
    }

    if start == end {
        return None;
    }

    let trimmed = &line[start..end];

    let s = std::str::from_utf8(trimmed).ok()?;

    if let Some(hex) = s.strip_prefix("0x") {
        u64::from_str_radix(hex, 16)
            .ok()
            .map(|v| (v as f64) * scale)
    } else {
        s.parse::<f64>().ok().map(|v| v * scale)
    }
}
