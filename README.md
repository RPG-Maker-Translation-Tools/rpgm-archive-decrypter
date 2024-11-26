# rpgm-archive-decrypter

RPGM Archive Decrypter is a [RPG Maker Decrypter](github.com/uuksu/rpgmakerdecrypter) rewrite in Rust (**_BLAZINGLY FAST_** :fire:).
It can be used to extract encrypted archives of RPG Maker XP/VX/VXAce game engines, and generate project files for decrypted data.
It is faster and lighter than RPG Maker Decrypter, and also has **NO** requirements to run, except a working PC.

_And also features much more cleaner code!_

## Installation

Get required binaries in Releases section.
One with `.exe` extension is for Windows, without it - is for Linux.

## Usage

Call `rpgmad.exe -h` for help.

```text

```

For example, to extract archive to same same directory where it exists:
`rpgmad C:/RPGMakerGame/Archive.rgssad`.

You can recongnize archives by their extensions: `rgssad`, `rgss2a`, `rgss3a`.

## GUI

Full-featured GTK GUI built with `gtk-rs` is in progress.

## Building

Requirements: `rustup` with installed Rust toolchain, linker (`gcc`, `llvm` or `msvc`).

Clone the repository with `git` and compile with `cargo b -r`.

## License

Project is licensed under WTFPL.
