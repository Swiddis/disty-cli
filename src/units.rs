use crate::formatting::Format;

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Unit {
    // Time units
    #[value(name = "ns")]
    Nanoseconds,
    #[value(name = "us")]
    Microseconds,
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
    pub fn scale(&self) -> f64 {
        match self {
            // Time: base unit is nanoseconds
            Self::Nanoseconds => 1.0,
            Self::Microseconds => 1e3,
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

    /// Returns the appropriate output format (time units display as durations, byte units as sizes)
    pub fn default_format(&self) -> Format {
        match self {
            Self::Nanoseconds
            | Self::Microseconds
            | Self::Milliseconds
            | Self::Seconds => Format::Time,
            _ => Format::Bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_unit_scales() {
        assert_eq!(Unit::Nanoseconds.scale(), 1.0);
        assert_eq!(Unit::Microseconds.scale(), 1e3);
        assert_eq!(Unit::Milliseconds.scale(), 1e6);
        assert_eq!(Unit::Seconds.scale(), 1e9);
    }

    #[test]
    fn test_decimal_byte_unit_scales() {
        assert_eq!(Unit::Bytes.scale(), 1.0);
        assert_eq!(Unit::Kilobytes.scale(), 1e3);
        assert_eq!(Unit::Megabytes.scale(), 1e6);
        assert_eq!(Unit::Gigabytes.scale(), 1e9);
        assert_eq!(Unit::Terabytes.scale(), 1e12);
        assert_eq!(Unit::Petabytes.scale(), 1e15);
    }

    #[test]
    fn test_binary_byte_unit_scales() {
        assert_eq!(Unit::Kibibytes.scale(), 1024.0);
        assert_eq!(Unit::Mebibytes.scale(), 1024.0_f64.powi(2));
        assert_eq!(Unit::Gibibytes.scale(), 1024.0_f64.powi(3));
        assert_eq!(Unit::Tebibytes.scale(), 1024.0_f64.powi(4));
        assert_eq!(Unit::Pebibytes.scale(), 1024.0_f64.powi(5));
    }

    #[test]
    fn test_time_unit_default_format() {
        assert!(matches!(Unit::Nanoseconds.default_format(), Format::Time));
        assert!(matches!(Unit::Microseconds.default_format(), Format::Time));
        assert!(matches!(Unit::Milliseconds.default_format(), Format::Time));
        assert!(matches!(Unit::Seconds.default_format(), Format::Time));
    }

    #[test]
    fn test_byte_unit_default_format() {
        assert!(matches!(Unit::Bytes.default_format(), Format::Bytes));
        assert!(matches!(Unit::Kilobytes.default_format(), Format::Bytes));
        assert!(matches!(Unit::Megabytes.default_format(), Format::Bytes));
        assert!(matches!(Unit::Kibibytes.default_format(), Format::Bytes));
        assert!(matches!(Unit::Mebibytes.default_format(), Format::Bytes));
    }

    #[test]
    fn test_conversion_examples() {
        // 5 microseconds = 5000 nanoseconds
        assert_eq!(5.0 * Unit::Microseconds.scale(), 5000.0);

        // 2 megabytes = 2,000,000 bytes
        assert_eq!(2.0 * Unit::Megabytes.scale(), 2_000_000.0);

        // 3 mebibytes = 3,145,728 bytes
        assert_eq!(3.0 * Unit::Mebibytes.scale(), 3_145_728.0);
    }

    #[test]
    fn test_decimal_vs_binary_difference() {
        // 1 MB (decimal) = 1,000,000 bytes
        // 1 MiB (binary) = 1,048,576 bytes
        let mb = Unit::Megabytes.scale();
        let mib = Unit::Mebibytes.scale();

        assert_eq!(mb, 1e6);
        assert_eq!(mib, 1024.0 * 1024.0);
        assert!(mib > mb);
        assert!((mib - mb).abs() > 48_000.0); // At least 48KB difference
    }
}
