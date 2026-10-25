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
