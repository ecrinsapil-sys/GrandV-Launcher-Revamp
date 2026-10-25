//! Interface for the GrandV backend launcher.
//!
//! Layout: a fixed header with the game-folder picker and the primary actions,
//! then a tab bar, then the active tab's content. Every colour and size comes
//! from `crate::theme`, and every failure is rendered through `crate::errors`.

use crate::errors::{Fault, Remedy, GUIDE};
use crate::theme::{self, Level};
use eframe::egui;
use gta_browser_launcher::{
    config::{self, Settings},
    discovery::{self, Found},
    server::{self, Running},
    tls, tunnel,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::{
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

struct ConsoleWindow {
    open: AtomicBool,
    follow: AtomicBool,
    screenshot_requested: AtomicBool,
}

enum Event {
    Started(anyhow::Result<(Running, Option<tunnel::Tunnel>)>, bool),
    Stopped,
    Found(anyhow::Result<Vec<Found>>),
    Certificate(anyhow::Result<(String, String)>),
}

// ---------------------------------------------------------------------------
// Tabs
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tab {
    Game = 0,
    Server = 1,
    Network = 2,
    Help = 3,
}

impl Tab {
    const ALL: [Tab; 4] = [Tab::Game, Tab::Server, Tab::Network, Tab::Help];

    fn label(self) -> &'static str {
        match self {
            Tab::Game => "Game",
            Tab::Server => "Server",
            Tab::Network => "Network",
            Tab::Help => "Help",
        }
    }

    fn from_index(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or(Tab::Game)
    }
}

const ASSET_TORRENT_URL: &str = concat!(
    "magnet:?xt=urn:btih:60a2d95f23154e4f7c75a965307133c24d9c7178&dn=GrandV-assets",
    "&tr=udp%3A%2F%2Ftracker.opentrackr.org%3A1337%2Fannounce",
    "&tr=udp%3A%2F%2Ftracker.torrent.eu.org%3A451",
    "&tr=http%3A%2F%2Ftracker.opentrackr.org%3A1337%2Fannounce",
    "&tr=udp%3A%2F%2Fopen.stealth.si%3A80%2Fannounce",
    "&tr=http%3A%2F%2Ftracker.qu.ax%3A6969%2Fannounce",
    "&tr=udp%3A%2F%2Ftracker.skynetcloud.site%3A6969%2Fannounce",
    "&tr=udp%3A%2F%2Ftracker.denness.net%3A6969%2Fannounce",
    "&tr=udp%3A%2F%2Ftracker.qu.ax%3A6969%2Fannounce",
    "&tr=udp%3A%2F%2Ftracker.ducks.party%3A1984%2Fannounce",
    "&tr=udp%3A%2F%2Ftracker.tryhackx.org%3A6969%2Fannounce",
    "&tr=udp%3A%2F%2Ftracker.corpscorp.online%3A80%2Fannounce",
    "&tr=udp%3A%2F%2Ftracker.bittor.pw%3A1337%2Fannounce",
    "&tr=udp%3A%2F%2Ftracker-udp.gbitt.info%3A80%2Fannounce",
    "&tr=udp%3A%2F%2Ftracker.nyaa.vc%3A6969%2Fannounce",
    "&tr=udp%3A%2F%2Ftracker.gmi.gd%3A6969%2Fannounce",
    "&tr=udp%3A%2F%2Ftracker2.dler.org%3A80%2Fannounce",
    "&tr=udp%3A%2F%2Ftracker.dler.org%3A6969%2Fannounce",
    "&tr=http%3A%2F%2Ftracker2.dler.org%3A80%2Fannounce",
    "&tr=http%3A%2F%2Ftracker.dler.com%3A6969%2Fannounce",
    "&tr=http%3A%2F%2Ftracker.renfei.net%3A8080%2Fannounce",
    "&tr=udp%3A%2F%2Ftracker.farted.net%3A6969%2Fannounce",
    "&tr=udp%3A%2F%2Ftracker.peerfect.org%3A6969%2Fannounce",
    "&tr=udp%3A%2F%2Ftracker.opentrackr.com%3A6969%2Fannounce",
    "&tr=udp%3A%2F%2Ftracker.ilibr.org%3A6969%2Fannounce",
    "&tr=udp%3A%2F%2Fmail.segso.net%3A6969%2Fannounce",
    "&tr=https%3A%2F%2Ftracker.pmman.tech%3A443%2Fannounce",
    "&tr=udp%3A%2F%2Ftracker.torrents.observer%3A80%2Fannounce",
    "&tr=udp%3A%2F%2Ftracker.nyaa.net%3A6969%2Fannounce",
    "&tr=udp%3A%2F%2Fopen.ftorrent.com%3A443%2Fannounce",
    "&tr=udp%3A%2F%2Ftracker.aruku.ovh%3A8081%2Fannounce",
);

#[cfg(windows)]
fn open_asset_torrent() -> std::io::Result<()> {
    let target: Vec<u16> = ASSET_TORRENT_URL.encode_utf16().chain(Some(0)).collect();
    let operation: Vec<u16> = "open".encode_utf16().chain(Some(0)).collect();
    // Use the magnet protocol handler, rather than the default HTTP browser.
    let result = unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    } as isize;
    if result <= 32 {
        return Err(std::io::Error::other(format!(
            "No torrent client could open the magnet link (Windows error {result})"
        )));
    }
    Ok(())
}

#[cfg(not(windows))]
fn open_asset_torrent() -> std::io::Result<()> {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    if !std::process::Command::new(program)
        .arg(ASSET_TORRENT_URL)
        .status()?
        .success()
    {
        return Err(std::io::Error::other(
            "No torrent client could open the magnet link",
        ));
    }
    Ok(())
}

pub struct Launcher {
    runtime: Option<tokio::runtime::Runtime>,
    tx: mpsc::Sender<Event>,
    rx: mpsc::Receiver<Event>,
    folder: String,
    settings: Settings,
    code: String,
    host: Option<Running>,
    tunnel: Option<tunnel::Tunnel>,
    host_addresses: Vec<std::net::Ipv4Addr>,
    busy: bool,
    status: String,
    level: Level,
    /// Set when the last failure produced a reportable fault.
    fault: Option<Fault>,
    tab: Tab,
    join: String,
    found: Vec<Found>,
    scanning: bool,
    /// The error report window is open.
    fault_open: bool,
    donate: bool,
    donation_coin: Option<String>,
    share: bool,
    updates: bool,
    obtain_assets: bool,
    wallets: serde_json::Value,
    smoke: bool,
    created: Instant,
    auto_open: bool,
    screenshot: Option<PathBuf>,
    screenshot_requested: bool,
    console: Arc<ConsoleWindow>,
    console_status: String,
}

