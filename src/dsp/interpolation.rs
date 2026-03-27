//! Interpolation utilities for wavetables and delay lines

/// Linear interpolation between array elements
pub fn linear_interp(samples: &[f32], index: f32) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let len = samples.len();
    let i = index.floor() as usize % len;
    let frac = index.fract();
    let next = (i + 1) % len;

    samples[i] * (1.0 - frac) + samples[next] * frac
}

/// Cubic (Hermite) interpolation for smoother wavetables
pub fn cubic_interp(samples: &[f32], index: f32) -> f32 {
    if samples.len() < 4 {
        return linear_interp(samples, index);
    }

    let len = samples.len();
    let i = index.floor() as usize % len;
    let frac = index.fract();

    let y0 = samples[(i + len - 1) % len];
    let y1 = samples[i];
    let y2 = samples[(i + 1) % len];
    let y3 = samples[(i + 2) % len];

    // Hermite interpolation
    let c0 = y1;
    let c1 = 0.5 * (y2 - y0);
    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);

    ((c3 * frac + c2) * frac + c1) * frac + c0
}

/// Sinc interpolation for highest quality (expensive)
pub fn sinc_interp(samples: &[f32], index: f32, window_size: usize) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let len = samples.len();
    let i = index.floor() as usize;
    let frac = index.fract();

    let mut sum = 0.0;
    let half_window = window_size / 2;

    for j in 0..window_size {
        let offset = j as i32 - half_window as i32;
        let sample_idx = ((i as i32 + offset).rem_euclid(len as i32)) as usize;
        let x = frac - offset as f32;

        // Sinc function
        let sinc = if x.abs() < 0.0001 {
            1.0
        } else {
            let px = std::f32::consts::PI * x;
            px.sin() / px
        };

        // Blackman window
        let t = (j as f32 + 0.5) / window_size as f32;
        let window = 0.42 - 0.5 * (2.0 * std::f32::consts::PI * t).cos()
            + 0.08 * (4.0 * std::f32::consts::PI * t).cos();

        sum += samples[sample_idx] * sinc * window;
    }

    sum
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_linear_interp_exact() {
        let samples = vec![0.0, 1.0, 0.0, -1.0];
        assert_relative_eq!(linear_interp(&samples, 0.0), 0.0, epsilon = 0.001);
        assert_relative_eq!(linear_interp(&samples, 1.0), 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_linear_interp_midpoint() {
        let samples = vec![0.0, 1.0, 0.0, -1.0];
        assert_relative_eq!(linear_interp(&samples, 0.5), 0.5, epsilon = 0.001);
    }

    #[test]
    fn test_linear_interp_wrap() {
        let samples = vec![0.0, 1.0, 0.0, -1.0];
        assert_relative_eq!(linear_interp(&samples, 4.0), 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_cubic_interp_smooth() {
        let samples: Vec<f32> = (0..64)
            .map(|i| (i as f32 * std::f32::consts::PI / 32.0).sin())
            .collect();
        // Cubic should be close to actual sine
        let result = cubic_interp(&samples, 16.5);
        let expected = (16.5 * std::f32::consts::PI / 32.0).sin();
        assert_relative_eq!(result, expected, epsilon = 0.01);
    }

    #[test]
    fn test_sinc_interp_exact() {
        let samples = vec![0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0];
        let result = sinc_interp(&samples, 1.0, 4);
        assert_relative_eq!(result, 1.0, epsilon = 0.1);
    }
}
