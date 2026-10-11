#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use encoding_rs::Encoding;
use rpgmad_lib::{ArchiveEntry, Decrypter, Engine};
use std::{
    borrow::Cow,
    collections::HashSet,
    ffi::OsStr,
    fs::{create_dir_all, read, read_dir, write},
    io::stdin,
    path::{Component, Path, PathBuf},
    time::Instant,
};
use walkdir::WalkDir;

const STANDARD_ENCRYPT_DIRS: &[&str] = &["audio", "data", "fonts", "graphics"];

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

        /// Output directory (created if missing). Defaults to the archive's directory if not set.
        #[arg(short, long, value_name = "OUTPUT_PATH")]
        output_path: Option<PathBuf>,

        /// Files to decrypt. When this is set, only specified files will be decrypted. This argument should be a list of files delimited by colon, e.g. "Data/Actors.rvdata2:Graphics/Animations/Animation1.png". Both `/` and `\` separators work.
        #[arg(short, long, value_name = "FILENAMES", value_delimiter = ':')]
        decrypt_files: Vec<String>,

        /// Encoding of the file names inside the archive. Entries whose names aren't valid in it are skipped with a warning, so for archives with non-ASCII names try `shift_jis`.
        #[arg(long, value_name = "ENCODING", default_value = "utf-8", value_parser = parse_encoding)]
        encoding: &'static Encoding,
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

        /// Output .rgss file, or an existing directory to write `Game.<ext>` into. Defaults to `input_path`.
        #[arg(short, long, value_name = "OUTPUT_PATH")]
        output_path: Option<PathBuf>,

        /// Engine to produce proper archive
        #[arg(short, long, value_name = "ENGINE", value_parser = ["xp", "vx", "vxace"])]
        engine: String,

        /// Directories to encrypt, separated by comma.
        #[arg(short = 'E', long, value_name = "ENCRYPT_DIRS", value_parser = ["audio", "data", "fonts", "graphics"], value_delimiter = ',', default_value = "data,graphics")]
        encrypt_dirs: Vec<String>,

        /// Additional directories to encrypt outside the standard `encrypt_dirs` ones, separated by comma.
        #[arg(
            long,
            value_name = "ADDITIONAL_ENCRYPT_DIRS",
            value_delimiter = ','
        )]
        additional_encrypt_dirs: Option<Vec<String>>,

        /// Encoding to store the file names in, e.g. `shift_jis`. Older XP/VX games made on Japanese systems expect it for non-ASCII names.
        #[arg(long, value_name = "ENCODING", default_value = "utf-8", value_parser = parse_encoding)]
        encoding: &'static Encoding,
    },
}

/// Accepts any WHATWG encoding label that `encoding_rs` can also encode to (so not UTF-16).
fn parse_encoding(label: &str) -> Result<&'static Encoding, String> {
    Encoding::for_label(label.as_bytes())
        .filter(|encoding| encoding.output_encoding() == *encoding)
        .ok_or_else(|| format!("unsupported encoding `{label}`"))
}

/// [`None`] if `bytes` isn't valid in `encoding`.
fn decode_path<'a>(bytes: &'a [u8], encoding: &'static Encoding) -> Option<Cow<'a, str>> {
    encoding.decode_without_bom_handling_and_without_replacement(bytes)
}

/// Whether `path` stays inside whatever directory it is joined to.
fn is_contained(path: &str) -> bool {
    !path.is_empty()
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn execute_decrypt(
    mut input_path: PathBuf,
    output_path: Option<PathBuf>,
    mut wanted_files: HashSet<String>,
    encoding: &'static Encoding,
) -> Result<()> {
    if !input_path.exists() {
        bail!("Input path does not exist.");
    }

    let output_path = output_path.unwrap_or_else(|| {
        if input_path.is_file() {
            // A bare file name has an empty parent, which is not a usable directory.
            input_path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or(Path::new("."))
                .to_path_buf()
        } else {
            input_path.clone()
        }
    });

    create_dir_all(&output_path)?;

    // Detect .rgss file inside folder
    if input_path
        .extension()
        .and_then(OsStr::to_str)
        .is_none_or(|ext| !ext.starts_with("rgss"))
    {
        let mut archives: Vec<PathBuf> = read_dir(&input_path)
            .with_context(|| {
                format!(
                    "{} is neither an .rgss archive nor a directory.",
                    input_path.display()
                )
            })?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .and_then(OsStr::to_str)
                    .is_some_and(|ext| ext.starts_with("rgss"))
            })
            .collect();

        // read_dir order is unspecified, sort so that the same directory always picks the same archive.
        archives.sort();

        if archives.len() > 1 {
            let names: Vec<_> = archives
                .iter()
                .filter_map(|path| path.file_name())
                .map(|name| name.to_string_lossy())
                .collect();

            eprintln!(
                "Warning: found multiple archives ({}). To pick another one, pass it to --input-path.",
                names.join(", ")
            );
        }

        input_path = archives
            .into_iter()
            .next()
            .context("No .rgss archive found in the directory.")?;

        println!("Using archive: {}", input_path.display());
    }

    let mut decrypter = Decrypter::new();
    let mut archive_data = read(&input_path)?;
    let decrypted_files = decrypter.decrypt(&mut archive_data)?;
    let selecting = !wanted_files.is_empty();
    let mut skipped: usize = 0;

    for file in decrypted_files {
        // Never write a mangled name: it could collide with another entry and overwrite it.
        let Some(decoded) = decode_path(file.path, encoding) else {
            eprintln!(
                "Warning: skipped an entry, its name isn't valid {}: {}",
                encoding.name(),
                String::from_utf8_lossy(file.path)
            );
            skipped += 1;
            continue;
        };

        // Archives store paths with backslashes. Forward slashes work on every platform, including Windows.
        let path = if decoded.contains('\\') {
            Cow::Owned(decoded.replace('\\', "/"))
        } else {
            decoded
        };

        if selecting && !wanted_files.remove(&*path) {
            continue;
        }

        // The archive is untrusted: an absolute or `..` path would otherwise write outside `output_path`.
        if !is_contained(&path) {
            bail!("Archive entry {path:?} points outside the output directory.");
        }

        let output_file_path = output_path.join(&*path);

        if let Some(parent) = output_file_path.parent() {
            create_dir_all(parent)?;
        }

        write(&output_file_path, file.data)
            .with_context(|| format!("Failed to write {}", output_file_path.display()))?;
    }

    for missing in &wanted_files {
        eprintln!("{missing} was not found in the archive.");
    }

    if skipped > 0 {
        eprintln!(
            "Warning: {skipped} entries were skipped because their names aren't valid {}. Run again with a different `--encoding`, e.g. `--encoding shift_jis`.",
            encoding.name()
        );
    }

    Ok(())
}

