# rpgm-archive-decrypter

RPGM Archive Decrypter is a [RPG Maker Decrypter](https://github.com/uuksu/rpgmakerdecrypter) [rewrite in Rust](https://github.com/savannstm/rpgm-archive-decrypter-lib) (**_BLAZINGLY FAST_** :fire:).

It can be used to extract encrypted archives of RPG Maker XP/VX/VXAce game engines.

It is faster and lighter than RPG Maker Decrypter, and also has **NO** requirements to run, except a working PC.

_And also features much more cleaner code!_

## Installation

Get required binaries in **Releases** section.

## Usage

Call `rpgmad -h` for help.

### Decryption

For example, to extract archive to same same directory where it exists:
`rpgmad decrypt -i C:/Game/Archive.rgssad`.

You can omit the file name, program will find it automatically: `rpgmad decrypt -i C:/Game`.

Or just `rpgmad decrypt` if you're already in the game directory.

Use `-o` to extract elsewhere (the directory is created if missing), and `-d` to extract only some files: `rpgmad decrypt -d "Data/Actors.rvdata2:Graphics/Titles/Title.png"`.

You can recongnize archives by their extensions: `rgssad`, `rgss2a`, `rgss3a`.

If a directory contains several archives, `rpgmad` warns, lists them and uses the first by name; the one in use is always printed. Pass the archive path to `-i` to choose.

#### File name encoding

File names inside archives are read as UTF-8 by default. Entries with names that are not valid UTF-8 are skipped with a warning (never written under a mangled name). Older archives made on Japanese systems usually need `--encoding shift_jis`; any [WHATWG label](https://encoding.spec.whatwg.org/#names-and-labels) except UTF-16 works.

### Encryption

`rpgmad` encrypts entries from `Data` and `Graphics` directories (default behavior, directories can be altered using `--encrypt-dirs` and `--additional-encrypt-dirs` arguments) back to the archives.

Encryption requires an `--engine` argument (`xp`, `vx` or `vxace`) for proper encryption to the correct archive.

For example, to encrypt Data/Graphics directories to a `Game.rgss3a` archive: `rpgmad encrypt -i C:/Game -e vxace`.

Or just `rpgmad encrypt -e vxace` if you're already in the game directory.

File names are stored as UTF-8 by default, use `--encoding shift_jis` for XP/VX games that expect it. A name that does not fit the encoding is an error.

`-o` accepts either the output archive path or an existing directory to put `Game.<ext>` into. Archives are limited to 4 GiB by the format.

## GUI

Our [rpgmdec](https://github.com/rpg-maker-translation-tools/rpgmdec) GUI provides the same functionality as `rpgmad`.

## Development

### Building

Requirements: `rustup` with installed Rust toolchain.

Clone the repository and compile with `cargo b -r`.

### Tests

I'm not really skilled in tests, but the validity of output files is tested the following ways:

- Of images, using image viewers.
- Of rx/rvdata files, using [rvpacker-txt-rs](https://github.com/savannstm/rvpacker-txt-rs).

As long as these tests succesful, there shouldn't be any bugs.

## Support

[Me](https://github.com/savannstm), the maintainer of this project, is a poor college student from Eastern Europe.

If you could, please consider supporting us through:

- [Boosty](https://boosty.to/mcdeimos) (has lower fees)
- [Ko-fi](https://ko-fi.com/savannstm)

Even if you don't, we will still continue working on the project anyways.

## License

Project is licensed under [WTFPL](https://www.wtfpl.net/).
