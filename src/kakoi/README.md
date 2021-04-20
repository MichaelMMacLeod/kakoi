# Building

## Install Dependencies

- [rust](https://www.rust-lang.org/tools/install)
- [cmake](https://cmake.org/download/)
- [shaderc](https://github.com/google/shaderc#downloads)

On Arch Linux, these can be installed via
```
pacman -S rust cmake shaderc
```

It is in theory possible to build `kakoi` on Windows and MacOS, but I haven't
tried it yet.

## Running

After installing dependencies, `cd` into the directory containing this file and
execute `cargo run -- --create-window`. To turn on optimizations, execute `cargo
run --release -- --create-window` instead. Optimized builds make a noticeable
difference when rendering large pieces of text on screen. You can use `-c` as a
shorthand for `--create-window`.

## Building

To build the project without running it, use `cargo build`.

## Tests

To run tests, use `cargo test`. There aren't many tests yet, though.

## Use `cargo check` for faster development

During development, use `cargo check` to check for errors instead of using
`cargo build` or `cargo run` to save a lot of compilation time.

## Documentation

Use the following command to build the documentation and open it in a web
browser:

```
cargo doc --package kakoi --document-private-items --open
```

To auto-rebuild documentation on file changes, use the following command instead:

```
# if you haven't installed cargo-watch, run 'cargo install cargo-watch' first
cargo watch --shell 'cargo doc --package kakoi --document-private-items'
```

