//! GrandV Back end launcher entry point.
//!
//! `theme` owns every colour and size, `errors` owns failure reporting, and
//! `ui` composes them into the screens.
//!
//! Note on the library half: this file calls into `gta_browser_launcher::…`,
//! which is the crate this package builds. Keep `src/lib.rs` next to this file
//! with the upstream modules declared (config, diagnostics, discovery, relay,
//! server, storage, tls, tunnel).

mod errors;
mod theme;
mod ui;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "GrandV Back end launcher; game assets remain external")]
struct Args {
    /// Path to the extracted game folder. Validated at startup.
    #[arg(long)]
    folder: Option<PathBuf>,
    /// Run the host without a GUI; read its existing launcher-settings.json.
    #[arg(long)]
    headless: bool,
    #[arg(long)]
    port: Option<u16>,
    #[arg(long)]
    no_open: bool,
    /// Open and automatically close a native GUI to check display initialization.
    #[arg(long)]
    ui_smoke: bool,
    #[arg(long)]
    ui_screenshot: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let args = Args::parse();

    #[cfg(windows)]
    if args.headless {
        // GUI launches have no console. Headless launches attach to their calling terminal for logs and Ctrl+C.
        unsafe {
            windows_sys::Win32::System::Console::AttachConsole(
                windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS,
            );
        }
    }

    let folder = args.folder.unwrap_or_else(|| {
        let executable = std::env::current_exe().unwrap_or_default();
        if let Some(bundle) = executable
            .ancestors()
            .find(|p| p.extension().is_some_and(|s| s == "app"))
        {
            bundle.parent().unwrap_or(bundle).to_path_buf()
        } else {
            executable.parent().unwrap_or(&executable).to_path_buf()
        }
    });

    if args.headless {
        let runtime = tokio::runtime::Runtime::new()?;
        return runtime.block_on(async {
            let folder = gta_browser_launcher::config::resolve_folder(&folder)?;
            let mut settings = gta_browser_launcher::config::Settings::load(&folder)?;
            if let Some(port) = args.port {
                settings.port = port;
            }
            let code = gta_browser_launcher::config::load_code(&folder)?;
            let host = gta_browser_launcher::server::start(&folder, settings, &code).await?;
            println!("Running: {}", host.url());
            if !args.no_open {
                let _ = webbrowser::open(&host.url());
            }
            shutdown_signal().await?;
            host.stop().await;
            Ok(())
        });
    }

    let logo = image::load_from_memory(include_bytes!(concat!(env!("OUT_DIR"), "/app-icon.png")))?
        .into_rgba8();
    let logo = image::imageops::resize(&logo, 256, 256, image::imageops::FilterType::Lanczos3);
    let icon = eframe::egui::IconData {
        rgba: logo.into_raw(),
        width: 256,
        height: 256,
    };
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(gta_browser_launcher::APP_NAME)
            .with_app_id("com.grandv.backendlauncher")
            .with_icon(icon)
            .with_inner_size([1040., 860.])
            .with_min_inner_size([720., 560.]),
        ..Default::default()
    };
    let screenshot = args.ui_screenshot.clone();
    eframe::run_native(
        gta_browser_launcher::APP_NAME,
        options,
        Box::new(move |cc| {
            theme::install(&cc.egui_ctx);
            Ok(Box::new(ui::Launcher::new(
                folder,
                args.port,
                args.ui_smoke,
                args.no_open,
                args.ui_screenshot,
            )))
        }),
    )
    .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    if let Some(path) = screenshot {
        anyhow::ensure!(path.is_file(), "Native screenshot was not saved");
    }
    Ok(())
}

async fn shutdown_signal() -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let mut interrupt = tokio::signal::windows::ctrl_break()?;
        tokio::select! {result=tokio::signal::ctrl_c()=>result,_=interrupt.recv()=>Ok(())}
    }
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {result=tokio::signal::ctrl_c()=>result,_=terminate.recv()=>Ok(())}
    }
}
