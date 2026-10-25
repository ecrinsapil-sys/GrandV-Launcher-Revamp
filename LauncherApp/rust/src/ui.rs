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
        ui.horizontal_wrapped(|ui| {
            if theme::button_w(ui, "Clear", 100.).clicked() {
                gta_browser_launcher::diagnostics::clear();
            }
            let mut follow = console.follow.load(Ordering::Relaxed);
            if ui
                .add_sized(
                    [140., theme::CONTROL_H],
                    egui::Checkbox::new(&mut follow, "Follow output"),
                )
                .changed()
            {
                console.follow.store(follow, Ordering::Relaxed);
            }
            if theme::button_w(ui, "Copy output", 140.).clicked() {
                ui.ctx()
                    .copy_text(gta_browser_launcher::diagnostics::snapshot());
            }
        });
