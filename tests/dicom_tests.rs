use kepler_wgpu::core::error::KeplerError;
use kepler_wgpu::data::dicom::CTImage;

#[cfg(test)]
mod pixel_data_edge_case_tests_pending {
    use super::*;

    #[test]
    #[ignore]
    fn test_odd_sized_pixel_data() {}

    #[test]
    #[ignore]
    fn test_pixel_data_not_divisible_by_two() {}

    #[test]
    #[ignore]
    fn test_endianness_mismatch() {}

    #[test]
    #[ignore]
    fn test_truncated_pixel_data() {}

    #[test]
    #[ignore]
    fn test_overflow_in_rescaling() {}

    #[test]
    #[ignore]
    fn test_zero_rescale_slope() {}

    #[test]
    #[ignore]
    fn test_negative_rescale_intercept() {}

    #[test]
    #[ignore]
    fn test_precision_loss_on_rounding() {}
}
