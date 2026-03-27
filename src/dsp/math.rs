//! Fast math utilities for DSP

use std::f32::consts::PI;

/// Fast sine approximation using polynomial
/// Accurate to ~0.001 for inputs in [-PI, PI]
pub fn fast_sin(x: f32) -> f32 {
    // Normalize to [-PI, PI]
    let mut x = x;
    while x > PI {
        x -= 2.0 * PI;
    }
    while x < -PI {
        x += 2.0 * PI;
    }

    // Parabolic approximation with correction
    let y = (4.0 / PI) * x - (4.0 / (PI * PI)) * x * x.abs();
    // Add precision with extra term
    0.225 * (y * y.abs() - y) + y
}

/// Fast tanh approximation
/// Uses rational approximation
pub fn fast_tanh(x: f32) -> f32 {
    if x < -3.0 {
        -1.0
    } else if x > 3.0 {
        1.0
    } else {
        let x2 = x * x;
        x * (27.0 + x2) / (27.0 + 9.0 * x2)
    }
}

/// Convert MIDI note number to frequency in Hz
/// A4 (note 69) = 440 Hz
pub fn midi_to_freq(note: u8) -> f32 {
    440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0)
}

/// Convert frequency to MIDI note number
pub fn freq_to_midi(freq: f32) -> f32 {
    69.0 + 12.0 * (freq / 440.0).log2()
}

/// Convert decibels to linear amplitude
pub fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

/// Convert linear amplitude to decibels
pub fn linear_to_db(linear: f32) -> f32 {
    20.0 * linear.log10()
}

/// Soft clipping using tanh
pub fn soft_clip(x: f32, drive: f32) -> f32 {
    fast_tanh(x * drive)
}

/// Hard clipping
pub fn hard_clip(x: f32, threshold: f32) -> f32 {
    x.clamp(-threshold, threshold)
}

/// Linear interpolation
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Exponential curve for envelope times
/// t in [0, 1], curve in [-1, 1] (0 = linear)
pub fn exp_curve(t: f32, curve: f32) -> f32 {
    if curve.abs() < 0.001 {
        t
    } else if curve > 0.0 {
        // Exponential rise
        (1.0 - (-curve * 5.0 * t).exp()) / (1.0 - (-curve * 5.0).exp())
    } else {
        // Logarithmic rise
        1.0 - (1.0 - (-curve * 5.0 * (1.0 - t)).exp()) / (1.0 - (-curve * 5.0).exp())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_fast_sin_zero() {
        assert_relative_eq!(fast_sin(0.0), 0.0, epsilon = 0.01);
    }

    #[test]
    fn test_fast_sin_pi_half() {
        assert_relative_eq!(fast_sin(PI / 2.0), 1.0, epsilon = 0.01);
    }

    #[test]
    fn test_fast_sin_pi() {
        assert_relative_eq!(fast_sin(PI), 0.0, epsilon = 0.01);
    }

    #[test]
    fn test_fast_tanh_bounds() {
        assert_relative_eq!(fast_tanh(0.0), 0.0, epsilon = 0.01);
        assert_relative_eq!(fast_tanh(5.0), 1.0, epsilon = 0.01);
        assert_relative_eq!(fast_tanh(-5.0), -1.0, epsilon = 0.01);
    }

    #[test]
    fn test_midi_to_freq_a4() {
        assert_relative_eq!(midi_to_freq(69), 440.0, epsilon = 0.1);
    }

    #[test]
    fn test_midi_to_freq_c4() {
        assert_relative_eq!(midi_to_freq(60), 261.63, epsilon = 0.1);
    }

    #[test]
    fn test_midi_to_freq_octave() {
        let f1 = midi_to_freq(60);
        let f2 = midi_to_freq(72);
        assert_relative_eq!(f2 / f1, 2.0, epsilon = 0.001);
    }

    #[test]
    fn test_db_conversion_roundtrip() {
        let linear = 0.5;
        let db = linear_to_db(linear);
        let back = db_to_linear(db);
        assert_relative_eq!(back, linear, epsilon = 0.001);
    }

    #[test]
    fn test_soft_clip_preserves_small() {
        let x = 0.1;
        assert_relative_eq!(soft_clip(x, 1.0), x, epsilon = 0.02);
    }

    #[test]
    fn test_lerp() {
        assert_relative_eq!(lerp(0.0, 10.0, 0.5), 5.0, epsilon = 0.001);
        assert_relative_eq!(lerp(0.0, 10.0, 0.0), 0.0, epsilon = 0.001);
        assert_relative_eq!(lerp(0.0, 10.0, 1.0), 10.0, epsilon = 0.001);
    }

    #[test]
    fn test_exp_curve_linear() {
        assert_relative_eq!(exp_curve(0.5, 0.0), 0.5, epsilon = 0.01);
    }
}
