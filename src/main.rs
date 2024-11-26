use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};
use rayon::prelude::*;
use std::{
    cell::UnsafeCell,
    fs::{create_dir_all, read, write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Instant,
};
#[derive(PartialEq, Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
enum EngineType {
    XPVX,
    VXAce,
}

impl std::fmt::Display for EngineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let variant_name: &str = match self {
            EngineType::XPVX => "XP/VX",
            EngineType::VXAce => "VXAce",
        };

        write!(f, "{}", variant_name)
    }
}

#[derive(Default)]
struct VecWalker {
    data: Vec<u8>,
    pos: usize,
    len: usize,
}

enum SeekFrom {
    Start,
    Current,
}

impl VecWalker {
    pub fn new(data: Vec<u8>) -> Self {
        let len: usize = data.len();
        VecWalker { data, pos: 0, len }
    }

    pub fn advance(&mut self, bytes: usize) -> &[u8] {
        let start: usize = self.pos;
        self.pos += bytes;
        &self.data[start..self.pos]
    }

    pub fn read_chunk(&mut self) -> [u8; 4] {
        let read: &[u8] = self.advance(4);
        unsafe { *(read.as_ptr() as *const [u8; 4]) }
    }

    pub fn read_byte(&mut self) -> u8 {
        let byte: u8 = self.data[self.pos];
        self.pos += 1;
        byte
    }

    pub fn seek(&mut self, offset: usize, seek_from: SeekFrom) {
        self.pos = match seek_from {
            SeekFrom::Start => offset,
            SeekFrom::Current => self.pos + offset,
        };
    }
}

struct Localization<'a> {
    // Arg descriptions
    input_path_arg_desc: &'a str,
    output_path_arg_desc: &'a str,
    force_arg_desc: &'a str,
    help_arg_desc: &'a str,
    about: &'a str,

    // Messages
    input_path_missing_msg: &'a str,
    output_path_missing_msg: &'a str,
    unknown_engine_type_msg: &'a str,
    unknown_archive_header_msg: &'a str,
    output_files_already_exists_msg: &'a str,
}

struct Archive {
    name: String,
    size: i32,
    offset: usize,
    key: u32,
}

struct Decrypter<'a> {
    walker: UnsafeCell<VecWalker>,
    key: u32,
    engine: EngineType,
    localization: Localization<'a>,
}

impl<'a> Decrypter<'a> {
    fn new(bytes: Vec<u8>, localization: Localization<'a>) -> Self {
        Self {
            walker: UnsafeCell::new(VecWalker::new(bytes)),
            key: 0xDEADCAFE,
            engine: EngineType::XPVX,
            localization,
        }
    }

    fn extract(&mut self, output_path: &Path, force: bool) {
        let version: u8 = self.get_archive_version();

        if version == 1 {
            self.engine = EngineType::XPVX
        } else if version == 3 {
            self.engine = EngineType::VXAce
        } else {
            panic!("{}", self.localization.unknown_engine_type_msg)
        }

        let archives: Vec<Archive> = self.read_archive();

        let walker: &mut VecWalker = unsafe { &mut *self.walker.get() };
        let arc: Arc<Mutex<&mut VecWalker>> = Arc::new(Mutex::new(walker));

        archives.into_par_iter().for_each(|archive: Archive| {
            let output_path: PathBuf = output_path.join(archive.name);

            if output_path.exists() && !force {
                println!("{}", self.localization.output_files_already_exists_msg);
                return;
            }

            let mut walker = arc.lock().unwrap();

            walker.seek(archive.offset, SeekFrom::Start);

            let mut data: Vec<u8> = Vec::with_capacity(archive.size as usize);
            data.extend_from_slice(walker.advance(archive.size as usize));

            drop(walker);

            create_dir_all(unsafe { output_path.parent().unwrap_unchecked() }).unwrap();
            write(output_path, Self::decrypt_archive(&data, archive.key)).unwrap();
        });
    }

    fn decrypt_archive(data: &[u8], mut key: u32) -> Vec<u8> {
        let mut decrypted: Vec<u8> = Vec::with_capacity(data.len());

        let mut key_bytes: [u8; 4] = key.to_le_bytes();
        let mut j: usize = 0;

        for item in data {
            if j == 4 {
                j = 0;
                key = key.wrapping_mul(7).wrapping_add(3);
                key_bytes = key.to_le_bytes();
            }

            decrypted.push(item ^ key_bytes[j]);
            j += 1;
        }

        decrypted
    }

    fn get_archive_version(&mut self) -> u8 {
        let walker: &mut VecWalker = unsafe { &mut *self.walker.get() };
        let header: &[u8] = walker.advance(6);

        if header != b"RGSSAD" {
            panic!("{}", self.localization.unknown_archive_header_msg);
        }

        walker.seek(1, SeekFrom::Current);
        let version: u8 = walker.read_byte();

        walker.seek(0, SeekFrom::Start);

        version
    }

    fn decrypt_integer(&mut self, value: i32) -> i32 {
        let result: i32 = value ^ self.key as i32;

        if self.engine == EngineType::XPVX {
            self.key = self.key.wrapping_mul(7).wrapping_add(3);
        }

        result
    }

