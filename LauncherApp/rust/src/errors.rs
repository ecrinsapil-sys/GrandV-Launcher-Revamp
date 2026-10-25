//! Structured error reporting.
//!
//! Every failure the launcher can produce is turned into a [`Fault`] carrying
//! four things the user actually needs: what happened, the full technical
//! chain, the likely cause, and the concrete steps that fix it.
//!
//! The raw error text is never thrown away — it lives in `detail` and is shown
//! verbatim, because guessing at a paraphrase is how people get stuck.

use crate::theme::Level;
use std::fmt::Write as _;

/// What the user should do about a failure.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Remedy {
    /// The folder does not contain the expected game files.
    PickGameFolder,
    /// The game package is incomplete or was not extracted.
    Repackage,
    /// Something already holds the port.
    ChangePort,
    /// cloudflared is missing or broken.
    InstallCloudflared,
    /// The tunnel did not come up in time.
    TunnelTimeout,
    /// The LAN certificate is expired, broken or mismatched.
    RenewCertificate,
    /// The OS or network blocked the listener.
    Firewall,
    /// Settings on disk could not be read or written.
    SettingsFile,
    /// No network interface was available.
    NetworkInterface,
    /// Anything else — show the chain and offer the console.
    Unknown,
}

impl Remedy {
    /// Human-readable fix steps, in order.
    pub fn steps(self) -> &'static [&'static str] {
        match self {
            Remedy::PickGameFolder => &[
                "Press Browse… and select the extracted GrandV-assets folder.",
                "The folder must contain mirror/playgta5.com/launcher.html.",
                "Do not select the launcher program, a ZIP file or a partly downloaded folder.",
                "If you selected the parent folder, the launcher also accepts that as long as the game folder is directly inside it.",
            ],
            Remedy::Repackage => &[
                "Finish the download completely — do not stop it early.",
                "Extract the archive again into a clean folder.",
                "Confirm the extracted folder contains grandv-assets.json next to mirror/.",
                "A missing or wrong-sized file is named in the detail above: download that file again.",
            ],
            Remedy::ChangePort => &[
                "Open Advanced server settings.",
                "Change the Game port to a free value, for example 8010 or 8085.",
                "Only change hosting settings while the server is stopped.",
                "Close whatever other program is using the port, or leave it running and pick another port.",
            ],
            Remedy::InstallCloudflared => &[
                "Open a terminal: PowerShell or Command Prompt on Windows.",
                "Paste and run: winget install --id Cloudflare.cloudflared --exact",
                "Restart the launcher afterwards so it picks up the new PATH.",
                "Alternatively place cloudflared.exe next to the launcher program.",
            ],
            Remedy::TunnelTimeout => &[
                "Check that the computer has working Internet access.",
                "Look at Console output for the exact cloudflared message.",
                "An existing .cloudflared/config.yaml can interfere — inspect it without deleting other tunnels.",
                "Try again: a Quick Tunnel URL sometimes takes a few extra seconds to appear.",
            ],
            Remedy::RenewCertificate => &[
                "Confirm the date and time are correct on both the host and the client.",
                "Open Advanced server settings and use Create / renew LAN certificate.",
                "Start hosting again and share the new link.",
                "Never delete the whole lan-tls folder — delete only what the detail above names.",
            ],
            Remedy::Firewall => &[
                "Allow the launcher program on the network profile you are using when the OS asks.",
                "Select LAN / school network before starting if other devices must connect.",
                "Do not disable the whole firewall to work around it.",
                "On a managed school or work computer, ask the administrator.",
            ],
            Remedy::SettingsFile => &[
                "Check that the game folder is writable — not read-only and not on a protected system path.",
                "If launcher-settings.json exists and is damaged, rename it and start again to get fresh defaults.",
                "The same applies to multiplayer-access-code.txt.",
            ],
            Remedy::NetworkInterface => &[
                "Check that at least one network adapter is enabled and has an IPv4 address.",
                "Wi-Fi that is connected but has no address yet also causes this.",
                "LAN server discovery needs a working local network.",
            ],
            Remedy::Unknown => &[
                "Open Console output for the full log around the failure.",
                "Use Copy output to send the log when asking for help.",
                "Getting the game folder and the selected audience right resolves most cases.",
            ],
        }
    }

    /// Short human title for the remedy, used as the fix-list heading.
    pub fn title(self) -> &'static str {
        match self {
            Remedy::PickGameFolder => "Choose the correct game folder",
            Remedy::Repackage => "Download or extract the game package again",
            Remedy::ChangePort => "Use a free port",
            Remedy::InstallCloudflared => "Install cloudflared",
            Remedy::TunnelTimeout => "The tunnel did not come up",
            Remedy::RenewCertificate => "Renew the LAN certificate",
            Remedy::Firewall => "Allow the launcher through the firewall",
            Remedy::SettingsFile => "The settings file could not be used",
            Remedy::NetworkInterface => "No usable network interface",
            Remedy::Unknown => "What to try next",
        }
    }
}

