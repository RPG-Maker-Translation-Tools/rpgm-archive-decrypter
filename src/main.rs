use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};
use std::{
    cell::UnsafeCell,
    fs::{create_dir_all, read, write},
    mem::take,
    path::{Path, PathBuf},
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
        let len = data.len();
        VecWalker { data, pos: 0, len }
    }

    pub fn advance(&mut self, bytes: usize) -> &[u8] {
        let start: usize = self.pos;
        self.pos += bytes;
        &self.data[start..self.pos]
    }

    pub fn seek(&mut self, offset: usize, seek_from: SeekFrom) {
        let new_pos: usize = match seek_from {
            SeekFrom::Start => offset,
            SeekFrom::Current => self.pos + offset,
        };

        self.pos = new_pos;
    }
}

struct Localization<'a> {
    input_path_arg: &'a str,
    input_path_missing: &'a str,
    output_path_arg: &'a str,
    output_path_missing: &'a str,
    unknown_engine_type: &'a str,
    unknown_archive_header: &'a str,
    could_not_get_archive_header: &'a str,
    output_files_already_exists: &'a str,
    force_arg: &'a str,
}

struct ArchivedFile {
    name: String,
    size: i32,
    offset: usize,
    key: u32,
}

struct Archive<'a> {
    walker: UnsafeCell<VecWalker>,
    archived_files: Vec<ArchivedFile>,
    key: u32,
    output_path: &'a Path,
    force: bool,
    engine_type: EngineType,
    localization: Localization<'a>,
}

impl<'a> Archive<'a> {
    fn new(
        input_path: &'a Path,
        output_path: &'a Path,
        force: bool,
        localization: Localization<'a>,
    ) -> Self {
        Self {
            walker: UnsafeCell::new(VecWalker::new(read(input_path).unwrap())),
            archived_files: Vec::new(),
            key: 0xDEADCAFE,
            output_path,
            force,
            engine_type: EngineType::XPVX,
            localization,
        }
    }

    fn extract(&mut self) {
        let version: u8 = self.get_version();

        if version == 1 {
            self.engine_type = EngineType::XPVX
        } else if version == 3 {
            self.engine_type = EngineType::VXAce
        } else {
            panic!("{}", self.localization.unknown_engine_type)
        }

        self.read_archive();
        let walker: &mut VecWalker = unsafe { &mut *self.walker.get() };

        for archive in take(&mut self.archived_files) {
            let actual_output_path: PathBuf = self.output_path.parent().unwrap().join(archive.name);

            if actual_output_path.exists() && !self.force {
                println!("{}", self.localization.output_files_already_exists);
                return;
            }

            walker.seek(archive.offset, SeekFrom::Start);
            let data: &[u8] = walker.advance(archive.size as usize);

            create_dir_all(actual_output_path.parent().unwrap()).unwrap();
            write(actual_output_path, self.decrypt_archive(data, archive.key)).unwrap();
        }
    }

