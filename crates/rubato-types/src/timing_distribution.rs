/// Timing distribution data for result screen.
///
/// Translated from Java: bms.player.beatoraja.result.AbstractResult.TimingDistribution
#[derive(Clone, Debug, Default)]
pub struct TimingDistribution {
    pub distribution: Vec<i32>,
    pub array_center: i32,
    pub average: f32,
    pub std_dev: f32,
}

impl TimingDistribution {
    pub fn timing_distribution(&self) -> &[i32] {
        &self.distribution
    }

    pub fn array_center(&self) -> i32 {
        self.array_center
    }

    pub fn average(&self) -> f32 {
        self.average
    }

    pub fn std_dev(&self) -> f32 {
        self.std_dev
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timing_distribution_default() {
        let td = TimingDistribution::default();
        assert!(td.distribution.is_empty());
        assert_eq!(td.array_center, 0);
        assert_eq!(td.average, 0.0);
        assert_eq!(td.std_dev, 0.0);
    }

    #[test]
    fn test_timing_distribution_with_data() {
        let td = TimingDistribution {
            distribution: vec![0, 1, 5, 10, 5, 1, 0],
            array_center: 3,
            average: -0.5,
            std_dev: 2.1,
        };
        assert_eq!(td.timing_distribution().len(), 7);
        assert_eq!(td.array_center(), 3);
        assert!(td.average() < 0.0);
        assert!(td.std_dev() > 0.0);
    }
}
