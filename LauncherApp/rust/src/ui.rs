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
