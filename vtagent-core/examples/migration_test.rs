use vtagent_core::ui::styled::*;

fn main() {
    // Test the migrated styling
    info("Testing migrated styling from main_modular.rs");
    println!(
        "{}{}{}",
        Styles::debug().render(),
        "Debug message",
        Styles::debug().render_reset()
    );
    println!(
        "{}{}{}",
        Styles::info().render(),
        "Info message with custom styling",
        Styles::info().render_reset()
    );

    println!("Migration test completed successfully!");
}
