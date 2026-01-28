//! Example: Display terminal information
//!
//! Run with: cargo run -p biscuit-terminal --example terminal_info

use biscuit_terminal::terminal::Terminal;

fn main() {
    let term = Terminal::new();

    println!("=== Terminal Information ===\n");

    println!("Application: {:?}", term.app);
    println!("Operating System: {:?}", term.os);
    if let Some(distro) = &term.distro {
        println!("Distribution: {} ({:?})", distro.name, distro.family);
    }

    println!("\n=== Dimensions ===\n");
    println!("Width:  {} columns", Terminal::width());
    println!("Height: {} rows", Terminal::height());

    println!("\n=== Capabilities ===\n");
    println!("TTY:            {}", if term.is_tty { "yes" } else { "no" });
    println!("CI Environment: {}", if term.is_ci { "yes" } else { "no" });
    println!("Color Depth:    {:?}", term.color_depth);
    println!("Color Mode:     {:?}", Terminal::color_mode());
    println!("Italics:        {}", if term.supports_italic { "yes" } else { "no" });
    println!("Images:         {:?}", term.image_support);
    println!("OSC8 Links:     {}", if term.osc_link_support { "yes" } else { "no" });

    println!("\n=== Underline Support ===\n");
    println!("Straight: {}", if term.underline_support.straight { "yes" } else { "no" });
    println!("Double:   {}", if term.underline_support.double { "yes" } else { "no" });
    println!("Curly:    {}", if term.underline_support.curly { "yes" } else { "no" });
    println!("Dotted:   {}", if term.underline_support.dotted { "yes" } else { "no" });
    println!("Dashed:   {}", if term.underline_support.dashed { "yes" } else { "no" });
    println!("Colored:  {}", if term.underline_support.colored { "yes" } else { "no" });

    if let Some(config) = &term.config_file {
        println!("\n=== Config ===\n");
        println!("Config file: {}", config.display());
    }
}
