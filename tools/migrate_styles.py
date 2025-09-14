#!/usr/bin/env python3

import sys
import re

def convert_style_calls(content):
    """
    Convert console::style calls to anstyle calls in Rust code
    """

    # Dictionary mapping console::style calls to anstyle equivalents
    style_mappings = {
        r'style\("([^"]*)"\)\.red\(\)\.bold\(\)': r'Styles::bold_error().render(), "\1", Styles::bold_error().render_reset()',
        r'style\("([^"]*)"\)\.green\(\)\.bold\(\)': r'Styles::bold_success().render(), "\1", Styles::bold_success().render_reset()',
        r'style\("([^"]*)"\)\.blue\(\)\.bold\(\)': r'Styles::header().render(), "\1", Styles::header().render_reset()',
        r'style\("([^"]*)"\)\.yellow\(\)\.bold\(\)': r'Styles::bold_warning().render(), "\1", Styles::bold_warning().render_reset()',
        r'style\("([^"]*)"\)\.cyan\(\)\.bold\(\)': r'Styles::bold().render(), "\1", Styles::bold().render_reset()',
        r'style\("([^"]*)"\)\.magenta\(\)\.bold\(\)': r'Styles::code().render(), "\1", Styles::code().render_reset()',
        r'style\("([^"]*)"\)\.white\(\)\.bold\(\)': r'Styles::bold().render(), "\1", Styles::bold().render_reset()',

        # Without bold
        r'style\("([^"]*)"\)\.red\(\)': r'Styles::error().render(), "\1", Styles::error().render_reset()',
        r'style\("([^"]*)"\)\.green\(\)': r'Styles::success().render(), "\1", Styles::success().render_reset()',
        r'style\("([^"]*)"\)\.blue\(\)': r'Styles::info().render(), "\1", Styles::info().render_reset()',
        r'style\("([^"]*)"\)\.yellow\(\)': r'Styles::warning().render(), "\1", Styles::warning().render_reset()',
        r'style\("([^"]*)"\)\.cyan\(\)': r'Styles::debug().render(), "\1", Styles::debug().render_reset()',
        r'style\("([^"]*)"\)\.magenta\(\)': r'Styles::code().render(), "\1", Styles::code().render_reset()',
        r'style\("([^"]*)"\)\.white\(\)': r'Styles::bold().render(), "\1", Styles::bold().render_reset()',

        # Dim styles
        r'style\("([^"]*)"\)\.dim\(\)': r'Styles::debug().render(), "\1", Styles::debug().render_reset()',
        r'style\("([^"]*)"\)\.dim\(\)\.on_black\(\)': r'Styles::debug().render(), "\1", Styles::debug().render_reset()',

        # Special cases with variables
        r'style\(([^"]*?)\)\.red\(\)\.bold\(\)': r'Styles::bold_error().render(), \1, Styles::bold_error().render_reset()',
        r'style\(([^"]*?)\)\.green\(\)\.bold\(\)': r'Styles::bold_success().render(), \1, Styles::bold_success().render_reset()',
        r'style\(([^"]*?)\)\.blue\(\)\.bold\(\)': r'Styles::header().render(), \1, Styles::header().render_reset()',
        r'style\(([^"]*?)\)\.yellow\(\)\.bold\(\)': r'Styles::bold_warning().render(), \1, Styles::bold_warning().render_reset()',
        r'style\(([^"]*?)\)\.cyan\(\)\.bold\(\)': r'Styles::bold().render(), \1, Styles::bold().render_reset()',
        r'style\(([^"]*?)\)\.magenta\(\)\.bold\(\)': r'Styles::code().render(), \1, Styles::code().render_reset()',
        r'style\(([^"]*?)\)\.white\(\)\.bold\(\)': r'Styles::bold().render(), \1, Styles::bold().render_reset()',

        # Variables without bold
        r'style\(([^"]*?)\)\.red\(\)': r'Styles::error().render(), \1, Styles::error().render_reset()',
        r'style\(([^"]*?)\)\.green\(\)': r'Styles::success().render(), \1, Styles::success().render_reset()',
        r'style\(([^"]*?)\)\.blue\(\)': r'Styles::info().render(), \1, Styles::info().render_reset()',
        r'style\(([^"]*?)\)\.yellow\(\)': r'Styles::warning().render(), \1, Styles::warning().render_reset()',
        r'style\(([^"]*?)\)\.cyan\(\)': r'Styles::debug().render(), \1, Styles::debug().render_reset()',
        r'style\(([^"]*?)\)\.magenta\(\)': r'Styles::code().render(), \1, Styles::code().render_reset()',
        r'style\(([^"]*?)\)\.white\(\)': r'Styles::bold().render(), \1, Styles::bold().render_reset()',
    }

    # Convert println! and eprintln! statements with style calls
    for pattern, replacement in style_mappings.items():
        # Handle println! statements
        println_pattern = r'println!\("([^"]]*)", ' + pattern + r'\)'
        println_replacement = r'println!("\1{}", ' + replacement + ')'
        content = re.sub(println_pattern, println_replacement, content)

        # Handle eprintln! statements
        eprintln_pattern = r'eprintln!\("([^"]]*)", ' + pattern + r'\)'
        eprintln_replacement = r'eprintln!("\1{}", ' + replacement + ')'
        content = re.sub(eprintln_pattern, eprintln_replacement, content)

        # Handle print! statements
        print_pattern = r'print!\("([^"]]*)", ' + pattern + r'\)'
        print_replacement = r'print!("\1{}", ' + replacement + ')'
        content = re.sub(print_pattern, print_replacement, content)

        # Handle standalone style calls (without format strings)
        standalone_pattern = r'println!\(' + pattern + r'\)'
        standalone_replacement = r'println!("{}{}{}", ' + replacement + ')'
        content = re.sub(standalone_pattern, standalone_replacement, content)

    return content

def main():
    if len(sys.argv) != 3:
        print("Usage: python3 migrate_styles.py <input_file> <output_file>")
        sys.exit(1)

    input_file = sys.argv[1]
    output_file = sys.argv[2]

    try:
        with open(input_file, 'r', encoding='utf-8') as f:
            content = f.read()

        converted_content = convert_style_calls(content)

        with open(output_file, 'w', encoding='utf-8') as f:
            f.write(converted_content)

        print(f"Successfully converted {input_file} to {output_file}")

    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()