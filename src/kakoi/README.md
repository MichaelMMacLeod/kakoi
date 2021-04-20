# Documentation

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

