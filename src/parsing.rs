use memmap2::Mmap;
use rayon::prelude::*;
use std::fs::File;

use crate::units::Unit;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Unit;

    #[test]
    fn test_parse_line_decimal() {
        assert_eq!(parse_line(b"42.5", 1.0), Some(42.5));
        assert_eq!(parse_line(b"100", 1.0), Some(100.0));
        assert_eq!(parse_line(b"0", 1.0), Some(0.0));
        assert_eq!(parse_line(b"-5.5", 1.0), Some(-5.5));
    }

    #[test]
    fn test_parse_line_hex() {
        assert_eq!(parse_line(b"0x10", 1.0), Some(16.0));
        assert_eq!(parse_line(b"0xFF", 1.0), Some(255.0));
        assert_eq!(parse_line(b"0x0", 1.0), Some(0.0));
        assert_eq!(parse_line(b"0xDEADBEEF", 1.0), Some(3735928559.0));
    }

    #[test]
    fn test_parse_line_with_whitespace() {
        assert_eq!(parse_line(b"  42.5  ", 1.0), Some(42.5));
        assert_eq!(parse_line(b"\t100\n", 1.0), Some(100.0));
        assert_eq!(parse_line(b"  0x10  ", 1.0), Some(16.0));
    }

    #[test]
    fn test_parse_line_with_scale() {
        assert_eq!(parse_line(b"10", 2.0), Some(20.0));
        assert_eq!(parse_line(b"5.5", 1000.0), Some(5500.0));
        assert_eq!(parse_line(b"0x10", 10.0), Some(160.0));
    }

    #[test]
    fn test_parse_line_invalid() {
        assert_eq!(parse_line(b"", 1.0), None);
        assert_eq!(parse_line(b"   ", 1.0), None);
        assert_eq!(parse_line(b"not_a_number", 1.0), None);
        assert_eq!(parse_line(b"0xinvalid", 1.0), None);
        assert_eq!(parse_line(b"12.34.56", 1.0), None);
    }

    #[test]
    fn test_parse_chunk_single_line() {
        let chunk = b"42.5\n";
        let result = parse_chunk(chunk, 1.0);
        assert_eq!(result, vec![42.5]);
    }

    #[test]
    fn test_parse_chunk_multiple_lines() {
        let chunk = b"10\n20\n30\n";
        let result = parse_chunk(chunk, 1.0);
        assert_eq!(result, vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn test_parse_chunk_mixed_formats() {
        let chunk = b"10\n0x20\n30.5\n";
        let result = parse_chunk(chunk, 1.0);
        assert_eq!(result, vec![10.0, 32.0, 30.5]);
    }

    #[test]
    fn test_parse_chunk_with_invalid_lines() {
        let chunk = b"10\ninvalid\n20\n";
        let result = parse_chunk(chunk, 1.0);
        assert_eq!(result, vec![10.0, 20.0]); // Invalid line is skipped
    }

    #[test]
    fn test_parse_chunk_no_trailing_newline() {
        let chunk = b"10\n20\n30";
        let result = parse_chunk(chunk, 1.0);
        assert_eq!(result, vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn test_parse_chunk_empty_lines() {
        let chunk = b"10\n\n20\n\n\n30\n";
        let result = parse_chunk(chunk, 1.0);
        assert_eq!(result, vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn test_parse_chunk_with_scale() {
        let chunk = b"1\n2\n3\n";
        let result = parse_chunk(chunk, 1000.0);
        assert_eq!(result, vec![1000.0, 2000.0, 3000.0]);
    }

    #[test]
    fn test_read_file_mmap_with_units() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "1000").unwrap();
        writeln!(temp_file, "2000").unwrap();
        writeln!(temp_file, "3000").unwrap();
        temp_file.flush().unwrap();

        let file = temp_file.reopen().unwrap();
        let result = read_file_mmap(&file, Some(Unit::Microseconds));

        // Microseconds scale is 1e3, so values should be multiplied
        assert_eq!(result, vec![1_000_000.0, 2_000_000.0, 3_000_000.0]);
    }

    #[test]
    fn test_read_file_mmap_empty() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let file = temp_file.reopen().unwrap();
        let result = read_file_mmap(&file, None);

        assert_eq!(result, vec![]);
    }
}
