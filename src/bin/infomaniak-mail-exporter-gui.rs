//! Binaire GUI : pilote l'export via une interface egui aux couleurs Infomaniak.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, Color32, RichText};
use infomaniak_mail_exporter::runner::{ExportConfig, ExportEvent, run_export};
use infomaniak_mail_exporter::theme;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;

/// Messages remontés du thread d'export vers l'UI.
enum Msg {
    Event(ExportEvent),
    Done(Result<usize, String>),
}

#[derive(PartialEq)]
enum Status {
    Idle,
    Running,
    Done,
    Error,
}

struct App {
    // Formulaire.
    email: String,
    password: String,
    output: String,
    server: String,
    port: u16,
    folders_csv: String,
    limit: usize,
    batch_size: usize,

    // Exécution.
    status: Status,
    current_folder: String,
    folder_done: usize,
    folder_total: usize,
    exported: usize,
    result_msg: String,
    log: Vec<String>,

    rx: Option<Receiver<Msg>>,
    cancel: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Default for App {
    fn default() -> Self {
        App {
            email: String::new(),
            password: String::new(),
            output: "./export".to_string(),
            server: "mail.infomaniak.com".to_string(),
            port: 993,
            folders_csv: String::new(),
            limit: 0,
            batch_size: 50,
            status: Status::Idle,
            current_folder: String::new(),
            folder_done: 0,
            folder_total: 0,
            exported: 0,
            result_msg: String::new(),
            log: Vec::new(),
            rx: None,
            cancel: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        theme::install_fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx);
        App::default()
    }

    fn push_log(&mut self, line: impl Into<String>) {
        self.log.push(line.into());
        // Garde les ~300 dernières lignes.
        if self.log.len() > 300 {
            let drain = self.log.len() - 300;
            self.log.drain(0..drain);
        }
    }

    fn can_start(&self) -> bool {
        !self.email.trim().is_empty() && !self.password.is_empty() && !self.output.trim().is_empty()
    }

    fn start_export(&mut self) {
        let folders: Vec<String> = self
            .folders_csv
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let cfg = ExportConfig {
            email: self.email.trim().to_string(),
            password: self.password.clone(),
            output: self.output.trim().into(),
            server: self.server.trim().to_string(),
            port: self.port,
            folders,
            limit: self.limit,
            batch_size: self.batch_size,
        };

        self.cancel = Arc::new(AtomicBool::new(false));
        let cancel = Arc::clone(&self.cancel);
        let (tx, rx): (Sender<Msg>, Receiver<Msg>) = channel();
        self.rx = Some(rx);

        self.status = Status::Running;
        self.exported = 0;
        self.folder_done = 0;
        self.folder_total = 0;
        self.current_folder.clear();
        self.result_msg.clear();
        self.log.clear();

        let tx_events = tx.clone();
        self.handle = Some(std::thread::spawn(move || {
            let result = run_export(&cfg, &cancel, |ev| {
                let _ = tx_events.send(Msg::Event(ev));
            });
            let _ = tx.send(Msg::Done(result.map_err(|e| format!("{e:#}"))));
        }));
    }

    fn drain_messages(&mut self) {
        let Some(rx) = &self.rx else { return };
        let mut messages = Vec::new();
        while let Ok(m) = rx.try_recv() {
            messages.push(m);
        }
        for m in messages {
            match m {
                Msg::Event(ev) => self.handle_event(ev),
                Msg::Done(Ok(total)) => {
                    self.status = Status::Done;
                    self.exported = total;
                    self.result_msg = format!("{total} nouveau(x) mail(s) exporté(s).");
                    self.rx = None;
                    self.handle = None;
                }
                Msg::Done(Err(e)) => {
                    // Annulation utilisateur : ne pas afficher comme une erreur.
                    if self.cancel.load(Ordering::Relaxed) {
                        self.status = Status::Done;
                        self.result_msg =
                            format!("Export annulé. {} mail(s) exporté(s).", self.exported);
                    } else {
                        self.status = Status::Error;
                        self.result_msg = e.clone();
                        self.push_log(format!("Erreur : {e}"));
                    }
                    self.rx = None;
                    self.handle = None;
                }
            }
        }
    }

    fn handle_event(&mut self, ev: ExportEvent) {
        match ev {
            ExportEvent::Connecting { server, port } => {
                self.push_log(format!("Connexion à {server}:{port}…"));
            }
            ExportEvent::FoldersListed { count } => {
                self.push_log(format!("{count} dossier(s) à exporter."));
            }
            ExportEvent::FolderStarted {
                name,
                total,
                already,
            } => {
                self.current_folder = name.clone();
                self.folder_total = total;
                self.folder_done = 0;
                self.push_log(format!(
                    "→ {name} : {already} déjà exporté(s), {total} à télécharger"
                ));
            }
            ExportEvent::Mail { subject, .. } => {
                self.folder_done += 1;
                self.exported += 1;
                self.push_log(format!("  ✓ {subject}"));
            }
            ExportEvent::FolderSkipped { name, reason } => {
                self.push_log(format!("  ! {name} ignoré : {reason}"));
            }
            ExportEvent::FolderDone { name } => {
                self.push_log(format!("✓ {name} terminé"));
            }
            ExportEvent::Finished { total } => {
                self.push_log(format!("Terminé : {total} mail(s) exporté(s)."));
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.status == Status::Running {
            self.drain_messages();
            ctx.request_repaint();
        }

        sidebar(ctx);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::BG_SOFT).inner_margin(24.0))
            .show(ctx, |ui| {
                ui.heading(RichText::new("Exporter mes mails").color(theme::TEXT));
                ui.add_space(2.0);
                ui.label(
                    RichText::new("Sauvegarde non-destructive de tous vos dossiers IMAP en .eml")
                        .color(theme::MUTED),
                );
                ui.add_space(16.0);

                self.form_card(ui);
                ui.add_space(14.0);
                self.action_row(ui);

                if self.status != Status::Idle {
                    ui.add_space(14.0);
                    self.progress_card(ui);
                }
            });
    }
}

fn sidebar(ctx: &egui::Context) {
    egui::SidePanel::left("sidebar")
        .exact_width(168.0)
        .resizable(false)
        .frame(egui::Frame::new().fill(theme::CARD).inner_margin(18.0))
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(18.0);
                ui.add(
                    egui::Image::new(egui::include_image!("../../assets/mail.svg")).max_width(64.0),
                );
                ui.add_space(12.0);
                ui.label(
                    RichText::new("Mail Exporter")
                        .font(theme::semibold(16.0))
                        .color(theme::TEXT),
                );
                ui.label(
                    RichText::new("Infomaniak")
                        .font(theme::semibold(12.5))
                        .color(theme::PINK),
                );
            });

            // Version en pied de barre latérale.
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                        .small()
                        .color(theme::MUTED),
                );
            });
        });
}

