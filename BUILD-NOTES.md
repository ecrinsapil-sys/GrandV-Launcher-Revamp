# Build notes

## Why this repository does not build on its own

The launcher crate is called `gta-browser-launcher`, and its library half is the
upstream code: HTTP server, LAN discovery, relay, storage, TLS, tunnel and
diagnostics. This repository holds the **UI half** — the four modules that were
rewritten — plus build configuration and packaging files.

`src/ui.rs` and `src/main.rs` refer to that library through
`gta_browser_launcher::…`, so a Rust compilation needs `src/lib.rs` and the
upstream module files next to them:

```
src/lib.rs        pub mod config; pub mod diagnostics; pub mod discovery;
                  pub mod relay; pub mod server; pub mod storage;
                  pub mod tls; pub mod tunnel;
                  pub const APP_NAME: &str = "GrandV Back end launcher";
src/config.rs     src/discovery.rs  src/relay.rs    src/storage.rs  src/tunnel.rs
src/server.rs     src/diagnostics.rs  src/tls.rs
```

Those files are upstream code and are deliberately **not** copied into this
repository. Without them the crate cannot link, and `cargo` reports what you saw
when `src/` was empty:

```
failed to parse manifest at `.../Cargo.toml`
  no targets specified in the manifest
```

## Applying this UI to a launcher checkout

1. Clone or fork the upstream launcher (its `LauncherApp/rust` directory).
2. Copy these files over the originals:

   ```
   src/theme.rs   (new)
   src/errors.rs  (new)
   src/ui.rs      (replaced)
   src/main.rs    (replaced)
   build.rs       (replaced, adds the icon fallback)
   ```

   `src/lib.rs` does not change: `theme` and `errors` are modules of the binary,
   not of the library.
3. Copy `.github/workflows/main.yml` into `.github/workflows/`.
4. Put an icon at `LauncherApp/rust/assets/app.png` — any square PNG with visible
   artwork works, since `build.rs` crops the transparent border itself. Without
   it the build still succeeds and uses a blank icon.
5. Build:

   ```
   cd LauncherApp/rust
   cargo test --all-targets -- --test-threads=1
   cargo build --release
   ```

## Seeing the interface before pushing

The Windows job runs the GUI with `--ui-smoke` and saves a screenshot:

```
cargo run -- --ui-smoke --no-open --ui-screenshot ui-smoke.png
```

On Linux without a display, wrap it in `xvfb-run -a`.

## Why there is no Cargo.lock here

The lock file belongs to the full dependency graph of the launcher crate, and
that graph includes upstream sources this repository does not carry. A lock
copied in here would pin versions the UI work does not depend on and would make
`--locked` builds fail. The workflow therefore runs plain `cargo test` and
`cargo build`.
