/// Pre-computed statistics over sorted dataset.
/// Data is kept sorted to enable efficient quantile lookups & binary search.
pub struct Stats {
    pub data: Vec<f64>,
    pub n: usize,
    pub sum: f64,
    pub mean: f64,
    pub geo_mean: f64,
    pub variance: f64,
    pub std_dev: f64,
}

impl Stats {
    pub fn new(mut data: Vec<f64>) -> Self {
        data.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let n = data.len();
        let sum: f64 = data.iter().sum();
        let mean = sum / n as f64;

        let geo_mean = if data.iter().all(|&x| x > 0.0) {
            let log_sum: f64 = data.iter().map(|x| x.ln()).sum();
            (log_sum / n as f64).exp()
        } else {
            f64::NAN
        };

        let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
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
    pub fn quantile(&self, q: f64) -> f64 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = Stats::new(data);

        assert_eq!(stats.n, 5);
        assert_eq!(stats.sum, 15.0);
        assert_eq!(stats.mean, 3.0);
    }

    #[test]
    fn test_stats_sorted() {
        let data = vec![5.0, 2.0, 4.0, 1.0, 3.0];
        let stats = Stats::new(data);

        // Data should be sorted
        assert_eq!(stats.data, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_stats_variance_and_stddev() {
        let data = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let stats = Stats::new(data);

        // Mean = 6.0
        // Variance = ((2-6)² + (4-6)² + (6-6)² + (8-6)² + (10-6)²) / 5
        //          = (16 + 4 + 0 + 4 + 16) / 5 = 40 / 5 = 8.0
        assert_eq!(stats.mean, 6.0);
        assert_eq!(stats.variance, 8.0);
        assert!((stats.std_dev - 8.0_f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_stats_geometric_mean() {
        let data = vec![1.0, 2.0, 4.0, 8.0];
        let stats = Stats::new(data);

        // Geometric mean = (1 * 2 * 4 * 8)^(1/4) = 64^(1/4) = 2.828...
        let expected_gmean = (1.0 * 2.0 * 4.0 * 8.0_f64).powf(0.25);
        assert!((stats.geo_mean - expected_gmean).abs() < 1e-10);
    }

    #[test]
    fn test_stats_geometric_mean_with_zero() {
        let data = vec![0.0, 1.0, 2.0, 3.0];
        let stats = Stats::new(data);

        // Geometric mean is undefined for data containing 0 or negative
        assert!(stats.geo_mean.is_nan());
    }

    #[test]
    fn test_stats_geometric_mean_with_negative() {
        let data = vec![-1.0, 1.0, 2.0, 3.0];
        let stats = Stats::new(data);

        // Geometric mean is undefined for data containing negative numbers
        assert!(stats.geo_mean.is_nan());
    }

    #[test]
    fn test_quantile_min() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = Stats::new(data);

        assert_eq!(stats.quantile(0.0), 1.0);
    }

    #[test]
    fn test_quantile_max() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = Stats::new(data);

        assert_eq!(stats.quantile(1.0), 5.0);
    }

    #[test]
    fn test_quantile_median() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = Stats::new(data);

        assert_eq!(stats.quantile(0.5), 3.0);
    }

    #[test]
    fn test_quantile_interpolation() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = Stats::new(data);

        // 25th percentile: between index 1 (value 2.0) and index 2 (value 3.0)
        // Linear interpolation at 0.25: 2.0 * 0.75 + 3.0 * 0.25 = 2.25
        let q25 = stats.quantile(0.25);
        assert!((q25 - 2.0).abs() < 1e-10);

        // 75th percentile: between index 3 (value 4.0) and index 4 (value 5.0)
        let q75 = stats.quantile(0.75);
        assert!((q75 - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_quantile_even_number_of_values() {
        let data = vec![1.0, 2.0, 3.0, 4.0];
        let stats = Stats::new(data);

        // Median should be between 2.0 and 3.0
        let median = stats.quantile(0.5);
        assert_eq!(median, 2.5);
    }

    #[test]
    fn test_quantile_empty_data() {
        let data = vec![];
        let stats = Stats::new(data);

        assert!(stats.quantile(0.5).is_nan());
    }

    #[test]
    fn test_quantile_single_value() {
        let data = vec![42.0];
        let stats = Stats::new(data);

        assert_eq!(stats.quantile(0.0), 42.0);
        assert_eq!(stats.quantile(0.5), 42.0);
        assert_eq!(stats.quantile(1.0), 42.0);
    }

    #[test]
    fn test_quantile_negative_q() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = Stats::new(data);

        // Negative q should return min
        assert_eq!(stats.quantile(-0.5), 1.0);
    }

    #[test]
    fn test_quantile_q_greater_than_one() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = Stats::new(data);

        // q > 1.0 should return max
        assert_eq!(stats.quantile(1.5), 5.0);
    }

    #[test]
    fn test_stats_with_duplicates() {
        let data = vec![1.0, 2.0, 2.0, 2.0, 5.0];
        let stats = Stats::new(data);

        assert_eq!(stats.n, 5);
        assert_eq!(stats.sum, 12.0);
        assert_eq!(stats.mean, 2.4);
        assert_eq!(stats.quantile(0.5), 2.0);
    }

    #[test]
    fn test_stats_all_same_values() {
        let data = vec![5.0, 5.0, 5.0, 5.0, 5.0];
        let stats = Stats::new(data);

        assert_eq!(stats.mean, 5.0);
        assert_eq!(stats.variance, 0.0);
        assert_eq!(stats.std_dev, 0.0);
        assert_eq!(stats.quantile(0.0), 5.0);
        assert_eq!(stats.quantile(0.5), 5.0);
        assert_eq!(stats.quantile(1.0), 5.0);
    }

    #[test]
    fn test_stats_large_range() {
        let data = vec![1.0, 1000.0, 1000000.0];
        let stats = Stats::new(data);

        assert_eq!(stats.n, 3);
        assert_eq!(stats.sum, 1001001.0);
        assert!((stats.mean - 333667.0).abs() < 1.0);
    }
}