impl App {
    fn form_card(&mut self, ui: &mut egui::Ui) {
        let enabled = self.status != Status::Running;
        card(ui, |ui| {
            ui.add_enabled_ui(enabled, |ui| {
                labeled(ui, "Adresse email", |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.email)
                            .hint_text("vous@votredomaine.ch")
                            .desired_width(f32::INFINITY),
                    );
                });
                labeled(ui, "Mot de passe", |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.password)
                            .password(true)
                            .desired_width(f32::INFINITY),
                    );
                });
                labeled(ui, "Dossier de sortie", |ui| {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.output)
                                .desired_width(ui.available_width() - 110.0),
                        );
                        if ui.button("Parcourir…").clicked()
                            && let Some(path) = rfd::FileDialog::new().pick_folder()
                        {
                            self.output = path.display().to_string();
                        }
                    });
                });

                ui.add_space(6.0);
                ui.collapsing(RichText::new("Avancé").color(theme::MUTED), |ui| {
                    labeled(ui, "Serveur IMAP", |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.server)
                                .desired_width(f32::INFINITY),
                        );
                    });
                    labeled(ui, "Port", |ui| {
                        ui.add(egui::DragValue::new(&mut self.port));
                    });
                    labeled(ui, "Dossiers (vide = tous, séparés par ,)", |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.folders_csv)
                                .hint_text("INBOX, Sent")
                                .desired_width(f32::INFINITY),
                        );
                    });
                    labeled(ui, "Limite par dossier (0 = illimité)", |ui| {
                        ui.add(egui::DragValue::new(&mut self.limit));
                    });
                    labeled(ui, "Taille de lot", |ui| {
                        ui.add(egui::DragValue::new(&mut self.batch_size).range(1..=200));
                    });
                });
            });
        });
    }

    fn action_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if self.status == Status::Running {
                let cancel = egui::Button::new(
                    RichText::new("Annuler")
                        .font(theme::semibold(14.5))
                        .color(Color32::WHITE),
                )
                .fill(theme::PINK_DARK)
                .corner_radius(10.0)
                .min_size(egui::vec2(140.0, 40.0));
                if ui.add(cancel).clicked() {
                    self.cancel.store(true, Ordering::Relaxed);
                    self.push_log("Annulation demandée…");
                }
                ui.add_space(8.0);
                ui.spinner();
            } else {
                let start = egui::Button::new(
                    RichText::new("Exporter")
                        .font(theme::semibold(15.0))
                        .color(Color32::WHITE),
                )
                .fill(theme::PINK)
                .corner_radius(10.0)
                .min_size(egui::vec2(170.0, 42.0));
                if ui.add_enabled(self.can_start(), start).clicked() {
                    self.start_export();
                }
                if !self.can_start() {
                    ui.label(
                        RichText::new("Renseignez email, mot de passe et dossier")
                            .color(theme::MUTED),
                    );
                }
            }
        });
    }

    fn progress_card(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            match self.status {
                Status::Running => {
                    let frac = if self.folder_total > 0 {
                        self.folder_done as f32 / self.folder_total as f32
                    } else {
                        0.0
                    };
                    let label = if self.current_folder.is_empty() {
                        "Connexion…".to_string()
                    } else {
                        format!(
                            "{} — {}/{}",
                            self.current_folder, self.folder_done, self.folder_total
                        )
                    };
                    ui.add(egui::ProgressBar::new(frac).fill(theme::PINK).text(label));
                    ui.label(
                        RichText::new(format!("{} mail(s) exporté(s) au total", self.exported))
                            .color(theme::MUTED),
                    );
                }
                Status::Done => {
                    ui.label(
                        RichText::new(format!("✓ {}", self.result_msg))
                            .color(theme::PINK_DARK)
                            .strong(),
                    );
                }
                Status::Error => {
                    ui.label(
                        RichText::new(format!("✗ {}", self.result_msg))
                            .color(Color32::from_rgb(0xC0, 0x30, 0x30))
                            .strong(),
                    );
                }
                Status::Idle => {}
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(RichText::new("Journal").color(theme::MUTED).small());
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for line in &self.log {
                        ui.label(RichText::new(line).monospace().size(12.0));
                    }
                });
        });
    }
}

/// Carte blanche arrondie.
fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(theme::CARD)
        .corner_radius(12.0)
        .inner_margin(18.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
}

/// Champ avec libellé au-dessus.
fn labeled(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(8.0);
    ui.label(
        RichText::new(label)
            .font(theme::semibold(12.5))
            .color(theme::TEXT),
    );
    ui.add_space(2.0);
    add(ui);
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([680.0, 540.0])
            .with_min_inner_size([560.0, 460.0])
            .with_title("Infomaniak Mail Exporter"),
        ..Default::default()
    };
    eframe::run_native(
        "Infomaniak Mail Exporter",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