fn execute_encrypt<'a>(
    input_path: &Path,
    output_path: Option<&Path>,
    engine: &str,
    encrypt_dirs: impl Iterator<Item = &'a String>,
    encoding: &'static Encoding,
) -> Result<()> {
    if !input_path.exists() {
        bail!("Input path does not exist.");
    }

    let archive_name = match engine {
        "xp" => "Game.rgssad",
        "vx" => "Game.rgss2a",
        "vxace" => "Game.rgss3a",
        _ => unreachable!(),
    };

    let output_file = match output_path {
        Some(path) if !path.is_dir() => path.to_path_buf(),
        Some(path) => path.join(archive_name),
        None => input_path.join(archive_name),
    };

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

    // (archive path, content)
    let mut files: Vec<(Vec<u8>, Vec<u8>)> = Vec::with_capacity(128);

    for dir in encrypt_dirs {
        // Uppercase the first character in lowercased directory name, if it's standard.
        // Non-standard should be correctly input by user.
        let mut dir_name = dir.clone();
        if STANDARD_ENCRYPT_DIRS.contains(&dir.as_str()) {
            dir_name[..1].make_ascii_uppercase();
        }

        let subdir = input_path.join(&dir_name);

        if !subdir.is_dir() {
            println!(
                "{} is not a directory. It won't be encrypted.",
                subdir.display()
            );
            continue;
        }

        for entry in WalkDir::new(&subdir).sort_by_file_name() {
            let entry = entry?;

            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();

            // `subdir` is `input_path.join(..)`, so every walked path starts with `input_path`.
            let relative_path = path.strip_prefix(input_path).unwrap();

            // RPG Maker archives use backslashes.
            let archive_path = relative_path
                .to_str()
                .with_context(|| format!("{} is not valid Unicode.", path.display()))?
                .replace('/', "\\");

            let (encoded_path, _, unmappable) = encoding.encode(&archive_path);

            if unmappable {
                bail!(
                    "{} can't be represented in {}. Pick another `--encoding`.",
                    path.display(),
                    encoding.name()
                );
            }

            let data = read(path).with_context(|| format!("Failed to read {}", path.display()))?;
            files.push((encoded_path.into_owned(), data));
        }
    }

    if files.is_empty() {
        bail!(
            "No valid directories found (Data, Graphics). Nothing to encrypt."
        );
    }

    let archive_entries: Vec<ArchiveEntry> = files
        .iter()
        .map(|(path, data)| ArchiveEntry { path, data })
        .collect();

    let engine = match engine {
        "xp" | "vx" => Engine::Older,
        "vxace" => Engine::VXAce,
        _ => unreachable!(),
    };

    // Zeroed allocation instead of `resize`: no memset over a buffer that is overwritten anyway.
    let mut archive_buffer = vec![0u8; Decrypter::encrypted_buffer_size(&archive_entries, engine)];
    Decrypter::new().encrypt(&archive_entries, engine, &mut archive_buffer)?;

    if let Some(parent) = output_file.parent() {
        create_dir_all(parent)?;
    }

    write(output_file, archive_buffer)?;
    Ok(())
}

fn main() -> Result<()> {
    let start_time = Instant::now();
    let cli = Cli::parse();

    match cli.command {
        Command::Decrypt {
            input_path,
            output_path,
            decrypt_files,
            encoding,
        } => execute_decrypt(
            input_path,
            output_path,
            decrypt_files
                .into_iter()
                .map(|file| file.replace('\\', "/"))
                .collect(),
            encoding,
        )?,

        Command::Encrypt {
            input_path,
            output_path,
            engine,
            encrypt_dirs,
            additional_encrypt_dirs,
            encoding,
        } => execute_encrypt(
            &input_path,
            output_path.as_deref(),
            &engine,
            encrypt_dirs
                .iter()
                .chain(additional_encrypt_dirs.iter().flatten()),
            encoding,
        )?,
    }

    println!("Elapsed: {:.2}s", start_time.elapsed().as_secs_f32());
    Ok(())
}
