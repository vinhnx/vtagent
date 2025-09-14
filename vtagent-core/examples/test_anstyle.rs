use anstyle::{AnsiColor, Color, Style};

fn main() {
    let style = Style::new()
        .fg_color(Some(Color::Ansi(AnsiColor::Red)))
        .bold();
    println!(
        "{}This is a red, bold text!{}",
        style.render(),
        style.render_reset()
    );

    let style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));
    println!(
        "{}This is a green text!{}",
        style.render(),
        style.render_reset()
    );

    let style = Style::new()
        .fg_color(Some(Color::Ansi(AnsiColor::Blue)))
        .bold();
    println!(
        "{}This is a blue, bold text!{}",
        style.render(),
        style.render_reset()
    );
}
