#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::needless_doctest_main)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::deref_addrof)]

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use rpgmad_lib::{ArchiveEntry, Decrypter, Engine};
use std::{
    borrow::Cow,
    ffi::OsStr,
    fs::{create_dir_all, read, read_dir, write},
    io::stdin,
    path::PathBuf,
    time::Instant,
};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(
    about = "A tool to extract encrypted .rgss RPG Maker archives and encrypt RPG Maker assets back to archives.",
    version,
    term_width = 120
)]
struct Cli {
    /// Subcommand: encrypt or decrypt
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Decrypt an .rgss archive
    Decrypt {
        /// Path to the .rgss file or directory containing it.
        #[arg(
            short,
            long,
            value_name = "INPUT_PATH",
            default_value = "./",
            hide_default_value = true
        )]
        input_path: PathBuf,

        /// Output directory. Defaults to `input_path` if not set.
        #[arg(short, long, value_name = "OUTPUT_PATH")]
        output_path: Option<PathBuf>,
    },

    /// Encrypt RPG Maker Data/Graphics assets to an archive
    Encrypt {
        /// Path to directory containing Data/Graphics directories
        #[arg(
            short,
            long,
            value_name = "INPUT_PATH",
            default_value = "./",
            hide_default_value = true
        )]
        input_path: PathBuf,

        /// Path to write output .rgss file
        #[arg(short, long, value_name = "OUTPUT_PATH")]
        output_path: Option<PathBuf>,

        /// Engine to produce proper archive
        #[arg(short, long, value_name = "ENGINE", value_parser = ["xp", "vx", "vxace"])]
        engine: String,
    },
}

const ENCRYPT_DIRS: &[&str] = &["Graphics", "Data", "Audio", "Fonts"];

fn decrypt_path(
    mut input_path: PathBuf,
    output_path: Option<PathBuf>,
) -> Result<()> {
    if !input_path.exists() {
        bail!("Input path does not exist.");
    }

    let output_path = output_path.unwrap_or_else(|| {
        if input_path.is_file() {
            input_path.parent().unwrap().to_path_buf()
        } else {
            input_path.clone()
        }
    });

    if !output_path.exists() {
        bail!("Output path does not exist.");
    }

    // Detect .rgss file inside folder
    if input_path
        .extension()
        .and_then(OsStr::to_str)
        .is_none_or(|ext| !ext.starts_with("rgss"))
    {
        input_path = read_dir(&input_path)?
            .flatten()
            .find(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(OsStr::to_str)
                    .is_some_and(|ext| ext.starts_with("rgss"))
            })
            .map(|entry| entry.path())
            .context("No .rgss archive found in the directory.")?;
    }

    let mut decrypter = Decrypter::new();
    let archive_data = read(&input_path)?;
    let decrypted_files = decrypter.decrypt(&archive_data)?;

    for file in decrypted_files {
        let path = String::from_utf8_lossy(&file.path);
        let output_file_path = output_path.join(path.as_ref());

        if let Some(parent) = output_file_path.parent() {
            create_dir_all(parent)?;
        }

        write(output_file_path, file.data)?;
    }

    Ok(())
}

fn encrypt_path(
    input_path: &PathBuf,
    output_path: Option<&PathBuf>,
    engine: &str,
) -> Result<()> {
    if !input_path.exists() {
        bail!("Input path does not exist.");
    }

    let output_path = output_path.unwrap_or(input_path);
    let output_file = output_path.join("Game").with_extension(match engine {
        "xp" => "rgssad",
        "vx" => "rgss2a",
        "vxace" => "rgss3a",
        _ => unreachable!(),
    });

    if output_file.exists() {
        let filename = output_file.file_name().unwrap();
        let mut input = String::with_capacity(4);

        println!(
            "{} already exists. Overwrite it? Input 'Y' to continue.",
            filename.display()
        );
        stdin().read_line(&mut input)?;

        if input.trim_end() != "Y" {
            return Ok(());
        }
    }

    let mut archive_entries: Vec<ArchiveEntry> = Vec::with_capacity(128);

    for dir in ENCRYPT_DIRS {
        let subdir = input_path.join(dir);

        if !subdir.is_dir() || !subdir.exists() {
            continue;
        }

        let entries = WalkDir::new(&subdir).into_iter().flatten();

        for entry in entries {
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let relative_path = path.strip_prefix(".").unwrap();
            let data = read(path)?;
            archive_entries.push(ArchiveEntry {
                path: Cow::Owned(
                    relative_path.as_os_str().as_encoded_bytes().to_vec(),
                ),
                data,
            });
        }
    }

    if archive_entries.is_empty() {
        bail!(
            "No valid directories found (Data, Graphics). Nothing to encrypt."
        );
    }

    let mut decrypter = Decrypter::new();
    let archive = decrypter.encrypt(
        &archive_entries,
        match engine {
            "xp" | "vx" => Engine::Older,
            "vxace" => Engine::VXAce,
            _ => unreachable!(),
        },
    );

    write(output_file, archive)?;
    Ok(())
}

fn main() -> Result<()> {
    let start_time = Instant::now();
    let cli = Cli::parse();

    match cli.command {
        Command::Decrypt {
            input_path,
            output_path,
        } => decrypt_path(input_path, output_path)?,

        Command::Encrypt {
            input_path,
            output_path: output_file,
            engine,
        } => encrypt_path(&input_path, output_file.as_ref(), &engine)?,
    }

    println!("Elapsed: {:.2}s", start_time.elapsed().as_secs_f32());
    Ok(())
}
