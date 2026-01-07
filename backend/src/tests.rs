#[cfg(test)]
mod recorder_settings_tests {
    use crate::job::{RecorderSettings, RecordingType, RecorderSettingsError};

    // Test zoom validation
    #[test]
    fn test_zoom_too_high() {
        let settings = RecorderSettings::new(
            RecordingType::PNG, 
            10_000_000, 
            32, 
            10, 
            None
        );
        assert!(matches!(settings.validate(), Err(RecorderSettingsError::ZoomTooHigh)));
    }

    #[test]
    fn test_zoom_min_boundary() {
        let settings = RecorderSettings::new(
            RecordingType::PNG,
            15_000_000,
            0,
            10,
            None
        );
        assert!(settings.validate().is_ok());
    }

    // Test frequency validation
    #[test]
    fn test_frequency_above_max() {
        let settings = RecorderSettings::new(
            RecordingType::PNG,
            16_681_359,
            0,
            10,
            None
        );
        assert!(matches!(settings.validate(), Err(RecorderSettingsError::FrequencyAboveMax)));
    }

    #[test]
    fn test_frequency_below_min() {
        let settings = RecorderSettings::new(
            RecordingType::PNG,
            147_500,
            2,
            10,
            None
        );
        assert!(matches!(settings.validate(), Err(RecorderSettingsError::FrequencyBelowMin)));
    }

    #[test]
    fn test_frequency_within_bounds() {
        let settings = RecorderSettings::new(
            RecordingType::PNG,
            15_000_000,
            0,
            10,
            None
        );
        assert!(settings.validate().is_ok());
    }

    // Test filename generation
    #[test]
    fn test_filename_format_png() {
        let settings = RecorderSettings::new(
            RecordingType::PNG,
            947_500,
            10,
            10,
            None
        );
        let filename = settings.get_filename("UID123");
        assert!(filename.contains("UID123"));
        assert!(filename.contains("Fq9d475e5")); // scientific format
        assert!(filename.contains("Zm10")); // zoom included
    }

    #[test]
    fn test_filename_format_iq() {
        let settings = RecorderSettings::new(
            RecordingType::IQ,
            16_490_000,
            2,
            10,
            None
        );
        let filename = settings.get_filename("UID123");
        assert!(filename.contains("UID123"));
        assert!(filename.contains("Fq1d649e7"));
        assert!(filename.contains("Bw1d2e4"));
    }

    // Test as_args
    #[test]
    fn test_as_args_png() {
        let settings = RecorderSettings::new(
            RecordingType::PNG,
            10_000_000,
            5,
            10,
            None
        );
        let args = settings.as_args("UID123");
        assert!(args.contains(&"--wf".to_string()));
        assert!(args.contains(&"--wf-png".to_string()));
        assert!(args.contains(&"--zoom=5".to_string()));
    }

    #[test]
    fn test_as_args_iq() {
        let settings = RecorderSettings::new(
            RecordingType::IQ,
            10_000_000,
            0,
            10,
            None
        );
        let args = settings.as_args("UID123");
        assert!(args.contains(&"--kiwi-wav".to_string()));
        assert!(args.contains(&"--modulation=iq".to_string()));
    }
}
