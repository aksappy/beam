use clap::Parser;
use std::fs;
use std::path::PathBuf;

mod animator;
mod ast;
mod gpu_renderer;
mod parser;
mod renderer;

#[derive(Parser)]
#[clap(version = "1.0", author = "Your Name")]
struct Cli {
    /// The path to the .beam file
    path: PathBuf,

    /// Render with GPU acceleration
    #[clap(long)]
    gpu: bool,
}

fn main() {
    let args = Cli::parse();
    let unparsed_file = fs::read_to_string(&args.path).expect("cannot read file");

    match parser::parse_str(&unparsed_file) {
        Ok(script) => {
            println!("✅ Parsed successfully!");
            let output_base = args.path.file_stem().unwrap().to_string_lossy();
            animator::animate_script(&script, &output_base, args.gpu);
        }
        Err(e) => {
            eprintln!("Error parsing script: {}", e);
        }
    }
}
