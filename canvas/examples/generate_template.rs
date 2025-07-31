// examples/generate_template.rs
use canvas::config::CanvasConfig;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 && args[1] == "clean" {
        // Generate clean template with 80% active code
        let template = CanvasConfig::generate_clean_template();
        println!("{}", template);
    } else {
        // Generate verbose template with descriptions (default)
        let template = CanvasConfig::generate_template();
        println!("{}", template);
    }
}

// Usage:
//   cargo run --example generate_template > canvas_config.toml
//   cargo run --example generate_template clean > canvas_config_clean.toml
