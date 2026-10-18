# rpgm-archive-decrypter

RPGM Archive Decrypter is a [RPG Maker Decrypter](https://github.com/uuksu/rpgmakerdecrypter) [rewrite in Rust](https://github.com/savannstm/rpgm-archive-decrypter-lib).

It extracts and creates encrypted archives of RPG Maker XP/VX/VXAce game engines. Faster and lighter than RPG Maker Decrypter.

## Installation

Binaries are in **Releases**. They require **[x86-64-v2](https://en.wikipedia.org/wiki/X86-64#Microarchitecture_levels)** support.

## Usage

Call `rpgmad -h` for help.

### Decryption

Extract an archive beside it: `rpgmad decrypt -i C:/Game/Archive.rgssad`.

The file name can be omitted; the archive is found automatically: `rpgmad decrypt -i C:/Game`, or just `rpgmad decrypt` inside the game directory.

Use `-o` to extract elsewhere (created if missing), and `-d` to extract only some files: `rpgmad decrypt -d "Data/Actors.rvdata2:Graphics/Titles/Title.png"`.

Recognized extensions: `rgssad`, `rgss2a`, `rgss3a`.

If a directory contains several archives, `rpgmad` warns, lists them and uses the first by name; the one in use is always printed. Pass the archive path to `-i` to choose.

#### File name encoding

File names are read as UTF-8 by default. Entries with invalid UTF-8 names are skipped with a warning, never written under a mangled name. Archives made on Japanese systems usually need `--encoding shift_jis`. Any [WHATWG label](https://encoding.spec.whatwg.org/#names-and-labels) works, except UTF-16.

### Encryption

`rpgmad` encrypts the `Data` and `Graphics` directories into an archive by default. Change them with `--encrypt-dirs` (`audio`, `data`, `fonts`, `graphics`), and add other folders, relative to the input path, with `--additional-encrypt-dirs`.

`--engine` (`xp`, `vx` or `vxace`) is required: it selects the archive format.

For example: `rpgmad encrypt -i C:/Game -e vxace` writes `Game.rgss3a`. Inside the game directory, `rpgmad encrypt -e vxace` is enough.

File names are stored as UTF-8 unless `--encoding` says otherwise, e.g. `--encoding shift_jis` for XP/VX games. A name the encoding cannot represent is an error.

`-o` takes an output archive path or an existing directory to put `Game.<ext>` into. The format limits archives to 4 GiB.

If the archive already exists, `rpgmad` asks before replacing it; type `Y` to continue.

## GUI

[rpgmdec](https://github.com/rpg-maker-translation-tools/rpgmdec) provides the same functionality as `rpgmad` in a GUI.

## Development

### Building

You need Rust `1.88`.

```bash
git clone https://github.com/savannstm/rpgm-archive-decrypter
cd rpgm-archive-decrypter
cargo build --release
```

The binary is `target/release/rpgmad` (`rpgmad.exe` on Windows).

### Features

| Feature    | Default | Effect                                |
| ---------- | ------- | ------------------------------------- |
| `mimalloc` | yes     | Uses the mimalloc allocator.          |

Disable the default to use the system allocator:

```bash
cargo build --release --no-default-features
```

## Support

[Me](https://github.com/savannstm), the maintainer of this project, is a poor college student from Eastern Europe.

If you could, please consider supporting us through:

- [Boosty](https://boosty.to/mcdeimos) (has lower fees)
- [Ko-fi](https://ko-fi.com/savannstm)

Even if you don't, we will still continue working on the project anyways.

## License

Licensed under [WTFPL](https://www.wtfpl.net/).
