/// Simple Gaussian Kernel Density Estimator
/// TODO make this even faster by porting the fast-kde paper cited at https://github.com/uwdata/fast-kde
#[allow(clippy::upper_case_acronyms)]
pub struct KDE<'a> {
    data: &'a [f64],
    bandwidth: f64,
}

impl<'a> KDE<'a> {
    /// Create a KDE with automatic bandwidth selection (Silverman's rule)
    /// Assumes data is already sorted
    pub fn new(data: &'a [f64]) -> Self {
        let n = data.len() as f64;

        let mean = data.iter().sum::<f64>() / n;
        let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
        let std_dev = variance.sqrt();

        // Silverman's rule of thumb: h ≈ 1.06 * σ * n^(-1/5)
        let bandwidth = 1.06 * std_dev * n.powf(-0.2);

        KDE { data, bandwidth }
    }

    /// Probability density at x
    pub fn pdf(&self, x: f64) -> f64 {
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

        let sum: f64 = self.data[start_idx..end_idx]
            .iter()
            .map(|&xi| gaussian_kernel((x - xi) / h))
            .sum();

        sum / (n * h)
    }

    /// Get bounds for plotting (data range + 10% padding)
    pub fn bounds(&self) -> (f64, f64) {
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
    // We can't use sqrt in const contexts still :(
    const INV_SQRT_2PI: f64 = 0.3989422804014327;
    INV_SQRT_2PI * (-0.5 * u * u).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gaussian_kernel_at_zero() {
        let result = gaussian_kernel(0.0);
        // At u=0, K(0) = 1/√(2π) ≈ 0.3989
        assert!((result - 0.3989422804014327).abs() < 1e-10);
    }

    #[test]
    fn test_gaussian_kernel_symmetric() {
        let u = 1.5;
        assert_eq!(gaussian_kernel(u), gaussian_kernel(-u));
    }

    #[test]
    fn test_gaussian_kernel_decreases() {
        // Kernel should decrease as we move away from 0
        assert!(gaussian_kernel(0.0) > gaussian_kernel(1.0));
        assert!(gaussian_kernel(1.0) > gaussian_kernel(2.0));
        assert!(gaussian_kernel(2.0) > gaussian_kernel(3.0));
    }

    #[test]
    fn test_kde_new_simple() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let kde = KDE::new(&data);

        // Data should match input (already sorted)
        assert_eq!(kde.data, &[1.0, 2.0, 3.0, 4.0, 5.0]);

        // Bandwidth should be positive
        assert!(kde.bandwidth > 0.0);
    }

    #[test]
    fn test_kde_new_sorted_input() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let kde = KDE::new(&data);

        // Data should remain sorted
        assert_eq!(kde.data, &[1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_kde_pdf_at_data_point() {
        let data = vec![1.0, 2.0, 3.0];
        let kde = KDE::new(&data);

        // PDF at actual data points should be positive
        assert!(kde.pdf(1.0) > 0.0);
        assert!(kde.pdf(2.0) > 0.0);
        assert!(kde.pdf(3.0) > 0.0);
    }

    #[test]
    fn test_kde_pdf_peak_at_mean() {
        let data = vec![1.8, 1.9, 2.0, 2.1, 2.2];
        let kde = KDE::new(&data);

        // PDF should be highest near the mean
        let pdf_at_mean = kde.pdf(2.0);
        let pdf_away = kde.pdf(5.0);
        assert!(pdf_at_mean > pdf_away);
    }

    #[test]
    fn test_kde_pdf_decreases_away_from_data() {
        let data = vec![4.0, 4.5, 5.0, 5.5, 6.0]; // Clustered around 5.0 with more spread
        let kde = KDE::new(&data);

        let center = kde.pdf(5.0);
        let far = kde.pdf(15.0);

        // PDF should be much higher at the center than far away
        assert!(center > far);
    }

    #[test]
    fn test_kde_bounds_simple() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let kde = KDE::new(&data);
        let (min, max) = kde.bounds();

        // Bounds should include data range with padding
        assert!(min <= 1.0);
        assert!(max >= 5.0);

        // But should have some padding (10% of range)
        let range = 5.0 - 1.0;
        let expected_padding = range * 0.1;
        assert!((max - 5.0) >= expected_padding * 0.99); // Allow small FP error
    }

    #[test]
    fn test_kde_bounds_non_negative_data() {
        let data = vec![1.0, 2.0, 3.0];
        let kde = KDE::new(&data);
        let (min, _) = kde.bounds();

        // Lower bound should be clamped to 0 for non-negative data
        assert!(min >= 0.0);
    }

    #[test]
    fn test_kde_bounds_negative_data() {
        let data = vec![-5.0, -2.0, 1.0];
        let kde = KDE::new(&data);
        let (min, _) = kde.bounds();

        // Lower bound can go negative for data with negative values
        assert!(min < -5.0); // Should have padding below -5.0
    }

    #[test]
    fn test_kde_bandwidth_silverman() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let n = data.len() as f64;
        let mean = data.iter().sum::<f64>() / n;
        let variance = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
        let std_dev = variance.sqrt();
        let expected_bandwidth = 1.06 * std_dev * n.powf(-0.2);

        let kde = KDE::new(&data);
        assert!((kde.bandwidth - expected_bandwidth).abs() < 1e-10);
    }

    #[test]
    fn test_kde_pdf_bimodal() {
        // Two clusters of points
        let data = vec![1.0, 1.1, 1.2, 5.0, 5.1, 5.2];
        let kde = KDE::new(&data);

        // PDF should have peaks near each cluster
        let pdf_cluster1 = kde.pdf(1.1);
        let pdf_cluster2 = kde.pdf(5.1);
        let pdf_middle = kde.pdf(3.0);

        // Peaks should be higher than middle
        assert!(pdf_cluster1 > pdf_middle);
        assert!(pdf_cluster2 > pdf_middle);
    }
}
