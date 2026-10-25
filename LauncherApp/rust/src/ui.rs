
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
