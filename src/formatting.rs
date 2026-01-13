#[derive(Clone, Copy, clap::ValueEnum)]
#[allow(non_camel_case_types)]
pub enum Format {
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
    pub fn format(&self, value: f64) -> String {
        match self {
            Format::Float => format!("{:.2}", value),
            Format::Hex => format!("0x{:x}", value as u64),
            Format::Time => format_duration(value),
            Format::Bytes => format_bytes(value),
        }
    }
}

pub fn format_duration(ns: f64) -> String {
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

pub fn format_bytes(bytes: f64) -> String {
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

/// Selects the largest unit where max_value remains >= 1 to avoid tiny decimals
/// (e.g., prefers "500ms" over "0.5s", but "2s" over "2000ms")
pub fn get_display_scale(max_value: f64, format: Format) -> (f64, &'static str) {
    match format {
        Format::Time => {
            // Choose unit based on maximum value (in nanoseconds)
            if max_value < 1e3 {
                (1.0, "ns")
            } else if max_value < 1e6 {
                (1e3, "µs")
            } else if max_value < 1e9 {
                (1e6, "ms")
            } else {
                (1e9, "s")
            }
        }
        Format::Bytes => {
            // Choose unit based on maximum value (in bytes)
            if max_value < 1024.0 {
                (1.0, "B")
            } else if max_value < 1024.0_f64.powi(2) {
                (1024.0, "KiB")
            } else if max_value < 1024.0_f64.powi(3) {
                (1024.0_f64.powi(2), "MiB")
            } else if max_value < 1024.0_f64.powi(4) {
                (1024.0_f64.powi(3), "GiB")
            } else if max_value < 1024.0_f64.powi(5) {
                (1024.0_f64.powi(4), "TiB")
            } else {
                (1024.0_f64.powi(5), "PiB")
            }
        }
        Format::Float => (1.0, ""),
        Format::Hex => (1.0, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_nanoseconds() {
        assert_eq!(format_duration(1.0), "1.00ns");
        assert_eq!(format_duration(500.0), "500.00ns");
        assert_eq!(format_duration(999.0), "999.00ns");
    }

    #[test]
    fn test_format_duration_microseconds() {
        assert_eq!(format_duration(1e3), "1.00µs");
        assert_eq!(format_duration(5e3), "5.00µs");
        assert_eq!(format_duration(500e3), "500.00µs");
    }

    #[test]
    fn test_format_duration_milliseconds() {
        assert_eq!(format_duration(1e6), "1.00ms");
        assert_eq!(format_duration(5e6), "5.00ms");
        assert_eq!(format_duration(500e6), "500.00ms");
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(1e9), "1.00s");
        assert_eq!(format_duration(5e9), "5.00s");
        assert_eq!(format_duration(30e9), "30.00s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(60e9), "1m0.00s");
        assert_eq!(format_duration(90e9), "1m30.00s");
        assert_eq!(format_duration(150e9), "2m30.00s");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(3600e9), "1h0m0.00s");
        assert_eq!(format_duration(3661e9), "1h1m1.00s");
        assert_eq!(format_duration(7384e9), "2h3m4.00s");
    }

    #[test]
    fn test_format_bytes_bytes() {
        assert_eq!(format_bytes(0.0), "0B");
        assert_eq!(format_bytes(100.0), "100B");
        assert_eq!(format_bytes(1023.0), "1023B");
    }

    #[test]
    fn test_format_bytes_kibibytes() {
        assert_eq!(format_bytes(1024.0), "1.00KiB");
        assert_eq!(format_bytes(2048.0), "2.00KiB");
        assert_eq!(format_bytes(1536.0), "1.50KiB");
    }

    #[test]
    fn test_format_bytes_mebibytes() {
        assert_eq!(format_bytes(1024.0 * 1024.0), "1.00MiB");
        assert_eq!(format_bytes(2.5 * 1024.0 * 1024.0), "2.50MiB");
    }

    #[test]
    fn test_format_bytes_gibibytes() {
        assert_eq!(format_bytes(1024.0_f64.powi(3)), "1.00GiB");
        assert_eq!(format_bytes(5.5 * 1024.0_f64.powi(3)), "5.50GiB");
    }

    #[test]
    fn test_format_bytes_tebibytes() {
        assert_eq!(format_bytes(1024.0_f64.powi(4)), "1.00TiB");
        assert_eq!(format_bytes(2.75 * 1024.0_f64.powi(4)), "2.75TiB");
    }

    #[test]
    fn test_format_bytes_pebibytes() {
        assert_eq!(format_bytes(1024.0_f64.powi(5)), "1.00PiB");
        assert_eq!(format_bytes(3.14 * 1024.0_f64.powi(5)), "3.14PiB");
    }

    #[test]
    fn test_format_float() {
        assert_eq!(Format::Float.format(42.567), "42.57");
        assert_eq!(Format::Float.format(0.123), "0.12");
        assert_eq!(Format::Float.format(1000.0), "1000.00");
    }

    #[test]
    fn test_format_hex() {
        assert_eq!(Format::Hex.format(255.0), "0xff");
        assert_eq!(Format::Hex.format(16.0), "0x10");
        assert_eq!(Format::Hex.format(0.0), "0x0");
    }

    #[test]
    fn test_format_time() {
        assert_eq!(Format::Time.format(1e6), "1.00ms");
        assert_eq!(Format::Time.format(60e9), "1m0.00s");
    }

    #[test]
    fn test_format_bytes_format() {
        assert_eq!(Format::Bytes.format(1024.0), "1.00KiB");
        assert_eq!(Format::Bytes.format(1024.0_f64.powi(2)), "1.00MiB");
    }

    #[test]
    fn test_get_display_scale_time_nanoseconds() {
        let (scale, unit) = get_display_scale(500.0, Format::Time);
        assert_eq!(scale, 1.0);
        assert_eq!(unit, "ns");
    }

    #[test]
    fn test_get_display_scale_time_microseconds() {
        let (scale, unit) = get_display_scale(5e3, Format::Time);
        assert_eq!(scale, 1e3);
        assert_eq!(unit, "µs");
    }

    #[test]
    fn test_get_display_scale_time_milliseconds() {
        let (scale, unit) = get_display_scale(5e6, Format::Time);
        assert_eq!(scale, 1e6);
        assert_eq!(unit, "ms");
    }

    #[test]
    fn test_get_display_scale_time_seconds() {
        let (scale, unit) = get_display_scale(5e9, Format::Time);
        assert_eq!(scale, 1e9);
        assert_eq!(unit, "s");
    }

    #[test]
    fn test_get_display_scale_bytes_b() {
        let (scale, unit) = get_display_scale(512.0, Format::Bytes);
        assert_eq!(scale, 1.0);
        assert_eq!(unit, "B");
    }

    #[test]
    fn test_get_display_scale_bytes_kib() {
        let (scale, unit) = get_display_scale(2048.0, Format::Bytes);
        assert_eq!(scale, 1024.0);
        assert_eq!(unit, "KiB");
    }

    #[test]
    fn test_get_display_scale_bytes_mib() {
        let (scale, unit) = get_display_scale(5.0 * 1024.0_f64.powi(2), Format::Bytes);
        assert_eq!(scale, 1024.0_f64.powi(2));
        assert_eq!(unit, "MiB");
    }

    #[test]
    fn test_get_display_scale_float() {
        let (scale, unit) = get_display_scale(1000.0, Format::Float);
        assert_eq!(scale, 1.0);
        assert_eq!(unit, "");
    }

    #[test]
    fn test_get_display_scale_hex() {
        let (scale, unit) = get_display_scale(255.0, Format::Hex);
        assert_eq!(scale, 1.0);
        assert_eq!(unit, "");
    }
}
