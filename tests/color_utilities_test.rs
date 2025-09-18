use vtcode_core::utils::colors::*;

#[test]
fn test_basic_colors() {
    let red_text = red("Hello");
    assert!(red_text.to_string().contains("Hello"));

    let green_text = green("World");
    assert!(green_text.to_string().contains("World"));
}

#[test]
fn test_styles() {
    let bold_text = bold("Bold");
    assert!(bold_text.to_string().contains("Bold"));

    let italic_text = italic("Italic");
    assert!(italic_text.to_string().contains("Italic"));
}

#[test]
fn test_rgb() {
    let rgb_text = rgb("RGB Color", 255, 128, 64);
    assert!(rgb_text.to_string().contains("RGB Color"));
}

#[test]
fn test_custom_style() {
    let styled_text = custom_style("Styled", &["red", "bold"]);
    assert!(styled_text.to_string().contains("Styled"));
}
