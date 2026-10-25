                ui.label(
                    egui::RichText::new(gta_browser_launcher::APP_NAME)
                        .size(theme::FONT_TITLE)
                        .strong()
                        .color(theme::TEXT),
                );
                ui.label(
                    egui::RichText::new("Serves the game files to phones and tablets on your network.")
                        .size(theme::FONT_SMALL)
                        .color(theme::TEXT_DIM),
                );
                if theme::button_w(ui, "Console output", 170.).clicked() {
                    let open = !self.console.open.load(Ordering::Relaxed);
                    self.console.open.store(open, Ordering::Relaxed);
                }
            });
            theme::hint(ui, "Start the server, then open the Network tab for the addresses.");
        });
    }

    fn game_tab(&mut self, ui: &mut egui::Ui) {
        theme::section(ui, "Get started");
        theme::wide_card(ui, |ui| {
            ui.label(
                egui::RichText::new(
                    "Choose the folder holding the extracted game files, press Start server, then open the game link.",
                )
                .size(theme::FONT_BODY)
                .color(theme::TEXT),
            );
            ui.add_space(theme::SPACE_S);
            ui.horizontal_wrapped(|ui| {
                if theme::button_w(ui, "Need to obtain assets?", 210.).clicked() {
                    self.obtain_assets = true;
                }
                if theme::button_w(ui, "Updates and info", 170.).clicked() {
                    self.updates = true;
                }
                if theme::button_w(ui, "Optional donation", 170.).clicked() {
                    self.donate = true;
                }
            });
        });

        ui.add_space(theme::SPACE_L);
        theme::section(ui, "Session");
        theme::wide_card(ui, |ui| {
            match &self.host {
                Some(host) => {
                    let mut copy: Option<String> = None;
                    theme::value_row(ui, "URL", &host.url(), &mut copy);
                    if let Some(value) = copy {
                        ui.ctx().copy_text(value);
                    }
                    ui.add_space(theme::SPACE_XS);
                    theme::hint(
                        ui,
                        format!(
                            "Listening on port {}. Audience: {}.",
                            host.port(),
                            match host.settings.mode.as_str() {
                                "lan" => "LAN / school network",
                                "external" => "Public website",
                                _ => "This PC",
                            }
                        ),
                    );
                }
                None => {
                    theme::note(
                        ui,
                        Level::Idle,
                        "The server is stopped. Press Start server in the header.",
                    );
                }
            }
        });
    }
