use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};
use rpgmad_lib::Decrypter;
use std::{
    fs::read,
    path::{Path, PathBuf},
    time::Instant,
};

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
}

fn main() {
    let start_time: Instant = Instant::now();

    const LOCALIZATION: Localization = Localization {
        input_path_arg_desc: "Path to the .rgss file.",
        output_path_arg_desc: "Path to put output files.",
        force_arg_desc: "Forcefully overwrite existing Data, Graphics and other files.",
        help_arg_desc: "Prints the help message.",
        about: "A tool to extract encrypted .rgss RPG Maker archives.",

        input_path_missing_msg: "Input file does not exist.",
        output_path_missing_msg: "Output path does not exist.",
    };

    let input_path_arg: Arg = Arg::new("input-path")
        .short('i')
        .long("input-file")
        .help(LOCALIZATION.input_path_arg_desc)
        .value_parser(value_parser!(PathBuf))
        .default_value("./")
        .hide_default_value(true);

    let output_path_arg: Arg = Arg::new("output-path")
        .short('o')
        .long("output-dir")
        .help(LOCALIZATION.output_path_arg_desc)
        .value_parser(value_parser!(PathBuf))
        .default_value("./")
        .hide_default_value(true);

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
    if *output_path.as_os_str() == *"./" {
        output_path = unsafe { input_path.parent().unwrap_unchecked() }
    }

    if !output_path.exists() {
        panic!("{}", LOCALIZATION.output_path_missing_msg);
    }

    let force_flag: bool = matches.get_flag("force");

    let bytes: Vec<u8> = read(input_path).unwrap();

    Decrypter::new(bytes)
        .extract(output_path, force_flag)
        .unwrap();

    println!("Elapsed: {}", start_time.elapsed().as_secs_f64())
}
