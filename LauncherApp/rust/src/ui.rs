    // -- drawing ------------------------------------------------------------

    fn header(&mut self, ui: &mut egui::Ui) {
        theme::wide_card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
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
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if theme::button_w(ui, "Troubleshooting", 150.).clicked() {
                        self.help = true;
                    }
                });
            });

            ui.add_space(theme::SPACE_M);
            theme::field(ui, "Game folder", 150., |ui| {
                let width = (ui.available_width() - 130.).max(200.);
                let mut folder = self.folder.clone();
                let changed = ui
                    .add_sized(
                        [width, theme::CONTROL_H],
                        egui::TextEdit::singleline(&mut folder)
                            .hint_text("Select the extracted GrandV-assets folder"),
                    )
                    .changed();
                if changed {
                    self.folder = folder;
                }
                if theme::button_w(ui, "Browse…", 110.).clicked() {
                    self.pick_folder();
                }
            });

            let valid = self.folder_is_valid();
            theme::validity_row(
                ui,
                valid,
                if valid {
                    "Game folder looks valid."
                } else {
                    "The folder must contain mirror/playgta5.com/launcher.html."
                },
            );

            ui.add_space(theme::SPACE_M);
            ui.horizontal_wrapped(|ui| {
                let running = self.host.is_some();
                let clicked = theme::action(
                    ui,
                    if running { "Stop server" } else { "Start server" },
                    180.,
                    if running { theme::STOP_BG } else { theme::PRIMARY_BG },
                    if running { theme::STOP_BG_HOVER } else { theme::PRIMARY_BG_HOVER },
                )
                .clicked();
                if clicked {
                    if running {
                        self.stop();
                    } else {
                        self.start(true);
                    }
                }
                let can_open = self.host.is_some();
                ui.add_enabled_ui(can_open, |ui| {
                    if theme::action(ui, "Open game", 150., theme::OPEN_BG, theme::OPEN_BG_HOVER).clicked()
                        && let Some(host) = &self.host
                    {
                        let _ = webbrowser::open(&host.url());
                    }
                });
                if theme::button_w(ui, "Console output", 160.).clicked() {
                    let open = !self.console.open.load(Ordering::Relaxed);
                    self.console.open.store(open, Ordering::Relaxed);
                }
            });

            ui.add_space(theme::SPACE_S);
            theme::status_pill(ui, self.level, &self.status);
            if self.fault.is_some() && theme::button_w(ui, "Error details", 150.).clicked() {
                self.fault_open = true;
            }
        });
    }
