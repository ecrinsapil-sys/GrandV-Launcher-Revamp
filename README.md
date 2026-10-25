# GrandV Launcher — Revamp

A reworked interface for the GrandV desktop backend launcher (Rust + egui).

This repository was created to hold **only the launcher's UI work**: the tabbed
interface, the game-folder picker, the structured error reporting and the unified
control/token scale. It contains no game data, no mirrors and no proprietary
assets of any kind.

## What changed

### 1. New interface

The single scrolling column was replaced by a fixed header plus four tabs.

| Area | Contents |
|---|---|
| Header | App title, game-folder picker, folder validation, primary actions, status pill |
| Game | Play-on-this-computer guidance, Join-a-server form, LAN discovery results |
| Server | Audience, multiplayer, public hosting, advanced server settings |
| Network | Every address the running server offers, share link, certificate fingerprint |
| Help | Error-handling explanation, common fixes, full guide, project links |

### 2. Game folder selection

The folder picker is now pinned at the top of the window instead of being buried
in the hosting section. It has a text field, a `Browse…` button, a `Verify`
button and a live validation row:

- `mirror/playgta5.com/launcher.html found — the game can be hosted from here.`
- `No game found in this folder. Press Browse… and select the extracted GrandV-assets folder.`

Primary actions are only enabled when the folder actually resolves, so the
launcher cannot be started against a folder that cannot work.

### 3. Detailed errors with their fixes

Failures no longer collapse into one line of text. Every failure becomes a
`Fault` carrying:

- **What happened** — a one-line summary on the status pill
- **Likely cause** — the most probable reason, in plain language
- **Technical detail** — the full error chain, verbatim, selectable
- **How to fix it** — a numbered list of concrete steps

Known failures are classified and given a tailored fix list, with a one-click
affordance where one applies:

| Failure | Fix list | Button |
|---|---|---|
| Wrong folder | Choose the correct game folder | Browse for game folder |
| Incomplete asset package | Download or extract again | Open asset help |
| Port already in use | Use a free port | Go to advanced settings |
| cloudflared missing | Install cloudflared | Copy install command |
| Tunnel timeout | The tunnel did not come up | — |
| Certificate expired or mismatched | Renew the LAN certificate | Go to certificate settings |
| Blocked by OS or network | Allow through the firewall | — |
| Settings file unreadable | The settings file could not be used | — |
| No usable interface | No usable network interface | — |
| Anything else | What to try next | Open console output |

`Copy full report` copies summary, cause, detail and steps together, so a
support request contains everything needed on the first message.

### 4. One control scale

`src/theme.rs` is the single source of truth for every colour, spacing value and
size. Screens never hard-code pixels or colours:

- `CONTROL_H` — every button and text field is the same height
- `CONTROL_H_PRIMARY` — primary actions are taller so the eye lands on them
- `SPACE_XS` … `SPACE_XL` — the whole spacing scale
- One palette with semantic status colours (`OK`, `WARN`, `DANGER`, `INFO`)
- One type scale, applied through `egui::TextStyle` instead of scattered font sizes

A consistency test asserts that every remedy offers at least one concrete step.

## Repository layout

The launcher is a **desktop** program. It builds to a Windows `.exe`, a macOS
`.app` bundle and a Linux binary — not an Android APK.

```
LauncherApp/
  rust/
    src/
      main.rs        entry point, CLI, window setup
      ui.rs          all screens and windows
      theme.rs       colours, sizes, spacing, drawing helpers
      errors.rs      fault classification and fix lists
    build.rs         icon generation from assets/app.png
    packaging/       Info.plist and .desktop file
.github/workflows/launcher-build.yml
```

The new modules are self-contained. The rest of the launcher — HTTP server, LAN
discovery, relay, storage, TLS, tunnel, diagnostics — is used exactly as before
through its existing public API (`gta_browser_launcher::*`).

### Dependency on the upstream crate

This repository deliberately does **not** vendor the upstream launcher source.
`src/main.rs` and `src/ui.rs` depend on the crate `gta-browser-launcher`, which
must sit next to this crate's `src` directory as a sibling module path, exactly
as the upstream `LauncherApp/rust` treats it.

**To build:** copy `src/theme.rs`, `src/errors.rs`, `src/ui.rs` and `src/main.rs`
from here into `LauncherApp/rust/src/` of your own working copy of the launcher,
then build that crate normally. See `BUILD-NOTES.md`.

## Building with GitHub Actions

`.github/workflows/launcher-build.yml` builds four targets:

- `windows-x64`
- `macos-apple-silicon`
- `macos-intel`
- `linux-x64`

Run it from **Actions → Build launcher → Run workflow**. Windows performs a GUI
smoke test with a screenshot before packaging, so a broken interface fails the
build instead of shipping.

Every artifact is uploaded, including the smoke screenshots, so you can inspect
the rendered UI straight from the run.

## Disclaimer

Provided for educational purposes only. This repository distributes no game
data and no proprietary assets. Grand Theft Auto V and all related trademarks
belong to their respective owners. This project is not affiliated with,
endorsed by, or connected to Rockstar Games or Take-Two Interactive.