/// A single failure, ready to be displayed.
#[derive(Clone, Debug)]
pub struct Fault {
    /// One-line summary, shown in the status bar.
    pub summary: String,
    /// Full technical chain, never paraphrased.
    pub detail: String,
    /// What most likely caused it.
    pub cause: String,
    /// Which fix list applies.
    pub remedy: Remedy,
}

impl Fault {
    pub fn new(summary: impl Into<String>, detail: impl Into<String>, remedy: Remedy) -> Self {
        let detail = detail.into();
        Self {
            cause: default_cause(remedy).to_owned(),
            summary: summary.into(),
            detail,
            remedy,
        }
    }

    /// Build a fault from any error, classifying it from its message.
    ///
    /// Classification is deliberately done on the message text: the launcher's
    /// own errors are written to be recognisable, and an unrecognised error
    /// still renders usefully as `Remedy::Unknown` with the full chain intact.
    pub fn from_error(context: &str, error: &anyhow::Error) -> Self {
        let detail = format!("{error:#}");
        let remedy = classify(&detail);
        Self {
            summary: format!("{context}: {}", first_line(&detail)),
            detail,
            cause: default_cause(remedy).to_owned(),
            remedy,
        }
    }

    pub fn level(&self) -> Level {
        Level::Error
    }

    /// The full report, as copied to the clipboard.
    pub fn report(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "=== GrandV launcher error ===");
        let _ = writeln!(out, "Summary : {}", self.summary);
        let _ = writeln!(out, "Cause   : {}", self.cause);
        let _ = writeln!(out, "\nDetail  :\n{}", self.detail);
        let _ = writeln!(out, "\nSuggested fixes ({})", self.remedy.title());
        for (index, step) in self.remedy.steps().iter().enumerate() {
            let _ = writeln!(out, "  {}. {}", index + 1, step);
        }
        out
    }
}

fn first_line(text: &str) -> String {
    text.lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("unknown error")
        .trim()
        .to_owned()
}

/// Map an error message onto the fix list that addresses it.
fn classify(detail: &str) -> Remedy {
    let text = detail.to_ascii_lowercase();

    if text.contains("grandv-assets is incomplete")
        || text.contains("file list is empty")
        || text.contains("unsupported grandv-assets package")
        || text.contains("file list is missing")
    {
        return Remedy::Repackage;
    }
    if text.contains("select the extracted grandv-assets folder") || text.contains("launcher.html") {
        return Remedy::PickGameFolder;
    }
    if text.contains("address already in use")
        || text.contains("address in use")
        || text.contains("os error 98")
        || text.contains("os error 10048")
    {
        return Remedy::ChangePort;
    }
    if text.contains("cloudflared") {
        if text.contains("timed out") || text.contains("timeout") {
            return Remedy::TunnelTimeout;
        }
        return Remedy::InstallCloudflared;
    }
    if text.contains("lan root")
        || text.contains("ca certificate")
        || text.contains("key mismatch")
        || text.contains("no private key")
        || text.contains("incomplete lan root ca pair")
    {
        return Remedy::RenewCertificate;
    }
    if text.contains("permission denied")
        || text.contains("access is denied")
        || text.contains("os error 13")
        || text.contains("os error 10013")
        || text.contains("firewall")
    {
        return Remedy::Firewall;
    }
    if text.contains("could not read settings")
        || text.contains("read access code")
        || text.contains("launcher-settings")
    {
        return Remedy::SettingsFile;
    }
    if text.contains("no discovery interface") {
        return Remedy::NetworkInterface;
    }
    Remedy::Unknown
}

