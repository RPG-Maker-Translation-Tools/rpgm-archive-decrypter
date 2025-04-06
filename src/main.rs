use clap::{value_parser, ArgAction, Parser};
use rpgmad_lib::Decrypter;
use std::{
    fs::{read, read_dir},
    path::PathBuf,
    time::Instant,
};

#[derive(Parser, Debug)]
#[command(
    about = "Extract encrypted .rgss RPG Maker archives.",
    term_width = 120
)]
struct Cli {
    /// Path to the .rgss file or directory containing it.
    #[arg(short, long, value_parser = value_parser!(PathBuf), default_value = "./", hide_default_value = true)]
    input_path: PathBuf,

    /// Output directory.
    #[arg(short, long, value_parser = value_parser!(PathBuf), default_value = "./", hide_default_value = true)]
    output_path: PathBuf,

    /// Overwrite existing files.
    #[arg(short, long, action = ArgAction::SetTrue)]
    force: bool,
}

fn main() {
    let start_time: Instant = Instant::now();
    let mut cli: Cli = Cli::parse();

    if !cli.input_path.exists() {
        panic!("Input path does not exist.");
    }

    if !cli.output_path.exists() {
        panic!("Output path does not exist.");
    }

    if cli
        .input_path
        .extension()
        .and_then(|e| e.to_str())
        .is_none_or(|e| !e.starts_with("rgss"))
    {
        cli.input_path = read_dir(&cli.input_path)
            .unwrap()
            .flatten()
            .find(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.starts_with("rgss"))
            })
            .map(|e| e.path())
            .expect("No .rgss archive found in the directory.");
    }

    let bytes: Vec<u8> = read(&cli.input_path).unwrap();

    Decrypter::new(bytes)
        .extract(&cli.output_path, cli.force)
        .unwrap();

    println!("Elapsed: {:.2}s", start_time.elapsed().as_secs_f32());
}