impl Launcher {
    pub fn new(
        folder: PathBuf,
        port: Option<u16>,
        smoke: bool,
        no_open: bool,
        screenshot: Option<PathBuf>,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        let mut app = Self {
            runtime: Some(tokio::runtime::Runtime::new().expect("Create runtime")),
            tx,
            rx,
            folder: folder.to_string_lossy().into(),
            settings: Settings::default(),
            code: String::new(),
            host: None,
            tunnel: None,
            host_addresses: Vec::new(),
            busy: false,
            status: "Stopped".into(),
            level: Level::Idle,
            fault: None,
            tab: Tab::Game,
            join: String::new(),
            found: Vec::new(),
            scanning: false,
            fault_open: false,
            donate: false,
            donation_coin: smoke.then(|| "solana".into()),
            share: false,
            updates: false,
            obtain_assets: false,
            wallets: serde_json::from_str(include_str!("../../donations/wallets.json"))
                .expect("Bundled wallets JSON"),
            smoke,
            created: Instant::now(),
            auto_open: !no_open && !smoke,
            screenshot,
            screenshot_requested: false,
            console: Arc::new(ConsoleWindow {
                open: AtomicBool::new(false),
                follow: AtomicBool::new(true),
                screenshot_requested: AtomicBool::new(false),
            }),
            console_status: String::new(),
        };
        if let Some(wallets) = app.wallets.as_array_mut() {
            wallets.sort_by_key(|wallet| wallet["id"].as_str() != Some("solana"));
        }
        app.load();
        if let Some(port) = port {
            app.settings.port = port;
        }
        if smoke
            && !(app.settings.cloudflare_tunnel && app.settings.mode == "external")
            && config::resolve_folder(&PathBuf::from(&app.folder)).is_ok()
        {
            app.donate = false;
            app.start(false);
        }
        app
    }

    // -- state helpers ------------------------------------------------------

    /// Set the status line together with its severity.
    fn set_status(&mut self, level: Level, text: impl Into<String>) {
        self.level = level;
        self.status = text.into();
        if level != Level::Error {
            self.fault = None;
            self.fault_open = false;
        }
    }

    /// Record a failure: status line, full report, and open the detail window.
    fn fail(&mut self, fault: Fault) {
        self.status = fault.summary.clone();
        self.level = Level::Error;
        gta_browser_launcher::diagnostics::record("Error", &fault.detail);
        self.fault = Some(fault);
        self.fault_open = true;
    }

    /// Whether the chosen folder currently resolves to a real game folder.
    fn folder_state(&self) -> Result<PathBuf, Fault> {
        config::resolve_folder(&PathBuf::from(&self.folder))
            .map_err(|error| Fault::from_error("The game folder is not usable", &error))
    }

    fn folder_is_valid(&self) -> bool {
        config::resolve_folder(&PathBuf::from(&self.folder)).is_ok()
    }

    fn load(&mut self) {
        match config::resolve_folder(&PathBuf::from(&self.folder)) {
            Ok(folder) => {
                self.folder = folder.to_string_lossy().into();
                match Settings::load(&folder) {
                    Ok(s) => self.settings = s,
                    Err(e) => {
                        let fault = Fault::from_error("Could not read settings", &e);
                        self.fail(fault);
                    }
                }
                match config::load_code(&folder) {
                    Ok(s) => self.code = s,
                    Err(e) => {
                        let fault = Fault::from_error("Could not read the access code", &e);
                        self.fail(fault);
                    }
                }
                if self.level != Level::Error {
                    self.set_status(Level::Idle, "Ready — press Start server");
                }
            }
            Err(error) => {
                // A wrong folder is the most common first-run state, so report
                // it in place rather than as a modal failure.
                let fault = Fault::from_error("The game folder is not usable", &error);
                self.level = Level::Warn;
                self.status = fault.summary.clone();
                self.fault = None;
                self.fault_open = false;
            }
        }
    }

    // -- server control -----------------------------------------------------

    fn start(&mut self, open_game: bool) {
        if self.busy || self.host.is_some() {
            return;
        }
        if let Err(fault) = self.folder_state() {
            self.fail(fault);
            return;
        }
        self.busy = true;
        self.set_status(Level::Busy, "Starting server…");
        let tx = self.tx.clone();
        let folder = PathBuf::from(&self.folder);
        let mut settings = self.settings.clone();
        let quick_tunnel = settings.cloudflare_tunnel && settings.mode == "external";
        let code = self.code.clone();
        self.runtime.as_ref().unwrap().spawn(async move {
            let result = async {
                let previous = settings.clone();
                if quick_tunnel {
                    settings.https = false;
                    settings.public_origin = "https://pending.invalid".into();
                }
                // Own the listener before publishing a tunnel, so a busy port cannot expose another app.
                let mut host = server::start(&folder, settings, &code).await?;
                let tunnel = if quick_tunnel {
                    match tunnel::start(host.port()).await {
                        Ok(tunnel) => {
                            if let Err(error) = host.set_public_origin(&tunnel.origin) {
                                tunnel.stop().await;
                                host.stop().await;
                                let _ = previous.save(&config::resolve_folder(&folder)?);
                                return Err(error);
                            }
                            Some(tunnel)
                        }
                        Err(error) => {
                            host.stop().await;
                            let _ = previous.save(&config::resolve_folder(&folder)?);
                            return Err(error);
                        }
                    }
                } else {
                    None
                };
                Ok((host, tunnel))
            }
            .await;
            let _ = tx.send(Event::Started(result, open_game));
        });
    }

    fn stop(&mut self) {
        if let Some(host) = self.host.take() {
            let tunnel = self.tunnel.take();
            self.busy = true;
            self.set_status(Level::Busy, "Stopping server…");
            let tx = self.tx.clone();
            self.runtime.as_ref().unwrap().spawn(async move {
                if let Some(tunnel) = tunnel {
                    tunnel.stop().await;
                }
                host.stop().await;
                let _ = tx.send(Event::Stopped);
            });
        }
    }