fn default_cause(remedy: Remedy) -> &'static str {
    match remedy {
        Remedy::PickGameFolder => {
            "The selected folder is not the extracted game folder, so the launcher cannot find the game inside it."
        }
        Remedy::Repackage => {
            "The game package on disk is incomplete or damaged, so the launcher refuses to host it."
        }
        Remedy::ChangePort => {
            "Another program is already using the chosen port, so the listener could not start."
        }
        Remedy::InstallCloudflared => {
            "The cloudflared program could not be started, so the public link cannot be created."
        }
        Remedy::TunnelTimeout => {
            "cloudflared ran but did not report a public address within the time limit."
        }
        Remedy::RenewCertificate => {
            "The LAN certificate stored in the game folder is missing, expired, or does not match its private key."
        }
        Remedy::Firewall => {
            "The operating system or security software blocked the launcher's access to the network or to its own files."
        }
        Remedy::SettingsFile => {
            "The launcher could not read or write its settings files inside the game folder."
        }
        Remedy::NetworkInterface => {
            "The launcher found no local network interface it could use, which LAN hosting needs."
        }
        Remedy::Unknown => {
            "The launcher did not recognise this failure, so the technical detail below is the best guide."
        }
    }
}

/// Fixed, curated troubleshooting entries bundled with the launcher.
///
/// These cover the situations a launcher cannot detect by itself (a wrong
/// certificate installed on a client, a school network blocking discovery),
/// so they are static content rather than runtime errors.
pub const GUIDE: &[(&str, &[&str])] = &[
    (
        "The game does not start in the browser",
        &[
            "Use an up-to-date Chrome, Edge or other Chromium browser.",
            "Enable hardware acceleration in the browser settings.",
            "Check the browser page for a WebGPU or shared-memory warning — the game needs both.",
            "On a laptop, make sure the browser is using the dedicated GPU, not a power-saving one.",
        ],
    ),
    (
        "Another device cannot connect",
        &[
            "Select LAN / school network on the host before starting, then use a displayed Local network address.",
            "localhost and 127.0.0.1 always mean the computer you are typing on — they never work from another device.",
            "Allow the launcher through the firewall on the active network profile.",
            "Guest Wi-Fi, VPNs and school filtering can block connections even when hosting is running.",
        ],
    ),
    (
        "Certificate warning on a LAN client",
        &[
            "Share only lan-root-ca.cer — never share a .key file.",
            "On Windows, install the certificate into Trusted Root Certification Authorities for the current user.",
            "On macOS, set the trust setting on the GrandV LAN Local Root CA to Always Trust.",
            "On Linux or a Chromebook, import it as a local or custom certificate authority, then restart the browser.",
        ],
    ),
    (
        "Assets load slowly or stall",
        &[
            "The speed limit unit is MiB/s, not Mbit/s: 10 MiB/s is about 84 Mbit/s.",
            "Leave zero (unlimited) on a fast LAN to avoid artificial throttling.",
            "For Internet hosting, budget roughly: upload Mbit/s x 0.8 / 8.39 / active players.",
            "The limit is per source IP, so players behind one proxy or tunnel share it.",
        ],
    ),
    (
        "Multiplayer code is rejected",
        &[
            "Codes are case-sensitive — copy the host's code exactly.",
            "Both players must be on the same build and map.",
            "Stop hosting before changing the code, then share it again.",
            "The built-in relay is a two-player room system; it does not provide shared missions or traffic.",
        ],
    ),
    (
        "Cloudflare free tunnel stops working",
        &[
            "Quick Tunnel links are temporary and their hostname changes on every start.",
            "Keep the launcher open and the host awake for as long as you want the link to work.",
            "Install cloudflared once and restart the launcher after installing so PATH changes apply.",
            "Browser saves and cache belong to each URL, so a new URL starts with empty browser storage.",
        ],
    ),
];