    fn decrypt_archive(&mut self, data: &[u8], mut key: u32) -> Vec<u8> {
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

    fn get_version(&mut self) -> u8 {
        let walker: &mut VecWalker = unsafe { &mut *self.walker.get() };

        if let Ok(header) = String::from_utf8(unsafe { (*self.walker.get()).advance(6).to_vec() }) {
            if header != "RGSSAD" {
                panic!("{}", self.localization.unknown_archive_header);
            }
        } else {
            panic!("{}", self.localization.could_not_get_archive_header);
        }

        let version: u8 = *unsafe { walker.advance(2).last().unwrap_unchecked() };
        walker.seek(0, SeekFrom::Start);

        version
    }

    fn decrypt_integer(&mut self, value: i32) -> i32 {
        let result: i32 = value ^ self.key as i32;

        if self.engine_type != EngineType::VXAce {
            self.key = self.key.wrapping_mul(7).wrapping_add(3);
        }

        result
    }

    fn decrypt_filename(&mut self, filename: &[u8]) -> String {
        let mut decrypted: Vec<u8> = Vec::with_capacity(filename.len());

        if self.engine_type == EngineType::VXAce {
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

    fn read_archive(&mut self) {
        let walker: &mut VecWalker = unsafe { &mut *self.walker.get() };
        walker.seek(8, SeekFrom::Start);

        if self.engine_type == EngineType::VXAce {
            self.key = u32::from_le_bytes(walker.advance(4).try_into().unwrap())
                .wrapping_mul(9)
                .wrapping_add(3);
        }

        loop {
            if self.engine_type == EngineType::VXAce {
                let offset: usize = self.decrypt_integer(i32::from_le_bytes(unsafe {
                    *(walker.advance(4).as_ptr() as *const [u8; 4])
                })) as usize;

                let size: i32 = self.decrypt_integer(i32::from_le_bytes(unsafe {
                    *(walker.advance(4).as_ptr() as *const [u8; 4])
                }));

                let key: u32 = self.decrypt_integer(i32::from_le_bytes(unsafe {
                    *(walker.advance(4).as_ptr() as *const [u8; 4])
                })) as u32;

                let length: i32 = self.decrypt_integer(i32::from_le_bytes(unsafe {
                    *(walker.advance(4).as_ptr() as *const [u8; 4])
                }));

                if offset == 0 {
                    break;
                }

                let name: String = self.decrypt_filename(walker.advance(length as usize));

                self.archived_files.push(ArchivedFile {
                    name,
                    size,
                    offset,
                    key,
                });
            }
            let length: i32 = self.decrypt_integer(i32::from_le_bytes(unsafe {
                *(walker.advance(4).as_ptr() as *const [u8; 4])
            }));

            let name: String = self.decrypt_filename(walker.advance(length as usize));

            let size: i32 = self.decrypt_integer(i32::from_le_bytes(unsafe {
                *(walker.advance(4).as_ptr() as *const [u8; 4])
            }));

            let offset: usize = walker.pos;

            let key: u32 = self.key;

            self.archived_files.push(ArchivedFile {
                name,
                size,
                offset,
                key,
            });

            walker.seek(size as usize, SeekFrom::Current);

            if walker.pos == walker.len {
                break;
            }
        }
    }
}

fn main() {
    let start_time: Instant = Instant::now();

    const LOCALIZATION: Localization = Localization {
        input_path_arg: "Path to the RGSSAD file.",
        input_path_missing: "Input file does not exist.",
        output_path_arg: "Where to put output files.",
        output_path_missing: "Output path does not exist.",
        could_not_get_archive_header: "Couldn't read archive header (first 6 bytes).",
        unknown_archive_header: "Unknown archive header. Expected: RGSSAD.",
        unknown_engine_type:
            "Unknown archive game engine. Maybe, file's extension is spelled wrong?",
        output_files_already_exists:
            "Output file already exists. Use --force to forcefully overwrite it.",
        force_arg: "Forcefully overwrite existing Data, Graphics etc. files.",
    };

    let input_path_arg: Arg = Arg::new("input-path")
        .short('i')
        .long("input")
        .help(LOCALIZATION.input_path_arg)
        .value_parser(value_parser!(PathBuf))
        .default_value("./");

    let output_path_arg: Arg = Arg::new("output-path")
        .short('o')
        .long("output")
        .help(LOCALIZATION.output_path_arg)
        .value_parser(value_parser!(PathBuf))
        .default_value("./");

    let force: Arg = Arg::new("force")
        .short('f')
        .long("force")
        .help(LOCALIZATION.force_arg)
        .action(ArgAction::SetTrue);

    let cli: Command = Command::new("")
        .disable_version_flag(true)
        .disable_help_subcommand(true)
        .disable_help_flag(true)
        .next_line_help(true)
        .term_width(120)
        .args([input_path_arg, output_path_arg, force])
        .hide_possible_values(true);

    let matches: ArgMatches = cli.get_matches();

    let input_path: &Path = matches.get_one::<PathBuf>("input-path").unwrap();

    if !input_path.exists() {
        panic!("{}", LOCALIZATION.input_path_missing)
    }

    let mut output_path: &Path = matches.get_one::<PathBuf>("output-path").unwrap();

    if !output_path.exists() {
        panic!("{}", LOCALIZATION.output_path_missing);
    }

    output_path = if *output_path.as_os_str() == *"./" {
        input_path
    } else {
        output_path
    };

    let force_flag: bool = matches.get_flag("force");

    Archive::new(input_path, output_path, force_flag, LOCALIZATION).extract();

    println!("Elapsed: {}", start_time.elapsed().as_secs_f64())
}
