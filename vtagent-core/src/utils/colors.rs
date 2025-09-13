//! Color utilities for the VT Agent
//!
//! This module provides advanced color manipulation capabilities using the coolor crate,
//! building on top of the existing console and owo-colors functionality.

use ::coolor::{AnsiColor, Hsl, Rgb};

/// Convert RGB values to an ANSI color that works in terminals
pub fn rgb_to_ansi(r: u8, g: u8, b: u8) -> u8 {
    let rgb = Rgb::new(r, g, b);
    let ansi = rgb.to_ansi();
    ansi.code
}

/// Convert HSL values to an ANSI color that works in terminals
pub fn hsl_to_ansi(h: f32, s: f32, l: f32) -> u8 {
    let hsl = Hsl::new(h, s, l);
    let ansi = hsl.to_ansi();
    ansi.code
}

/// Create a console Style from RGB values
pub fn style_from_rgb(r: u8, g: u8, b: u8) -> console::Style {
    let ansi_index = rgb_to_ansi(r, g, b);
    console::Style::new().color256(ansi_index)
}

/// Create a console Style from HSL values
pub fn style_from_hsl(h: f32, s: f32, l: f32) -> console::Style {
    let ansi_index = hsl_to_ansi(h, s, l);
    console::Style::new().color256(ansi_index)
}

/// Generate a harmonious color scheme with a base color
/// Returns a vector of RGB values that work well together
pub fn generate_harmonious_scheme(r: u8, g: u8, b: u8, count: usize) -> Vec<(u8, u8, u8)> {
    let base_rgb = Rgb::new(r, g, b);
    let base_hsl: Hsl = base_rgb.into();
    
    let mut scheme = Vec::with_capacity(count);
    scheme.push((r, g, b));
    
    // Generate complementary and analogous colors
    for i in 1..count {
        let hue_shift = (i as f32) * (360.0 / (count as f32));
        let new_hue = (base_hsl.h + hue_shift) % 360.0;
        let new_hsl = Hsl::new(new_hue, base_hsl.s, base_hsl.l);
        let new_rgb = new_hsl.to_rgb();
        scheme.push((new_rgb.r, new_rgb.g, new_rgb.b));
    }
    
    scheme
}

/// Lighten a color by a given percentage
pub fn lighten_color(r: u8, g: u8, b: u8, percentage: f32) -> (u8, u8, u8) {
    let rgb = Rgb::new(r, g, b);
    let ansi: AnsiColor = rgb.to_ansi();
    let lightened = ansi.with_luminosity_change(percentage);
    let result = lightened.to_rgb();
    (result.r, result.g, result.b)
}

/// Darken a color by a given percentage
pub fn darken_color(r: u8, g: u8, b: u8, percentage: f32) -> (u8, u8, u8) {
    let rgb = Rgb::new(r, g, b);
    let ansi: AnsiColor = rgb.to_ansi();
    let darkened = ansi.with_luminosity_change(-percentage);
    let result = darkened.to_rgb();
    (result.r, result.g, result.b)
}

/// Blend two colors with a given ratio (0.0 to 1.0)
pub fn blend_colors(
    r1: u8, g1: u8, b1: u8,
    r2: u8, g2: u8, b2: u8,
    ratio: f32
) -> (u8, u8, u8) {
    let rgb1 = Rgb::new(r1, g1, b1);
    let rgb2 = Rgb::new(r2, g2, b2);
    // Use the mix method from Hsl for better blending
    let hsl1: Hsl = rgb1.into();
    let hsl2: Hsl = rgb2.into();
    let blended = Hsl::mix(hsl1, 1.0 - ratio, hsl2, ratio);
    let result = blended.to_rgb();
    (result.r, result.g, result.b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_to_ansi() {
        let ansi = rgb_to_ansi(255, 0, 0); // Red
        assert_eq!(ansi, 196); // Standard red in 256-color mode
    }

    #[test]
    fn test_hsl_to_ansi() {
        let ansi = hsl_to_ansi(0.0, 1.0, 0.5); // Red in HSL
        assert_eq!(ansi, 196); // Should convert to same red
    }

    #[test]
    fn test_generate_harmonious_scheme() {
        let scheme = generate_harmonious_scheme(255, 0, 0, 3);
        assert_eq!(scheme.len(), 3);
    }

    #[test]
    fn test_lighten_color() {
        let (r, g, b) = lighten_color(128, 128, 128, 0.5);
        // Lightened gray should be brighter
        assert!(r > 128 && g > 128 && b > 128);
    }

    #[test]
    fn test_darken_color() {
        let (r, g, b) = darken_color(128, 128, 128, 0.5);
        // Darkened gray should be darker
        assert!(r < 128 && g < 128 && b < 128);
    }

    #[test]
    fn test_blend_colors() {
        // Blend red and blue to get purple
        let (r, g, b) = blend_colors(255, 0, 0, 0, 0, 255, 0.5);
        // Should be purple (red + blue)
        assert!(r > 100 && b > 100 && g < 50);
    }
}