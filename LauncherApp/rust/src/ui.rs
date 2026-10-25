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
