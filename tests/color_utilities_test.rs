use vtagent_core::utils::colors::*;

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