use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

use eframe::egui::{self, Color32, RichText, TextEdit, TextStyle};

use crate::{
    config::AppConfig,
    provider::{ChatMessage, ProviderClient},
    tools,
};

pub struct PlasmaApp {
    workspace: PathBuf,
    config: AppConfig,
    files: Vec<PathBuf>,
    active_file: Option<PathBuf>,
    editor_text: String,
    input: String,
    messages: Vec<ChatMessage>,
    status: String,
    busy: bool,
    response_rx: Option<Receiver<Result<String, String>>>,
    show_settings: bool,
}

impl PlasmaApp {
    pub fn new(workspace: PathBuf, config: AppConfig) -> Self {
        let files = tools::list_text_files(&workspace);
        let mut app = Self {
            workspace,
            config,
            files,
            active_file: None,
            editor_text: String::new(),
            input: String::new(),
            messages: vec![ChatMessage { role: "assistant".to_owned(), content: "Welcome to NS Plasma. Add an OpenAI-compatible provider in Settings, then ask Plasma to help with your project.".to_owned() }],
            status: "Ready".to_owned(),
            busy: false,
            response_rx: None,
            show_settings: false,
        };
        if let Some(first) = app.files.first().cloned() {
            app.open_file(first);
        }
        app
    }

    fn open_file(&mut self, path: PathBuf) {
        match tools::read_text(&self.workspace, &path) {
            Ok(text) => {
                self.active_file = Some(path);
                self.editor_text = text;
                self.status = "File opened".to_owned();
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    fn save_file(&mut self) {
        let Some(path) = self.active_file.as_ref() else {
            return;
        };
        match tools::write_text(&self.workspace, path, &self.editor_text) {
            Ok(()) => {
                self.status = "Saved".to_owned();
                self.files = tools::list_text_files(&self.workspace);
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    fn send_message(&mut self) {
        let prompt = self.input.trim().to_owned();
        if prompt.is_empty() || self.busy {
            return;
        }
        let Some(provider) = self.config.provider.clone() else {
            self.messages.push(ChatMessage {
                role: "user".to_owned(),
                content: prompt,
            });
            self.messages.push(ChatMessage { role: "assistant".to_owned(), content: "No provider is configured yet. Open Settings → Provider and enter a real endpoint. Plasma does not invent model lists or credentials.".to_owned() });
            self.input.clear();
            return;
        };
        let mut request_messages = self.messages.clone();
        request_messages.push(ChatMessage {
            role: "user".to_owned(),
            content: prompt.clone(),
        });
        self.messages.push(ChatMessage {
            role: "user".to_owned(),
            content: prompt,
        });
        self.input.clear();
        self.busy = true;
        self.status = "Working…".to_owned();
        let (tx, rx) = mpsc::channel();
        self.response_rx = Some(rx);
        thread::spawn(move || {
            let result = ProviderClient::new(provider)
                .and_then(|client| client.chat(&request_messages))
                .map_err(|error| error.to_string());
            let _ = tx.send(result);
        });
    }

    fn poll_response(&mut self) {
        let Some(rx) = self.response_rx.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(text)) => {
                self.messages.push(ChatMessage {
                    role: "assistant".to_owned(),
                    content: text,
                });
                self.status = "Ready".to_owned();
                self.busy = false;
                self.response_rx = None;
            }
            Ok(Err(error)) => {
                self.messages.push(ChatMessage {
                    role: "assistant".to_owned(),
                    content: format!("Provider error: {error}"),
                });
                self.status = "Provider error".to_owned();
                self.busy = false;
                self.response_rx = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.messages.push(ChatMessage {
                    role: "assistant".to_owned(),
                    content: "The provider worker stopped before returning a response.".to_owned(),
                });
                self.status = "Worker stopped".to_owned();
                self.busy = false;
                self.response_rx = None;
            }
        }
    }

    fn settings_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.label("Provider");
        let mut provider = self.config.provider.clone().unwrap_or_default();
        ui.add(TextEdit::singleline(&mut provider.name).hint_text("Name"));
        ui.add(
            TextEdit::singleline(&mut provider.base_url)
                .hint_text("Base URL, e.g. http://localhost:11434/v1"),
        );
        ui.add(TextEdit::singleline(&mut provider.model).hint_text("Model ID from the provider"));
        let mut api_key_env = provider.api_key_env.clone().unwrap_or_default();
        ui.add(
            TextEdit::singleline(&mut api_key_env)
                .hint_text("API key environment variable (optional)"),
        );
        provider.api_key_env = (!api_key_env.trim().is_empty()).then_some(api_key_env);
        self.config.provider = Some(provider);
        if ui.button("Save settings").clicked() {
            match self.config.save(&self.workspace) {
                Ok(()) => {
                    self.status = format!("Saved {}", AppConfig::path(&self.workspace).display())
                }
                Err(error) => self.status = error.to_string(),
            }
        }
        ui.separator();
        ui.label(RichText::new("Secrets stay outside settings.json. Set the named environment variable before testing a remote provider.").small().color(Color32::GRAY));
    }
}

impl eframe::App for PlasmaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_response();
        ctx.request_repaint_after(Duration::from_millis(100));
        let dark = self.config.ui.theme != "plasma-light";
        if dark {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::Image::from_bytes("embedded://ns-plasma-logo", crate::LOGO_BYTES)
                        .fit_to_exact_size(egui::vec2(36.0, 36.0)),
                );
                ui.heading(RichText::new("NS Plasma").strong());
                ui.label(RichText::new("Plasma").color(Color32::from_rgb(140, 168, 255)));
                ui.separator();
                ui.label(self.workspace.display().to_string());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙ Settings").clicked() {
                        self.show_settings = !self.show_settings;
                    }
                    ui.label(RichText::new(&self.status).small().color(Color32::GRAY));
                });
            });
        });

        egui::SidePanel::left("explorer")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Explorer");
                ui.small("Project files");
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for path in self.files.clone() {
                        let selected = self.active_file.as_ref() == Some(&path);
                        let label = path
                            .strip_prefix(&self.workspace)
                            .unwrap_or(&path)
                            .display()
                            .to_string();
                        if ui.selectable_label(selected, label).clicked() {
                            self.open_file(path);
                        }
                    }
                });
            });

        egui::SidePanel::right("agent")
            .resizable(true)
            .default_width(390.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Agent");
                    if self.busy {
                        ui.spinner();
                    }
                });
                ui.small(
                    self.config
                        .provider
                        .as_ref()
                        .map(|p| {
                            format!(
                                "{} · {}",
                                p.name,
                                if p.model.is_empty() {
                                    "model not selected"
                                } else {
                                    &p.model
                                }
                            )
                        })
                        .unwrap_or_else(|| "No provider configured".to_owned()),
                );
                ui.separator();
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for message in &self.messages {
                            let is_user = message.role == "user";
                            let color = if is_user {
                                Color32::from_rgb(140, 168, 255)
                            } else {
                                ui.visuals().text_color()
                            };
                            ui.label(
                                RichText::new(if is_user { "You" } else { "Plasma" })
                                    .strong()
                                    .color(color),
                            );
                            ui.label(&message.content);
                            ui.add_space(10.0);
                        }
                    });
                ui.separator();
                let send = ui.add_sized(
                    [ui.available_width(), 80.0],
                    TextEdit::multiline(&mut self.input)
                        .hint_text("Ask Plasma… (Ctrl+Enter to send)")
                        .desired_rows(3),
                );
                if send.has_focus()
                    && ui.input(|input| input.key_pressed(egui::Key::Enter) && input.modifiers.ctrl)
                {
                    self.send_message();
                }
                if ui.button("Send").clicked() {
                    self.send_message();
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.show_settings {
                self.settings_panel(ui);
            } else {
                ui.horizontal(|ui| {
                    ui.heading(
                        self.active_file
                            .as_ref()
                            .map(|p| p.file_name().and_then(|n| n.to_str()).unwrap_or("Editor"))
                            .unwrap_or("Editor"),
                    );
                    if ui.button("Save").clicked() {
                        self.save_file();
                    }
                });
                ui.separator();
                ui.add_sized(
                    ui.available_size(),
                    TextEdit::multiline(&mut self.editor_text)
                        .font(TextStyle::Monospace)
                        .desired_rows(30)
                        .lock_focus(true),
                );
            }
        });
    }
}

pub fn run(workspace: &Path, config: AppConfig) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("NS Plasma")
            .with_inner_size([1500.0, 900.0])
            .with_min_inner_size([1000.0, 650.0]),
        ..Default::default()
    };
    let root = workspace.to_path_buf();
    eframe::run_native(
        "NS Plasma",
        options,
        Box::new(move |_cc| Ok(Box::new(PlasmaApp::new(root, config)))),
    )
}