    fn decrypt_filename(&mut self, filename: &[u8]) -> String {
        let mut decrypted: Vec<u8> = Vec::with_capacity(filename.len());

        if self.engine == EngineType::VXAce {
            let key_bytes: [u8; 4] = self.key.to_le_bytes();
            let mut j: usize = 0;

            for item in filename {
                if j == 4 {
                    j = 0;
                }

                decrypted.push(item ^ key_bytes[j]);
                j += 1;
            }
        } else {
            for item in filename {
                decrypted.push(item ^ (self.key & 0xff) as u8);
                self.key = self.key.wrapping_mul(7).wrapping_add(3);
            }
        }

        String::from_utf8(decrypted).unwrap()
    }

    fn read_archive(&mut self) -> Vec<Archive> {
        let walker: &mut VecWalker = unsafe { &mut *self.walker.get() };
        walker.seek(8, SeekFrom::Start);

        if self.engine == EngineType::VXAce {
            self.key = u32::from_le_bytes(walker.read_chunk())
                .wrapping_mul(9)
                .wrapping_add(3);
        }

        let mut archives: Vec<Archive> = Vec::with_capacity(1024);

        loop {
            let (name, size, offset, key) = if self.engine == EngineType::VXAce {
                let offset: usize =
                    self.decrypt_integer(i32::from_le_bytes(walker.read_chunk())) as usize;

                let size: i32 = self.decrypt_integer(i32::from_le_bytes(walker.read_chunk()));

                let key: u32 = self.decrypt_integer(i32::from_le_bytes(walker.read_chunk())) as u32;

                let length: i32 = self.decrypt_integer(i32::from_le_bytes(walker.read_chunk()));

                if offset == 0 {
                    break;
                }

                let name: String = self.decrypt_filename(walker.advance(length as usize));

                (name, size, offset, key)
            } else {
                let length: i32 = self.decrypt_integer(i32::from_le_bytes(walker.read_chunk()));

                let name: String = self.decrypt_filename(walker.advance(length as usize));

                let size: i32 = self.decrypt_integer(i32::from_le_bytes(walker.read_chunk()));

                let offset: usize = walker.pos;

                let key: u32 = self.key;

                walker.seek(size as usize, SeekFrom::Current);

                if walker.pos == walker.len {
                    break;
                }

                (name, size, offset, key)
            };

            archives.push(Archive {
                name,
                size,
                offset,
                key,
            });
        }

        archives
    }
}

fn main() {
    let start_time: Instant = Instant::now();

    const LOCALIZATION: Localization = Localization {
        input_path_arg_desc: "Path to the RGSSAD file.",
        output_path_arg_desc: "Path to put output files.",
        force_arg_desc: "Forcefully overwrite existing Data, Graphics and other files.",
        help_arg_desc: "Prints the help message.",
        about: "A tool to extract encrypted .rgss RPG Maker archives.",

        input_path_missing_msg: "Input file does not exist.",
        output_path_missing_msg: "Output path does not exist.",
        unknown_archive_header_msg: "Unknown archive header. Expected: RGSSAD.",
        unknown_engine_type_msg:
            "Unknown archive game engine. Maybe, file's extension is spelled wrong?",
        output_files_already_exists_msg:
            "Output file already exists. Use --force to forcefully overwrite it.",
    };

    let input_path_arg: Arg = Arg::new("input-path")
        .short('i')
        .long("input-file")
        .help(LOCALIZATION.input_path_arg_desc)
        .value_parser(value_parser!(PathBuf))
        .default_value("./");

    let output_path_arg: Arg = Arg::new("output-path")
        .short('o')
        .long("output-dir")
        .help(LOCALIZATION.output_path_arg_desc)
        .value_parser(value_parser!(PathBuf))
        .default_value("./");

    let force: Arg = Arg::new("force")
        .short('f')
        .long("force")
        .help(LOCALIZATION.force_arg_desc)
        .action(ArgAction::SetTrue);

    let help: Arg = Arg::new("help")
        .short('h')
        .long("help")
        .help(LOCALIZATION.help_arg_desc)
        .action(ArgAction::Help);

    let cli: Command = Command::new("")
        .about(LOCALIZATION.about)
        .disable_version_flag(true)
        .disable_help_subcommand(true)
        .disable_help_flag(true)
        .next_line_help(true)
        .term_width(120)
        .args([input_path_arg, output_path_arg, force, help]);

    let matches: ArgMatches = cli.get_matches();

    let input_path: &Path = matches.get_one::<PathBuf>("input-path").unwrap();

    if !input_path.exists() {
        panic!("{}", LOCALIZATION.input_path_missing_msg)
    }

    let mut output_path: &Path = matches.get_one::<PathBuf>("output-path").unwrap();

    output_path = if *output_path.as_os_str() == *"./" {
        unsafe { input_path.parent().unwrap_unchecked() }
    } else {
        output_path
    };

    if !output_path.exists() {
        panic!("{}", LOCALIZATION.output_path_missing_msg);
    }

    let force_flag: bool = matches.get_flag("force");

    let bytes: Vec<u8> = read(input_path).unwrap();
    Decrypter::new(bytes, LOCALIZATION).extract(output_path, force_flag);

    println!("Elapsed: {}", start_time.elapsed().as_secs_f64())
}