    fn events(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Started(result, open_game) => {
                    self.busy = false;
                    match result {
                        Ok((host, tunnel)) => {
                            self.tunnel = tunnel;
                            self.settings = host.settings.clone();
                            self.host_addresses = config::addresses();
                            self.code = host.code.clone();
                            let url = host.url().split('#').next().unwrap_or("").to_owned();
                            let mut text = format!("Running: {url}");
                            let mut level = Level::Ok;
                            if let Some(error) = &host.discovery_error {
                                text.push_str(&format!(
                                    " — discovery unavailable: {error}. Share the link manually."
                                ));
                                level = Level::Warn;
                            }
                            if open_game && self.auto_open {
                                let _ = webbrowser::open(&host.url());
                            }
                            self.host = Some(host);
                            self.set_status(level, text);
                            self.tab = Tab::Server;
                        }
                        Err(e) => {
                            self.fail(Fault::from_error("Could not start the server", &e));
                        }
                    }
                }
                Event::Stopped => {
                    self.busy = false;
                    let text = if self.status.starts_with("Cloudflare tunnel exited") {
                        "Stopped: the Cloudflare tunnel exited. Check Console output, then start again for a new link."
                            .to_owned()
                    } else {
                        "Stopped".to_owned()
                    };
                    self.host_addresses.clear();
                    self.share = false;
                    self.set_status(Level::Idle, text);
                }
                Event::Found(result) => {
                    self.scanning = false;
                    match result {
                        Ok(rows) => {
                            let count = rows.len();
                            self.found = rows;
                            self.set_status(Level::Idle, format!("Found {count} LAN server(s)"));
                        }
                        Err(e) => {
                            self.fail(Fault::from_error("LAN discovery failed", &e));
                        }
                    }
                }
                Event::Certificate(result) => {
                    self.busy = false;
                    match result {
                        Ok((cert, key)) => {
                            self.settings.tls_cert = cert;
                            self.settings.tls_key = key;
                            self.settings.https = true;
                            self.set_status(
                                Level::Ok,
                                "LAN certificate created. Share only lan-root-ca.cer — never a .key file.",
                            );
                        }
                        Err(e) => {
                            self.fail(Fault::from_error("Certificate generation failed", &e));
                        }
                    }
                }
            }
        }
        if let Some(host) = &self.host {
            self.settings.port = host.port();
        }
        if self.tunnel.as_ref().is_some_and(|t| t.finished()) {
            self.tunnel.take();
            self.stop();
            self.set_status(
                Level::Warn,
                "The Cloudflare tunnel exited; stopping the server. Check Console output, then start again for a new link.",
            );
        }
    }

    // -- pickers ------------------------------------------------------------

    /// Open a native folder picker and validate the choice immediately.
    fn pick_folder(&mut self) {
        let picked = rfd::FileDialog::new()
            .set_title("Select the extracted GrandV-assets folder")
            .pick_folder();
        if let Some(path) = picked {
            self.folder = path.to_string_lossy().into();
            self.load();
            if !self.folder_is_valid() {
                if let Err(fault) = self.folder_state() {
                    self.fault = Some(fault);
                }
            }
        }
    }

    /// Pick a certificate or key file for the Advanced settings.
    fn pick_pem(target: &mut String) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("PEM / key", &["pem", "key", "cer"])
            .pick_file()
        {
            *target = path.to_string_lossy().into();
        }
    }

    // -- drawing ------------------------------------------------------------

    fn header(&mut self, ui: &mut egui::Ui) {
        theme::wide_card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(gta_browser_launcher::APP_NAME)
                        .size(theme::FONT_TITLE)
                        .strong()
                        .color(theme::ACCENT),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("Story • Sandbox • Multiplayer")
                            .size(theme::FONT_SMALL)
                            .color(theme::TEXT_FAINT),
                    );
                });
            });

            ui.add_space(theme::SPACE_M);

            // ---- game folder picker, always visible -------------------------
            let valid = self.folder_is_valid();
            let editable = !self.busy && self.host.is_none();
            theme::field(ui, "Game folder", 96., |ui| {
                ui.add_enabled(
                    editable,
                    egui::TextEdit::singleline(&mut self.folder)
                        .desired_width(ui.available_width() - 260.)
                        .hint_text("Select the folder containing mirror/playgta5.com/launcher.html"),
                );
                if ui
                    .add_enabled(
                        editable,
                        egui::Button::new(egui::RichText::new("Browse…").size(theme::FONT_BODY)),
                    )
                    .clicked()
                {
                    self.pick_folder();
                }
                if ui
                    .add_enabled(
                        editable,
                        egui::Button::new(egui::RichText::new("Verify").size(theme::FONT_BODY)),
                    )
                    .clicked()
                {
                    match self.folder_state() {
                        Ok(path) => self.set_status(
                            Level::Ok,
                            format!("Game folder verified: {}", path.display()),
                        ),
                        Err(fault) => self.fail(fault),
                    }
                }
            });

            if valid {
                theme::validity_row(
                    ui,
                    true,
                    "mirror/playgta5.com/launcher.html found — the game can be hosted from here.",
                );
            } else {
                theme::validity_row(
                    ui,
                    false,
                    "No game found in this folder. Press Browse… and select the extracted GrandV-assets folder.",
                );
            }

            ui.add_space(theme::SPACE_M);

            // ---- primary actions -------------------------------------------
            let running = self.host.is_some() && !self.busy;
            ui.horizontal_wrapped(|ui| {
                if theme::action(
                    ui,
                    "Start server",
                    160.,
                    theme::PRIMARY_BG,
                    theme::PRIMARY_BG_HOVER,
                )
                .clicked()
                    && editable
                    && valid
                {
                    self.start(false);
                }
                if theme::action(ui, "Start & open game", 190., theme::OPEN_BG, theme::OPEN_BG_HOVER)
                    .clicked()
                    && editable
                    && valid
                {
                    self.start(true);
                }
                if theme::action(ui, "Stop server", 140., theme::STOP_BG, theme::STOP_BG_HOVER)
                    .clicked()
                    && running
                {
                    self.stop();
                }
                if let Some(host) = &self.host
                    && theme::button_w(ui, "Open game", 120.).clicked()
                {
                    let _ = webbrowser::open(&host.url());
                }
                if theme::button_w(ui, "Console output", 150.).clicked() {
                    if self.console.open.load(Ordering::Relaxed) {
                        ui.ctx().send_viewport_cmd_to(
                            egui::ViewportId::from_hash_of("grandv-console"),
                            egui::ViewportCommand::Focus,
                        );
                    }
                    self.console.open.store(true, Ordering::Relaxed);
                }
            });

            ui.add_space(theme::SPACE_S);

            // ---- status line and error entry point -------------------------
            let level = self.level;
            let text = self.status.clone();
            let has_fault = self.fault.is_some();
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_min_width((ui.available_width() - 150.).max(200.));
                    theme::status_pill(ui, level, &text);
                });
                if level == Level::Error && has_fault && theme::button_w(ui, "Error details", 140.).clicked() {
                    self.fault_open = true;
                }
            });
        });
    }

    fn game_tab(&mut self, ui: &mut egui::Ui) {
        theme::section(ui, "Play on this computer");
        theme::wide_card(ui, |ui| {
            ui.label(
                egui::RichText::new(
                    "Select the game folder above, then press Start & open game. The launcher serves the game to your own browser and stores host settings in that folder.",
                )
                .size(theme::FONT_BODY)
                .color(theme::TEXT_DIM),
            );
            ui.add_space(theme::SPACE_S);
            theme::hint(
                ui,
                "Saves are kept per mode and backed up on disk under the game folder's userdata directory.",
            );
            ui.add_space(theme::SPACE_M);
            ui.horizontal_wrapped(|ui| {
                if theme::button_w(ui, "Need to obtain assets?", 210.).clicked() {
                    self.obtain_assets = true;
                }
                if theme::button_w(ui, "Updates and info", 170.).clicked() {
                    self.updates = true;
                }
            });
        });

        ui.add_space(theme::SPACE_L);
        theme::section(ui, "Join a server — no game files needed");
        theme::wide_card(ui, |ui| {
            ui.label(
                egui::RichText::new(
                    "Enter the host's game website link. You do not need the game files or this launcher on the joining computer.",
                )
                .size(theme::FONT_BODY)
                .color(theme::TEXT_DIM),
            );
            ui.add_space(theme::SPACE_S);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.join)
                        .desired_width(ui.available_width() - 260.)
                        .hint_text("https://…/launcher.html"),
                );
                if theme::button(ui, "Open game").clicked() {
                    match config::join_url(&self.join) {
                        Ok(url) => {
                            if let Err(error) = webbrowser::open(&url) {
                                self.fail(Fault::new(
                                    format!("Could not open the browser: {error}"),
                                    error.to_string(),
                                    Remedy::Unknown,
                                ));
                            } else {
                                self.set_status(Level::Ok, format!("Opened {url}"));
                            }
                        }
                        Err(error) => self.fail(Fault::from_error(
                            "That website address is not valid",
                            &error,
                        )),
                    }
                }
                if theme::button(ui, "Find LAN servers").clicked() && !self.scanning {
                    self.scanning = true;
                    self.set_status(Level::Busy, "Scanning the local network…");
                    let tx = self.tx.clone();
                    self.runtime
                        .as_ref()
                        .unwrap()
                        .spawn(async move { let _ = tx.send(Event::Found(discovery::discover().await)); });
                }
            });

            if !self.found.is_empty() {
                ui.add_space(theme::SPACE_M);
                ui.label(
                    egui::RichText::new("Servers found on this network")
                        .size(theme::FONT_SMALL)
                        .strong()
                        .color(theme::TEXT),
                );
                let mut copy: Option<String> = None;
                for found in &self.found {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(&found.name)
                                .size(theme::FONT_SMALL)
                                .color(theme::TEXT),
                        );
                        if ui
                            .add(
                                egui::Label::new(
                                    egui::RichText::new(&found.url)
                                        .size(theme::FONT_SMALL)
                                        .monospace()
                                        .color(theme::ACCENT),
                                )
                                .selectable(true),
                            )
                            .clicked()
                        {
                            let _ = webbrowser::open(&found.url);
                        }
                        if ui
                            .add_sized(
                                [90., theme::CONTROL_H],
                                egui::Button::new(
                                    egui::RichText::new("Copy link").size(theme::FONT_SMALL),
                                ),
                            )
                            .clicked()
                        {
                            copy = Some(found.url.clone());
                        }
                        if !found.fingerprint.is_empty() {
                            ui.label(
                                egui::RichText::new(format!("CA SHA-256: {}", found.fingerprint))
                                    .size(theme::FONT_SMALL)
                                    .monospace()
                                    .color(theme::TEXT_FAINT),
                            );
                        }
                    });
                }
                if let Some(value) = copy {
                    ui.ctx().copy_text(value);
                }
            } else if self.scanning {
                theme::note(ui, Level::Busy, "Scanning…");
            } else {
                theme::hint(
                    ui,
                    "Discovery can be blocked by the network. A missing result does not mean the host is offline — use the host's copied link.",
                );
            }
        });
    }

    fn server_tab(&mut self, ui: &mut egui::Ui) {
        let editable = !self.busy && self.host.is_none();
        let valid = self.folder_is_valid();

        theme::section(ui, "Hosting");
        theme::wide_card(ui, |ui| {
            theme::field(ui, "Audience", 96., |ui| {
                egui::ComboBox::from_id_salt("audience")
                    .width(260.)
                    .selected_text(match self.settings.mode.as_str() {
                        "lan" => "LAN / school network",
                        "external" => "Public website",
                        _ => "This PC",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.settings.mode, "device".into(), "This PC");
                        ui.selectable_value(
                            &mut self.settings.mode,
                            "lan".into(),
                            "LAN / school network",
                        );
                        ui.selectable_value(
                            &mut self.settings.mode,
                            "external".into(),
                            "Public website",
                        );
                    });
            });
            theme::hint(
                ui,
                "This PC blocks network access. LAN lets other devices on your network connect. Public hosts on a website or tunnel.",
            );

            ui.add_space(theme::SPACE_S);
            ui.checkbox(
                &mut self.settings.multiplayer,
                egui::RichText::new("Enable multiplayer (2 players per room)").size(theme::FONT_BODY),
            );
            if self.settings.multiplayer {
                ui.add_space(theme::SPACE_XS);
                theme::field(ui, "Access code", 96., |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.code)
                            .desired_width(ui.available_width() - 240.),
                    );
                    if theme::button_w(ui, "Generate", 100.).clicked() {
                        self.code = config::random_token();
                    }
                    if theme::button_w(ui, "Copy", 80.).clicked() {
                        let code = self.code.clone();
                        ui.ctx().copy_text(code);
                    }
                });
                theme::hint(
                    ui,
                    "Codes are case-sensitive. Share the code separately from the link.",
                );
            }
        });

        if self.settings.mode == "external" {
            ui.add_space(theme::SPACE_L);
            theme::section(ui, "Public hosting");
            theme::wide_card(ui, |ui| {
                ui.checkbox(
                    &mut self.settings.cloudflare_tunnel,
                    egui::RichText::new("Cloudflare free tunnel (temporary HTTPS link)")
                        .size(theme::FONT_BODY),
                );
                if self.settings.cloudflare_tunnel {
                    ui.add_space(theme::SPACE_S);
                    theme::hint(
                        ui,
                        "Start creates a public HTTPS link. No domain, account, port forwarding or client certificate import needed. Keep this launcher open.",
                    );
                    theme::hint(
                        ui,
                        "Anyone with the link can access the game and save APIs. The multiplayer code protects only the relay. The URL changes on restart, and browser saves belong to each URL.",
                    );
                    theme::hint(
                        ui,
                        "Install cloudflared once, then place it beside the launcher or on PATH. Direct HTTPS is automatically disabled in this mode.",
                    );
                    ui.add_space(theme::SPACE_S);
                    ui.horizontal_wrapped(|ui| {
                        ui.hyperlink_to(
                            egui::RichText::new("Cloudflare setup / downloads").size(theme::FONT_SMALL),
                            "https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/downloads/",
                        );
                        if theme::button_w(ui, "Copy install command", 190.).clicked() {
                            ui.ctx().copy_text(
                                "winget install --id Cloudflare.cloudflared --exact".into(),
                            );
                        }
                    });
                    theme::hint(
                        ui,
                        "Windows: open PowerShell or Command Prompt, paste the copied command and press Enter. Restart the launcher afterwards.",
                    );
                    theme::hint(
                        ui,
                        "Quick Tunnels have no uptime guarantee, allow 200 in-flight requests and do not support SSE. Asset transfers use your upload bandwidth.",
                    );
                } else {
                    ui.add_space(theme::SPACE_S);
                    theme::field(ui, "Public origin", 110., |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.settings.public_origin)
                                .desired_width(ui.available_width()),
                        );
                    });
                    theme::hint(
                        ui,
                        "Configure the domain, HTTPS reverse proxy and firewall before sharing.",
                    );
                }
            });
        }

        ui.add_space(theme::SPACE_L);
        theme::section(ui, "Advanced server settings");
        theme::wide_card(ui, |ui| {
            let through_tunnel = self.settings.cloudflare_tunnel && self.settings.mode == "external";
            ui.add_enabled_ui(editable, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new("Game port")
                            .size(theme::FONT_BODY)
                            .color(theme::TEXT_DIM),
                    );
                    ui.add_sized(
                        [110., theme::CONTROL_H],
                        egui::DragValue::new(&mut self.settings.port).range(1024..=65535),
                    );
                    ui.label(
                        egui::RichText::new("Maximum clients (0 = unlimited)")
                            .size(theme::FONT_BODY)
                            .color(theme::TEXT_DIM),
                    );
                    ui.add_sized(
                        [110., theme::CONTROL_H],
                        egui::DragValue::new(&mut self.settings.max_clients).range(0..=10000),
                    );
                });

                ui.add_space(theme::SPACE_S);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new("Max concurrent connections / streams (0 = unlimited)")
                            .size(theme::FONT_BODY)
                            .color(theme::TEXT_DIM),
                    );
                    if through_tunnel {
                        let mut limit = 200_usize;
                        ui.add_enabled(
                            false,
                            egui::DragValue::new(&mut limit).range(0..=100000).speed(1.),
                        );
                    } else {
                        ui.add_sized(
                            [110., theme::CONTROL_H],
                            egui::DragValue::new(&mut self.settings.max_connections).range(0..=100000),
                        );
                    }
                });
                theme::hint(
                    ui,
                    if through_tunnel {
                        "Cloudflare mode uses 200 connections. Your direct-hosting limit is retained; Quick Tunnel also has its own 200 in-flight request limit."
                    } else {
                        "Applies to direct HTTP/HTTPS and manual proxy hosting. Counts open connections, including idle ones, rather than players."
                    },
                );

                ui.add_space(theme::SPACE_S);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(if through_tunnel {
                            "MiB/s shared by tunnel users (0 = unlimited)"
                        } else {
                            "MiB/s per IP (0 = unlimited)"
                        })
                        .size(theme::FONT_BODY)
                        .color(theme::TEXT_DIM),
                    );
                    ui.add_sized(
                        [110., theme::CONTROL_H],
                        egui::DragValue::new(&mut self.settings.speed_mib)
                            .range(0.0..=100000.0)
                            .speed(0.1),
                    );
                });
                theme::hint(
                    ui,
                    "The unit is MiB/s, not Mbit/s: 10 MiB/s is about 84 Mbit/s. Zero means unlimited.",
                );
                theme::hint(
                    ui,
                    "Internet hosting: leave about 20% upload headroom. Per-player budget = upload Mbit/s x 0.8 / 8.39 / active players.",
                );
                theme::hint(
                    ui,
                    if through_tunnel {
                        "Through Cloudflare all visitors share one combined limit, because they appear as loopback to this backend."
                    } else {
                        "Limits are per source IP, not per browser: clients behind one proxy or IP share the cap."
                    },
                );

                ui.add_space(theme::SPACE_S);
                ui.add_enabled(
                    !through_tunnel,
                    egui::Checkbox::new(
                        &mut self.settings.https,
                        egui::RichText::new("Use direct HTTPS / custom certificates")
                            .size(theme::FONT_BODY),
                    ),
                );

                for (label, value) in [
                    ("Certificate PEM", &mut self.settings.tls_cert),
                    ("Private key PEM", &mut self.settings.tls_key),
                ] {
                    theme::field(ui, label, 120., |ui| {
                        ui.add(
                            egui::TextEdit::singleline(value)
                                .desired_width(ui.available_width() - 130.),
                        );
                        if theme::button_w(ui, "Browse…", 110.).clicked() {
                            Self::pick_pem(value);
                        }
                    });
                }

                ui.add_space(theme::SPACE_S);
                if theme::button_w(ui, "Create / renew LAN certificate", 260.).clicked() {
                    match config::resolve_folder(&PathBuf::from(&self.folder)) {
                        Ok(folder) => {
                            self.busy = true;
                            self.set_status(Level::Busy, "Creating the LAN certificate…");
                            let tx = self.tx.clone();
                            self.runtime.as_ref().unwrap().spawn(async move {
                                let result =
                                    tokio::task::spawn_blocking(move || tls::generate(&folder))
                                        .await
                                        .map_err(anyhow::Error::from)
                                        .and_then(|v| v);
                                let _ = tx.send(Event::Certificate(result));
                            });
                        }
                        Err(error) => {
                            self.fail(Fault::from_error("Cannot create a certificate", &error));
                        }
                    }
                }
                theme::hint(
                    ui,
                    "LAN creates a private CA automatically. Trust is installed manually on each client.",
                );
            });

            if !editable {
                theme::note(
                    ui,
                    Level::Warn,
                    "Stop the server before changing hosting settings.",
                );
            }
            if !valid {
                theme::note(
                    ui,
                    Level::Warn,
                    "Select a valid game folder before starting the server.",
                );
            }
        });
    }

    fn network_tab(&mut self, ui: &mut egui::Ui) {
        theme::section(ui, "Addresses");
        let mut share_clicked = false;
        let mut copy: Option<String> = None;
        theme::wide_card(ui, |ui| {
            let Some(host) = &self.host else {
                theme::note(
                    ui,
                    Level::Idle,
                    "No server is running. Start the server to see its addresses here.",
                );
                return;
            };
            let scheme = host.settings.scheme();
            let port = host.port();

            theme::value_row(
                ui,
                "Local",
                &format!("{scheme}://localhost:{port}/launcher.html"),
                &mut copy,
            );
            theme::value_row(
                ui,
                "Loopback",
                &format!("{scheme}://127.0.0.1:{port}/launcher.html"),
                &mut copy,
            );

            if host.settings.bind_ip().is_unspecified() {
                if self.host_addresses.is_empty() {
                    theme::note(
                        ui,
                        Level::Warn,
                        "No IPv4 address was found, so other devices cannot connect.",
                    );
                }
                for ip in &self.host_addresses {
                    theme::value_row(
                        ui,
                        "Network",
                        &format!("{scheme}://{ip}:{port}/launcher.html"),
                        &mut copy,
                    );
                }
            } else if host.settings.mode == "device" {
                theme::note(
                    ui,
                    Level::Idle,
                    "Network access is off. Select LAN / school network before starting to allow other devices.",
                );
            } else {
                theme::note(
                    ui,
                    Level::Idle,
                    "The backend listens on localhost; network clients connect through the configured public website.",
                );
            }

            if !host.settings.public_origin.is_empty() {
                theme::value_row(
                    ui,
                    "Public",
                    &format!(
                        "{}/launcher.html",
                        host.settings.public_origin.trim_end_matches('/')
                    ),
                    &mut copy,
                );
            }

            ui.add_space(theme::SPACE_S);
            ui.label(
                egui::RichText::new(format!(
                    "Listener port: {}  •  Multiplayer: {}  •  Audience: {}",
                    host.port(),
                    if host.settings.multiplayer {
                        "enabled"
                    } else {
                        "disabled"
                    },
                    match host.settings.mode.as_str() {
                        "lan" => "LAN / school network",
                        "external" => "Public website",
                        _ => "This PC",
                    }
                ))
                .size(theme::FONT_SMALL)
                .color(theme::TEXT_DIM),
            );

            ui.horizontal_wrapped(|ui| {
                if host.settings.mode != "device"
                    && theme::button_w(ui, "Share game link", 170.).clicked()
                {
                    share_clicked = true;
                }
                if host.settings.multiplayer
                    && theme::button_w(ui, "Copy access code", 170.).clicked()
                {
                    let code = host.code.clone();
                    ui.ctx().copy_text(code);
                }
                if theme::button_w(ui, "Open game", 130.).clicked() {
                    let _ = webbrowser::open(&host.url());
                }
            });
            if !host.fingerprint().is_empty() {
                ui.add_space(theme::SPACE_S);
                theme::hint(
                    ui,
                    format!(
                        "LAN root CA SHA-256: {} — share only lan-root-ca.cer, never a .key file.",
                        host.fingerprint()
                    ),
                );
            }
        });
        if let Some(value) = copy {
            ui.ctx().copy_text(value);
        }
        if share_clicked {
            self.share = true;
        }
    }

    fn help_tab(&mut self, ui: &mut egui::Ui) {
        theme::section(ui, "When something goes wrong");
        theme::wide_card(ui, |ui| {
            ui.label(
                egui::RichText::new(
                    "Failures appear as a red status line with an Error details button. That window shows the exact error, the likely cause and the steps that fix it.",
                )
                .size(theme::FONT_BODY)
                .color(theme::TEXT_DIM),
            );
            ui.add_space(theme::SPACE_S);
            theme::hint(
                ui,
                "Console output keeps the full log. Use Copy output there when asking for help.",
            );
        });

        ui.add_space(theme::SPACE_L);
        theme::section(ui, "Quick fixes for common failures");
        for (title, steps) in GUIDE {
            theme::wide_card(ui, |ui| {
                egui::CollapsingHeader::new(
                    egui::RichText::new(*title)
                        .size(theme::FONT_BODY)
                        .strong()
                        .color(theme::TEXT),
                )
                .id_salt(*title)
                .show(ui, |ui| {
                    ui.add_space(theme::SPACE_XS);
                    for (index, step) in steps.iter().enumerate() {
                        ui.horizontal_top(|ui| {
                            ui.label(
                                egui::RichText::new(format!("{}.", index + 1))
                                    .size(theme::FONT_SMALL)
                                    .strong()
                                    .color(theme::ACCENT),
                            );
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(*step)
                                        .size(theme::FONT_SMALL)
                                        .line_height(Some(theme::LINE_BODY))
                                        .color(theme::TEXT),
                                )
                                .wrap(),
                            );
                        });
                        ui.add_space(theme::SPACE_XS);
                    }
                });
            });
            ui.add_space(theme::SPACE_S);
        }

        ui.add_space(theme::SPACE_L);
        theme::section(ui, "Project links");
        theme::wide_card(ui, |ui| {
            let mut failed: Option<(String, String)> = None;
            ui.horizontal_wrapped(|ui| {
                for (label, url) in [
                    ("Telegram", "https://t.me/reGRANDTA"),
                    ("Discord", "https://discord.gg/hrbcPjjZfj"),
                    ("GitHub", "https://github.com/SebasKitten/Grand-V"),
                ] {
                    if theme::button_w(ui, label, 150.).clicked()
                        && let Err(error) = webbrowser::open(url)
                    {
                        failed = Some((label.to_owned(), error.to_string()));
                    }
                }
            });
            ui.add_space(theme::SPACE_S);
            theme::hint(
                ui,
                "This launcher is provided for educational purposes only and distributes no game assets.",
            );
            if let Some((label, error)) = failed {
                self.fail(Fault::new(
                    format!("Could not open {label}: {error}"),
                    error,
                    Remedy::Unknown,
                ));
            }
        });
    }

    fn layout(&mut self, ctx: &egui::Context) {
        self.events();
        if self.console_status != self.status {
            gta_browser_launcher::diagnostics::record("Launcher", &self.status);
            self.console_status = self.status.clone();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(theme::SPACE_M);
                    ui.horizontal(|ui| {
                        ui.add_space(theme::SPACE_M);
                        ui.vertical(|ui| {
                            ui.set_max_width(1020.);
                            self.header(ui);

                            ui.add_space(theme::SPACE_L);
                            let labels: Vec<&str> = Tab::ALL.iter().map(|t| t.label()).collect();
                            let chosen = theme::tabs(ui, &labels, self.tab as usize);
                            self.tab = Tab::from_index(chosen);
                            ui.add_space(theme::SPACE_M);

                            match self.tab {
                                Tab::Game => self.game_tab(ui),
                                Tab::Server => self.server_tab(ui),
                                Tab::Network => self.network_tab(ui),
                                Tab::Help => self.help_tab(ui),
                            }
                            ui.add_space(theme::SPACE_XL);
                        });
                    });
                });
        });

        self.obtain_assets_window(ctx);
        self.updates_window(ctx);
        self.share_window(ctx);
        self.fault_window(ctx);
        self.donate_window(ctx);
        self.show_console(ctx);
    }

    fn obtain_assets_window(&mut self, ctx: &egui::Context) {
        if !self.obtain_assets {
            return;
        }
        let mut open = self.obtain_assets;
        let mut failure: Option<(String, String)> = None;
        egui::Window::new("Need to obtain assets?")
            .open(&mut open)
            .default_width(560.)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(
                        "A torrent downloads files in small pieces from other people sharing them. You can pause and resume it.",
                    )
                    .size(theme::FONT_BODY),
                );
                ui.add_space(theme::SPACE_S);
                ui.label(
                    egui::RichText::new("We recommend qBittorrent: free, open source and no ads.")
                        .size(theme::FONT_BODY)
                        .color(theme::TEXT_DIM),
                );
                ui.add_space(theme::SPACE_S);
                if theme::button_w(ui, "1. Get qBittorrent", 220.).clicked()
                    && let Err(error) = webbrowser::open("https://www.qbittorrent.org/download")
                {
                    failure = Some((
                        format!("Could not open the qBittorrent download page: {error}"),
                        error.to_string(),
                    ));
                }
                ui.add_space(theme::SPACE_S);
                ui.label(
                    egui::RichText::new("2. Open the asset torrent and choose a download location.")
                        .size(theme::FONT_BODY),
                );
                ui.horizontal_wrapped(|ui| {
                    if theme::button_w(ui, "Open asset torrent", 190.).clicked()
                        && let Err(error) = open_asset_torrent()
                    {
                        failure = Some((
                            format!("Could not open the torrent client: {error}"),
                            format!(
                                "{error}\n\nUse Copy magnet link and add it to your torrent client manually."
                            ),
                        ));
                    }
                    if theme::button_w(ui, "Copy magnet link", 170.).clicked() {
                        ui.ctx().copy_text(ASSET_TORRENT_URL.to_owned());
                    }
                });
                theme::hint(
                    ui,
                    "If the link does not open, copy it and paste it into qBittorrent.",
                );
                ui.add_space(theme::SPACE_S);
                ui.label(
                    egui::RichText::new(
                        "3. Wait for 100%, then use Browse to select GrandV-assets and choose Start & open game. No extraction is needed.",
                    )
                    .size(theme::FONT_BODY),
                );
            });
        self.obtain_assets = open;
        if let Some((summary, detail)) = failure {
            self.fail(Fault::new(summary, detail, Remedy::Unknown));
        }
    }

    fn updates_window(&mut self, ctx: &egui::Context) {
        if !self.updates {
            return;
        }
        let mut open = self.updates;
        let mut failed: Option<(String, String)> = None;
        egui::Window::new("Updates and info")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(460.)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (label, url) in [
                        ("Telegram", "https://t.me/reGRANDTA"),
                        ("Discord", "https://discord.gg/hrbcPjjZfj"),
                        ("GitHub", "https://github.com/SebasKitten/Grand-V"),
                    ] {
                        if theme::button_w(ui, label, 140.).clicked()
                            && let Err(error) = webbrowser::open(url)
                        {
                            failed = Some((label.to_owned(), error.to_string()));
                        }
                    }
                });
                ui.add_space(theme::SPACE_S);
                ui.label(
                    egui::RichText::new(
                        "A major re-release is planned. Stability, rendering, audio, streaming and compatibility are the active work areas.",
                    )
                    .size(theme::FONT_BODY)
                    .color(theme::TEXT_DIM),
                );
            });
        self.updates = open;
        if let Some((label, error)) = failed {
            self.fail(Fault::new(
                format!("Could not open {label}: {error}"),
                error,
                Remedy::Unknown,
            ));
        }
    }

    fn share_window(&mut self, ctx: &egui::Context) {
        if !self.share {
            return;
        }
        let Some(host) = &self.host else {
            self.share = false;
            return;
        };
        let url = host.share_url();
        let code = host.code.clone();
        let multiplayer = host.settings.multiplayer;
        let fingerprint = host.fingerprint().to_owned();
        let mut open = self.share;
        egui::Window::new("Share game link")
            .open(&mut open)
            .default_width(520.)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(url.clone())
                        .size(theme::FONT_SMALL)
                        .monospace()
                        .color(theme::ACCENT),
                );
                ui.horizontal_wrapped(|ui| {
                    if theme::button_w(ui, "Copy link", 130.).clicked() {
                        ui.ctx().copy_text(url.clone());
                    }
                    if multiplayer && theme::button_w(ui, "Copy access code", 170.).clicked() {
                        ui.ctx().copy_text(code.clone());
                    }
                });
                ui.add_space(theme::SPACE_S);
                Self::qr(ui, &url);
                if !fingerprint.is_empty() {
                    ui.add_space(theme::SPACE_S);
                    theme::hint(
                        ui,
                        "Confirm this fingerprint with the person connecting, so both of you use the same certificate.",
                    );
                    ui.label(
                        egui::RichText::new(format!("CA SHA-256: {fingerprint}"))
                            .size(theme::FONT_SMALL)
                            .monospace()
                            .color(theme::TEXT_FAINT),
                    );
                }
                if multiplayer {
                    theme::note(
                        ui,
                        Level::Warn,
                        "The QR code contains the link only. Share the multiplayer access code separately.",
                    );
                }
            });
        self.share = open;
    }

    /// The error report window: summary, cause, verbatim detail, and the fix list.
    fn fault_window(&mut self, ctx: &egui::Context) {
        if !self.fault_open {
            return;
        }
        let Some(fault) = self.fault.clone() else {
            self.fault_open = false;
            return;
        };
        let mut open = self.fault_open;
        let mut clear = false;
        let mut to_tab: Option<Tab> = None;
        let mut open_console = false;
        let mut open_assets = false;
        let mut browse = false;
        let mut copy_report = false;
        egui::Window::new("Error details")
            .open(&mut open)
            .default_width(640.)
            .default_height(520.)
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(&fault.summary)
                            .size(theme::FONT_HEADING)
                            .strong()
                            .color(theme::DANGER),
                    );
                    ui.add_space(theme::SPACE_M);

                    ui.label(
                        egui::RichText::new("What happened")
                            .size(theme::FONT_SMALL)
                            .strong()
                            .color(theme::TEXT_DIM),
                    );
                    ui.add(
                        egui::Label::new(egui::RichText::new(&fault.summary).size(theme::FONT_BODY))
                            .wrap(),
                    );

                    ui.add_space(theme::SPACE_M);
                    ui.label(
                        egui::RichText::new("Likely cause")
                            .size(theme::FONT_SMALL)
                            .strong()
                            .color(theme::TEXT_DIM),
                    );
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&fault.cause)
                                .size(theme::FONT_BODY)
                                .color(theme::TEXT),
                        )
                        .wrap(),
                    );

                    ui.add_space(theme::SPACE_M);
                    ui.label(
                        egui::RichText::new("Technical detail")
                            .size(theme::FONT_SMALL)
                            .strong()
                            .color(theme::TEXT_DIM),
                    );
                    theme::sunken().show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&fault.detail)
                                    .size(theme::FONT_SMALL)
                                    .monospace()
                                    .color(theme::TEXT),
                            )
                            .selectable(true)
                            .wrap(),
                        );
                    });

                    ui.add_space(theme::SPACE_M);
                    ui.label(
                        egui::RichText::new(format!("How to fix it — {}", fault.remedy.title()))
                            .size(theme::FONT_SMALL)
                            .strong()
                            .color(theme::ACCENT_SOFT),
                    );
                    ui.add_space(theme::SPACE_XS);
                    for (index, step) in fault.remedy.steps().iter().enumerate() {
                        ui.horizontal_top(|ui| {
                            ui.label(
                                egui::RichText::new(format!("{}.", index + 1))
                                    .size(theme::FONT_BODY)
                                    .strong()
                                    .color(theme::ACCENT),
                            );
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(*step)
                                        .size(theme::FONT_BODY)
                                        .line_height(Some(theme::LINE_BODY)),
                                )
                                .wrap(),
                            );
                        });
                        ui.add_space(theme::SPACE_XS);
                    }

                    // Remedy-specific one-click affordances.
                    ui.add_space(theme::SPACE_M);
                    ui.horizontal_wrapped(|ui| match fault.remedy {
                        Remedy::PickGameFolder => {
                            if theme::button_w(ui, "Browse for game folder", 220.).clicked() {
                                browse = true;
                            }
                        }
                        Remedy::ChangePort => {
                            if theme::button_w(ui, "Go to advanced settings", 230.).clicked() {
                                to_tab = Some(Tab::Server);
                            }
                        }
                        Remedy::InstallCloudflared => {
                            if theme::button_w(ui, "Copy install command", 190.).clicked() {
                                ui.ctx().copy_text(
                                    "winget install --id Cloudflare.cloudflared --exact".into(),
                                );
                            }
                        }
                        Remedy::RenewCertificate => {
                            if theme::button_w(ui, "Go to certificate settings", 240.).clicked() {
                                to_tab = Some(Tab::Server);
                            }
                        }
                        Remedy::Repackage => {
                            if theme::button_w(ui, "Open asset help", 160.).clicked() {
                                open_assets = true;
                            }
                        }
                        _ => {}
                    });

                    ui.add_space(theme::SPACE_M);
                    ui.horizontal_wrapped(|ui| {
                        if theme::button_w(ui, "Copy full report", 170.).clicked() {
                            copy_report = true;
                        }
                        if theme::button_w(ui, "Open console output", 190.).clicked() {
                            open_console = true;
                        }
                        if theme::button_w(ui, "Dismiss", 120.).clicked() {
                            clear = true;
                        }
                    });
                });
            });
        self.fault_open = open && !clear;
        if copy_report {
            ctx.copy_text(fault.report());
        }
        if open_console {
            self.console.open.store(true, Ordering::Relaxed);
        }
        if open_assets {
            self.fault_open = false;
            self.obtain_assets = true;
        }
        if browse {
            self.fault_open = false;
            self.pick_folder();
        }
        if let Some(tab) = to_tab {
            self.fault_open = false;
            self.tab = tab;
        }
    }

    fn donate_window(&mut self, ctx: &egui::Context) {
        if !self.donate {
            return;
        }
        let mut open = self.donate;
        egui::Window::new("Optional donation")
            .open(&mut open)
            .default_width(520.)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(
                        "Supporting the project is entirely optional. Thank you if you choose to.",
                    )
                    .size(theme::FONT_BODY)
                    .color(theme::TEXT_DIM),
                );
                ui.add_space(theme::SPACE_S);
                if let Some(wallets) = self.wallets.as_array() {
                    for wallet in wallets {
                        let name = wallet["name"].as_str().unwrap_or("Wallet");
                        let symbol = wallet["symbol"].as_str().unwrap_or("");
                        let address = wallet["address"].as_str().unwrap_or("");
                        if address.is_empty() {
                            continue;
                        }
                        let selected = self.donation_coin.as_deref() == wallet["id"].as_str();
                        if ui
                            .selectable_label(
                                selected,
                                egui::RichText::new(format!("{name} ({symbol})"))
                                    .size(theme::FONT_BODY),
                            )
                            .clicked()
                        {
                            self.donation_coin = wallet["id"].as_str().map(str::to_owned);
                        }
                        if selected {
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(address)
                                            .size(theme::FONT_SMALL)
                                            .monospace()
                                            .color(theme::TEXT),
                                    )
                                    .selectable(true)
                                    .wrap(),
                                );
                                if theme::button_w(ui, "Copy", 80.).clicked() {
                                    ui.ctx().copy_text(address.to_owned());
                                }
                            });
                        }
                    }
                }
            });
        self.donate = open;
    }

    fn qr(ui: &mut egui::Ui, value: &str) {
        if let Ok(code) = qrcode::QrCode::new(value) {
            let width = code.width();
            let side = 230.;
            let cell = side / (width + 8) as f32;
            let (rect, _) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::hover());
            ui.painter().rect_filled(rect, 0., egui::Color32::WHITE);
            for y in 0..width {
                for x in 0..width {
                    if code[(x, y)] == qrcode::Color::Dark {
                        let p = rect.min + egui::vec2((x + 4) as f32 * cell, (y + 4) as f32 * cell);
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(p, egui::vec2(cell, cell)),
                            0.,
                            egui::Color32::BLACK,
                        );
                    }
                }
            }
        }
    }

    fn console_contents(ui: &mut egui::Ui, console: &ConsoleWindow) {
        let mut follow = console.follow.load(Ordering::Relaxed);
        ui.horizontal(|ui| {
            ui.checkbox(
                &mut follow,
                egui::RichText::new("Follow output").size(theme::FONT_BODY),
            );
            if ui
                .add_sized(
                    [140., theme::CONTROL_H],
                    egui::Button::new(egui::RichText::new("Copy output").size(theme::FONT_BODY)),
                )
                .clicked()
            {
                ui.ctx()
                    .copy_text(gta_browser_launcher::diagnostics::snapshot());
            }
            if ui
                .add_sized(
                    [110., theme::CONTROL_H],
                    egui::Button::new(egui::RichText::new("Clear").size(theme::FONT_BODY)),
                )
                .clicked()
            {
                gta_browser_launcher::diagnostics::clear();
            }
        });
        console.follow.store(follow, Ordering::Relaxed);
        theme::hint(
            ui,
            "Closing this window hides the console; the server keeps running.",
        );
        ui.separator();
        let text = gta_browser_launcher::diagnostics::snapshot();
        egui::ScrollArea::both()
            .stick_to_bottom(follow)
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(if text.is_empty() {
                            "Waiting for output…"
                        } else {
                            &text
                        })
                        .size(theme::FONT_SMALL)
                        .monospace()
                        .color(theme::TEXT),
                    )
                    .selectable(true)
                    .wrap_mode(egui::TextWrapMode::Extend),
                );
            });
    }

    fn show_console(&self, ctx: &egui::Context) {
        let console = self.console.clone();
        let smoke = self.smoke;
        let screenshot = self.screenshot.clone();
        let created = self.created;
        ctx.show_viewport_deferred(
            egui::ViewportId::from_hash_of("grandv-console"),
            egui::ViewportBuilder::default()
                .with_title("GrandV Console output")
                .with_inner_size([900., 500.])
                .with_min_inner_size([480., 240.])
                .with_icon(console_icon()),
            move |ctx, class| {
                if class == egui::ViewportClass::Embedded {
                    let mut open = console.open.load(Ordering::Relaxed);
                    egui::Window::new("Console output")
                        .open(&mut open)
                        .show(ctx, |ui| Self::console_contents(ui, &console));
                    console.open.store(open, Ordering::Relaxed);
                } else {
                    if ctx.input(|i| i.viewport().close_requested()) {
                        console.open.store(false, Ordering::Relaxed);
                        ctx.request_repaint_of(egui::ViewportId::ROOT);
                        return;
                    }
                    egui::CentralPanel::default()
                        .frame(egui::Frame::central_panel(&ctx.style()).fill(theme::SUNKEN))
                        .show(ctx, |ui| Self::console_contents(ui, &console));
                    ctx.request_repaint_after(Duration::from_millis(100));
                    if smoke
                        && screenshot.is_some()
                        && created.elapsed() > Duration::from_secs(1)
                        && !console.screenshot_requested.swap(true, Ordering::Relaxed)
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
                    }
                    if let Some(path) = &screenshot {
                        ctx.input(|i| {
                            for event in &i.events {
                                if let egui::Event::Screenshot { image, .. } = event {
                                    let pixels: Vec<_> =
                                        image.pixels.iter().flat_map(|c| c.to_array()).collect();
                                    let _ = image::save_buffer(
                                        path.with_extension("console.png"),
                                        &pixels,
                                        image.size[0] as u32,
                                        image.size[1] as u32,
                                        image::ColorType::Rgba8,
                                    );
                                }
                            }
                        });
                    }
                }
            },
        );
    }
}

