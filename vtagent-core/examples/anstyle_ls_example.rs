//! Example demonstrating anstyle-ls usage
//!
//! This example shows how to parse LS_COLORS.

fn main() {
    println!("=== anstyle-ls Example ===");
    
    // Example LS_COLORS string
    let ls_colors = "rs=01;36:di=01;34:*.txt=01;31";
    
    // Parse the LS_COLORS
    let style_map = anstyle_ls::parse(ls_colors);
    
    println!("Parsed style map: {:?}", style_map);
}