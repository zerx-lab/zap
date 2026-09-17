use std::time::Duration;

use super::{breathing_opacity, BREATHING_PERIOD};

#[test]
fn breathing_opacity_starts_near_low_end() {
    assert_eq!(breathing_opacity(Duration::ZERO, BREATHING_PERIOD), 102);
}

#[test]
fn breathing_opacity_peaks_at_half_period() {
    assert_eq!(
        breathing_opacity(BREATHING_PERIOD / 2, BREATHING_PERIOD),
        255
    );
}

#[test]
fn breathing_opacity_is_periodic() {
    assert_eq!(
        breathing_opacity(Duration::ZERO, BREATHING_PERIOD),
        breathing_opacity(BREATHING_PERIOD, BREATHING_PERIOD)
    );
}
