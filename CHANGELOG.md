## 7.0.0

- Encrypting now includes files reached through symbolic links instead of silently skipping them.
- Decrypting and encrypting large archives now uses much less memory.
- Archives with uppercase extensions such as `.RGSS3A` are now recognized.
- Only `.rgssad`, `.rgss2a` and `.rgss3a` files are treated as archives. Other names that start with `rgss` are no longer picked.
- `--decrypt-files` now ignores case, so `data/actors.rvdata2` finds `Data/Actors.rvdata2`.
- Fixed a failed encryption replacing an existing archive with a truncated one.
- Changed declining the overwrite prompt to exit with an error instead of succeeding silently.
- Fixed a crash when `--additional-encrypt-dirs` is given a folder outside the game folder. It now reports an error.
- Fixed files being stored twice in an archive when a folder is listed more than once.
- Fixed an existing archive being packed into the new one when encrypting a folder that contains it.
- Empty names in `--additional-encrypt-dirs`, such as a trailing comma, are now rejected instead of packing the whole game folder.
- A run that finds no archive no longer creates the output folder.

### From rpgmad-lib v7.0.0

- **Breaking:** `Decrypter` is removed. Use `decrypt` to read an archive and `encrypt` or `encrypt_to_vec` to write one.
- **Breaking:** `Engine::VXAce` is now `Engine::Vxace`, and `Engine.VXAce` is now `Engine.Vxace` in JavaScript. Rename it in your code.
- **Breaking:** The C `rpgmard_encrypt` and `rpgmard_encrypted_buffer_size` functions no longer take a scratch buffer, and the decrypter size and alignment functions are removed.
- **Breaking:** `ExtractError` and `EncryptError` are now `non_exhaustive`, and `Truncated` and `Corrupt` carry a byte offset. Update matches on them.
- Added streaming decryption and encryption for Rust, so archives can be read and written without loading them fully into memory.
- Added `encrypt_to_vec`, which returns a finished archive without sizing a buffer by hand.
- Added a way to get an engine from a file extension such as `.rgss3a`.
- Added checks for archive paths that would escape the output folder, such as `..` components or absolute paths.
- Added an `alloc` feature so `no_std` projects can use `encrypt_to_vec`.
- Added `rpgmard_status_message` to C, which describes any status code.
- Added `rpgmard_decrypter_entries_left` to C, which reports how many entries remain.
- Fixed `encryptArchive` in JavaScript returning a zero-filled buffer for archives over 4 GiB. It now throws.
- Sped up decrypting and encrypting archives.
- Archives with more than 1,048,576 entries are now rejected with an error.