fn console_icon() -> std::sync::Arc<egui::IconData> {
    static ICON: std::sync::OnceLock<std::sync::Arc<egui::IconData>> = std::sync::OnceLock::new();
    ICON.get_or_init(|| {
        let image =
            image::load_from_memory(include_bytes!(concat!(env!("OUT_DIR"), "/app-icon.png")))
                .expect("Embedded app icon")
                .into_rgba8();
        let image = image::imageops::resize(&image, 128, 128, image::imageops::FilterType::Lanczos3);
        std::sync::Arc::new(egui::IconData {
            rgba: image.into_raw(),
            width: 128,
            height: 128,
        })
    })
    .clone()
}

fn open_file(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer.exe").arg(path).spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path.parent().unwrap_or(path))
            .spawn()?;
    }
    Ok(())
}

impl eframe::App for Launcher {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.layout(ctx);
        ctx.request_repaint_after(Duration::from_millis(100));
        if self.screenshot.is_some()
            && !self.screenshot_requested
            && self.created.elapsed() > Duration::from_secs(1)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            self.screenshot_requested = true;
        }
        if let Some(path) = &self.screenshot {
            ctx.input(|i| {
                for event in &i.events {
                    if let egui::Event::Screenshot { image, .. } = event {
                        let pixels: Vec<_> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
                        let _ = image::save_buffer(
                            path,
                            &pixels,
                            image.size[0] as u32,
                            image.size[1] as u32,
                            image::ColorType::Rgba8,
                        );
                    }
                }
            });
        }
        if self.smoke && self.created.elapsed() > Duration::from_secs(3) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl Drop for Launcher {
    fn drop(&mut self) {
        if let Some(tunnel) = self.tunnel.take()
            && let Some(rt) = &self.runtime
        {
            rt.block_on(tunnel.stop());
        }
        if let Some(host) = self.host.take() {
            host.request_stop();
            if let Some(rt) = &self.runtime {
                rt.block_on(host.stop());
            }
        }
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_timeout(Duration::from_secs(3));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_works_without_game_folder_and_contains_embedded_data() {
        let app = Launcher::new(
            PathBuf::from("/definitely/not/a/game/folder"),
            None,
            false,
            true,
            None,
        );
        assert!(!app.folder_is_valid());
        assert_eq!(app.level, Level::Warn);
        assert!(app.wallets.as_array().is_some_and(|w| !w.is_empty()));
        assert_eq!(app.tab as usize, Tab::Game as usize);
    }

    #[test]
    fn every_remedy_offers_at_least_one_concrete_step() {
        for remedy in [
            Remedy::PickGameFolder,
            Remedy::Repackage,
            Remedy::ChangePort,
            Remedy::InstallCloudflared,
            Remedy::TunnelTimeout,
            Remedy::RenewCertificate,
            Remedy::Firewall,
            Remedy::SettingsFile,
            Remedy::NetworkInterface,
            Remedy::Unknown,
        ] {
            assert!(!remedy.steps().is_empty(), "{remedy:?} has no fix steps");
            assert!(remedy.steps().iter().all(|s| !s.trim().is_empty()));
            assert!(!remedy.title().is_empty());
        }
    }

    #[test]
    fn wrong_folder_is_classified_as_a_folder_problem() {
        let error = anyhow::anyhow!(
            "Select the extracted GrandV-assets folder, not the launcher folder or an archive. Expected mirror/playgta5.com/launcher.html inside it."
        );
        let fault = Fault::from_error("The game folder is not usable", &error);
        assert_eq!(fault.remedy, Remedy::PickGameFolder);
        assert!(fault.detail.contains("launcher.html"));
        assert!(fault.report().contains("Suggested fixes"));
    }

    #[test]
    fn busy_port_is_classified_as_a_port_problem() {
        for message in [
            "Address already in use (os error 98)",
            "Only one usage of each socket address is normally permitted (os error 10048)",
        ] {
            let error = anyhow::anyhow!(message);
            assert_eq!(
                Fault::from_error("Could not start the server", &error).remedy,
                Remedy::ChangePort
            );
        }
    }

    #[test]
    fn tab_order_is_stable() {
        for (index, tab) in Tab::ALL.iter().enumerate() {
            assert_eq!(Tab::from_index(index), *tab);
        }
        assert_eq!(Tab::from_index(99), Tab::Game);
    }
}
