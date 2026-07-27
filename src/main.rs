#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod anonymize;
mod extract;
mod mistral;
mod workflow;

use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use workflow::MAX_SOURCE_FILES;
use zeroize::Zeroize;

const SUPPORTED_EXTENSIONS: &[&str] = &["xlsx", "xls", "xlsb"];
const ACCENT: egui::Color32 = egui::Color32::from_rgb(46, 144, 250);
const SUCCESS: egui::Color32 = egui::Color32::from_rgb(53, 190, 128);
const ERROR: egui::Color32 = egui::Color32::from_rgb(244, 91, 105);
const CARD: egui::Color32 = egui::Color32::from_rgb(27, 34, 46);
const BACKGROUND: egui::Color32 = egui::Color32::from_rgb(15, 20, 29);

enum JobMessage {
    Progress(workflow::RunStage),
    Finished(Result<workflow::RunResult, workflow::WorkflowError>),
    Panicked,
}

#[derive(Clone, Copy)]
enum StatusKind {
    Info,
    Success,
    Error,
}

struct ConsolidApp {
    sources: Vec<PathBuf>,
    consolidation: Option<PathBuf>,
    output: Option<PathBuf>,
    api_key: String,
    agent_id: String,
    show_api_key: bool,
    status: String,
    status_kind: StatusKind,
    running: bool,
    receiver: Option<Receiver<JobMessage>>,
    cancellation: Option<workflow::CancellationToken>,
}

impl Default for ConsolidApp {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            consolidation: None,
            output: None,
            api_key: String::new(),
            agent_id: String::new(),
            show_api_key: false,
            status: "Ajoutez les classeurs Excel sources, puis la consolidation à contrôler."
                .into(),
            status_kind: StatusKind::Info,
            running: false,
            receiver: None,
            cancellation: None,
        }
    }
}

impl Drop for ConsolidApp {
    fn drop(&mut self) {
        if let Some(cancellation) = &self.cancellation {
            cancellation.cancel();
        }
        self.api_key.zeroize();
    }
}

impl ConsolidApp {
    fn select_sources(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("Classeurs Excel", SUPPORTED_EXTENSIONS)
            .pick_files()
        {
            self.add_sources(paths);
        }
    }

    fn add_sources(&mut self, paths: impl IntoIterator<Item = PathBuf>) {
        let mut added = 0;
        let mut rejected = 0;
        for path in paths {
            if self.sources.len() >= MAX_SOURCE_FILES
                || !is_supported_file(&path)
                || self
                    .consolidation
                    .as_deref()
                    .is_some_and(|target| same_path(target, &path))
                || self.sources.iter().any(|source| same_path(source, &path))
            {
                rejected += 1;
                continue;
            }
            self.sources.push(path);
            added += 1;
        }
        if added > 0 || rejected > 0 {
            self.set_status(
                if rejected == 0 {
                    format!("{added} pièce(s) source(s) ajoutée(s).")
                } else {
                    format!(
                        "{added} ajoutée(s), {rejected} ignorée(s) : doublon, format, taille ou conflit."
                    )
                },
                if added > 0 {
                    StatusKind::Info
                } else {
                    StatusKind::Error
                },
            );
        }
    }

    fn select_consolidation(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Classeurs Excel", SUPPORTED_EXTENSIONS)
            .pick_file()
        else {
            return;
        };
        if !is_supported_file(&path) {
            self.set_status(
                "Consolidation refusée : seuls les classeurs Excel (.xlsx, .xls, .xlsb) de moins de 50 Mio sont acceptés.",
                StatusKind::Error,
            );
        } else if self.sources.iter().any(|source| same_path(source, &path)) {
            self.set_status(
                "Ce fichier est déjà une pièce source ; choisissez une consolidation distincte.",
                StatusKind::Error,
            );
        } else {
            self.consolidation = Some(path);
            self.set_status("Consolidation sélectionnée.", StatusKind::Info);
        }
    }

    fn select_output(&mut self) {
        let Some(mut path) = rfd::FileDialog::new()
            .add_filter("Classeur Excel", &["xlsx"])
            .set_file_name("consolidation_verifiee.xlsx")
            .save_file()
        else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension("xlsx");
        }
        if !has_output_extension(&path) {
            self.set_status(
                "Sortie refusée : utilisez l’extension .xlsx.",
                StatusKind::Error,
            );
        } else if self.sources.iter().any(|source| same_path(source, &path))
            || self
                .consolidation
                .as_deref()
                .is_some_and(|target| same_path(target, &path))
        {
            self.set_status(
                "Sortie refusée : elle remplacerait un fichier d’entrée.",
                StatusKind::Error,
            );
        } else {
            self.output = Some(path);
            self.set_status("Fichier de sortie sélectionné.", StatusKind::Info);
        }
    }

    fn start(&mut self) {
        if let Some(reason) = self.readiness_issue() {
            self.set_status(reason, StatusKind::Error);
            return;
        }
        let (Some(consolidation), Some(output)) = (self.consolidation.clone(), self.output.clone())
        else {
            self.set_status(
                "Sélection incomplète ; vérifiez les fichiers choisis.",
                StatusKind::Error,
            );
            return;
        };
        let config = workflow::RunConfig {
            sources: self.sources.clone(),
            consolidation,
            output,
            api_key: self.api_key.clone(),
            agent_id: self.agent_id.clone(),
        };
        let cancellation = workflow::CancellationToken::default();
        let worker_cancellation = cancellation.clone();
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        self.cancellation = Some(cancellation);
        self.running = true;
        self.set_status("Démarrage du contrôle…", StatusKind::Info);
        std::thread::spawn(move || {
            let progress_sender = sender.clone();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                workflow::run_with_progress(config, &worker_cancellation, |stage| {
                    let _ = progress_sender.send(JobMessage::Progress(stage));
                })
            }));
            let message = match result {
                Ok(result) => JobMessage::Finished(result),
                Err(_) => JobMessage::Panicked,
            };
            let _ = sender.send(message);
        });
    }

    fn cancel(&mut self) {
        if let Some(cancellation) = &self.cancellation {
            cancellation.cancel();
            self.set_status(
                "Annulation demandée ; la requête réseau en cours doit d’abord se terminer.",
                StatusKind::Info,
            );
        }
    }

    fn reset(&mut self) {
        if self.running {
            return;
        }
        self.sources.clear();
        self.consolidation = None;
        self.output = None;
        self.api_key.zeroize();
        self.api_key.clear();
        self.show_api_key = false;
        self.agent_id.clear();
        self.set_status(
            "Sélection et secret effacés de l’interface.",
            StatusKind::Info,
        );
    }

    fn poll_job(&mut self, context: &egui::Context) {
        while let Some(receiver) = self.receiver.as_ref() {
            let message = receiver.try_recv();
            match message {
                Ok(JobMessage::Progress(stage)) => {
                    let cancellation_requested = self
                        .cancellation
                        .as_ref()
                        .is_some_and(workflow::CancellationToken::is_cancelled);
                    if !cancellation_requested {
                        self.set_status(stage.label(), StatusKind::Info);
                    }
                }
                Ok(JobMessage::Finished(result)) => {
                    self.finish(result);
                    break;
                }
                Ok(JobMessage::Panicked) => {
                    self.running = false;
                    self.receiver = None;
                    self.cancellation = None;
                    self.set_status(
                        "Erreur interne inattendue ; aucun résultat fiable n’a été confirmé.",
                        StatusKind::Error,
                    );
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.running = false;
                    self.receiver = None;
                    self.cancellation = None;
                    self.set_status(
                        "Le traitement s’est interrompu sans résultat.",
                        StatusKind::Error,
                    );
                    break;
                }
            }
        }
        if self.running {
            context.request_repaint_after(std::time::Duration::from_millis(120));
        }
    }

    fn finish(&mut self, result: Result<workflow::RunResult, workflow::WorkflowError>) {
        self.running = false;
        self.receiver = None;
        self.cancellation = None;
        match result {
            Ok(result) => self.set_status(
                format!(
                    "Terminé : {} — {} correspondance(s) restaurée(s).",
                    display_name(&result.output),
                    result.replacements
                ),
                StatusKind::Success,
            ),
            Err(error) if error.is_cancelled() => self.set_status(
                "Traitement annulé ; aucun résultat écrit.",
                StatusKind::Info,
            ),
            Err(error) => self.set_status(format!("Échec : {error}"), StatusKind::Error),
        }
    }

    fn readiness_issue(&self) -> Option<String> {
        if self.sources.is_empty() {
            return Some("Ajoutez au moins une pièce source.".into());
        }
        if self.sources.len() > MAX_SOURCE_FILES {
            return Some(format!(
                "Limitez la sélection à {MAX_SOURCE_FILES} sources."
            ));
        }
        let Some(consolidation) = self.consolidation.as_deref() else {
            return Some("Sélectionnez la consolidation à vérifier.".into());
        };
        let Some(output) = self.output.as_deref() else {
            return Some("Choisissez le fichier de sortie.".into());
        };
        if self
            .sources
            .iter()
            .any(|source| same_path(source, consolidation))
        {
            return Some("La consolidation ne peut pas être une pièce source.".into());
        }
        if same_path(consolidation, output)
            || self.sources.iter().any(|source| same_path(source, output))
        {
            return Some("La sortie ne peut remplacer aucun fichier d’entrée.".into());
        }
        if let Err(error) = mistral::validate_parameters(&self.api_key, self.agent_id.trim()) {
            return Some(error.to_string());
        }
        None
    }

    fn set_status(&mut self, status: impl Into<String>, kind: StatusKind) {
        self.status = status.into();
        self.status_kind = kind;
    }

    fn handle_dropped_files(&mut self, context: &egui::Context) {
        if self.running {
            return;
        }
        let paths = context.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .filter_map(|file| file.path.clone())
                .collect::<Vec<_>>()
        });
        if !paths.is_empty() {
            self.add_sources(paths);
        }
    }

    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("CONSOLID")
                        .size(28.0)
                        .strong()
                        .color(egui::Color32::WHITE),
                );
                ui.label(
                    egui::RichText::new("Audit de consolidation • traitement local protégé")
                        .color(egui::Color32::from_gray(165)),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(!self.running, egui::Button::new("Réinitialiser"))
                    .clicked()
                {
                    self.reset();
                }
                ui.label(
                    egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .monospace()
                        .color(egui::Color32::from_gray(125)),
                );
            });
        });
    }

    fn render_sources(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            section_title(
                ui,
                "1",
                "Pièces sources",
                "Classeurs Excel servant de référence",
            );
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!self.running, egui::Button::new("＋ Ajouter des fichiers"))
                    .clicked()
                {
                    self.select_sources();
                }
                if ui
                    .add_enabled(
                        !self.running && !self.sources.is_empty(),
                        egui::Button::new("Vider la liste"),
                    )
                    .clicked()
                {
                    self.sources.clear();
                    self.set_status("Liste des sources vidée.", StatusKind::Info);
                }
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {} fichier(s)",
                        self.sources.len(),
                        MAX_SOURCE_FILES
                    ))
                    .color(egui::Color32::from_gray(145)),
                );
            });
            ui.small(
                "Glisser-déposer accepté ici • doublons et fichiers non pris en charge ignorés.",
            );
            ui.add_space(6.0);
            if self.sources.is_empty() {
                ui.label(
                    egui::RichText::new("Aucune pièce source")
                        .italics()
                        .color(egui::Color32::from_gray(125)),
                );
            } else {
                egui::ScrollArea::vertical()
                    .id_salt("source-list")
                    .max_height(150.0)
                    .show(ui, |ui| {
                        let mut remove = None;
                        for (index, path) in self.sources.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("{:02}", index + 1))
                                        .monospace()
                                        .color(ACCENT),
                                );
                                ui.label(display_name(path))
                                    .on_hover_text(path.display().to_string());
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .add_enabled(
                                                !self.running,
                                                egui::Button::new("Retirer"),
                                            )
                                            .clicked()
                                        {
                                            remove = Some(index);
                                        }
                                        ui.label(
                                            egui::RichText::new(file_size(path))
                                                .color(egui::Color32::from_gray(125)),
                                        );
                                    },
                                );
                            });
                        }
                        if let Some(index) = remove {
                            self.sources.remove(index);
                            self.set_status("Pièce source retirée.", StatusKind::Info);
                        }
                    });
            }
        });
    }

    fn render_consolidation(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            section_title(
                ui,
                "2",
                "Consolidation existante",
                "Classeur Excel à contrôler et corriger",
            );
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!self.running, egui::Button::new("Sélectionner…"))
                    .clicked()
                {
                    self.select_consolidation();
                }
                path_label(ui, self.consolidation.as_deref());
            });
        });
    }

    fn render_mistral(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            section_title(
                ui,
                "3",
                "Agent Mistral Studio",
                "Secret conservé en mémoire, endpoint fixe en HTTPS, conversation non conservée",
            );
            egui::Grid::new("mistral-fields")
                .num_columns(2)
                .spacing([16.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Clé API");
                    ui.horizontal(|ui| {
                        ui.add_enabled(
                            !self.running,
                            egui::TextEdit::singleline(&mut self.api_key)
                                .password(!self.show_api_key)
                                .hint_text("Saisir la clé Mistral")
                                .desired_width(360.0),
                        );
                        if ui
                            .add_enabled(
                                !self.running,
                                egui::Button::new(if self.show_api_key {
                                    "Masquer"
                                } else {
                                    "Afficher"
                                }),
                            )
                            .clicked()
                        {
                            self.show_api_key = !self.show_api_key;
                        }
                        if ui
                            .add_enabled(
                                !self.running && !self.api_key.is_empty(),
                                egui::Button::new("Effacer"),
                            )
                            .clicked()
                        {
                            self.api_key.zeroize();
                            self.api_key.clear();
                            self.show_api_key = false;
                        }
                    });
                    ui.end_row();

                    ui.label("Agent");
                    ui.add_enabled(
                        !self.running,
                        egui::TextEdit::singleline(&mut self.agent_id)
                            .hint_text("ag_… ou ag:… (identifiant de l’agent)")
                            .desired_width(360.0),
                    )
                    .on_hover_text(
                        "Identifiant de l’agent personnalisé créé dans Mistral Studio. \
                         Le modèle et les instructions système proviennent de l’agent.",
                    );
                    ui.end_row();
                });
            ui.add_space(6.0);
            ui.small(
                "Le modèle, les instructions et les outils sont définis dans l’agent ; \
                 l’application n’envoie que les données protégées.",
            );
        });
    }

    fn render_output(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            section_title(
                ui,
                "4",
                "Résultat",
                "Classeur Excel écrit atomiquement après contrôle des jetons",
            );
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!self.running, egui::Button::new("Choisir la sortie…"))
                    .clicked()
                {
                    self.select_output();
                }
                path_label(ui, self.output.as_deref());
            });
            ui.small("Format de sortie : classeur Excel (.xlsx).");
        });
    }

    fn render_action(&mut self, ui: &mut egui::Ui) {
        let issue = self.readiness_issue();
        let ready = issue.is_none() && !self.running;
        ui.horizontal(|ui| {
            let button = egui::Button::new(
                egui::RichText::new("Vérifier la consolidation")
                    .strong()
                    .color(egui::Color32::WHITE),
            )
            .fill(ACCENT)
            .min_size(egui::vec2(250.0, 42.0));
            let response = ui.add_enabled(ready, button);
            if response.clicked() {
                self.start();
            }
            if let Some(issue) = issue {
                response.on_disabled_hover_text(issue);
            }
            if self.running {
                ui.spinner();
                let cancellation_requested = self
                    .cancellation
                    .as_ref()
                    .is_some_and(workflow::CancellationToken::is_cancelled);
                if ui
                    .add_enabled(
                        !cancellation_requested,
                        egui::Button::new("Annuler le traitement"),
                    )
                    .clicked()
                {
                    self.cancel();
                }
            }
        });
        ui.add_space(8.0);
        let status_color = match self.status_kind {
            StatusKind::Info => egui::Color32::from_rgb(157, 190, 225),
            StatusKind::Success => SUCCESS,
            StatusKind::Error => ERROR,
        };
        egui::Frame::new()
            .fill(status_color.gamma_multiply(0.10))
            .stroke(egui::Stroke::new(1.0, status_color.gamma_multiply(0.55)))
            .corner_radius(8)
            .inner_margin(12)
            .show(ui, |ui| {
                ui.label(egui::RichText::new(&self.status).color(status_color));
            });
    }
}

impl eframe::App for ConsolidApp {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_job(root.ctx());
        self.handle_dropped_files(root.ctx());
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BACKGROUND).inner_margin(0))
            .show(root, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(22.0);
                    ui.horizontal(|ui| {
                        ui.add_space(((ui.available_width() - 860.0) / 2.0).max(16.0));
                        ui.vertical(|ui| {
                            ui.set_max_width(860.0);
                            self.render_header(ui);
                            ui.add_space(18.0);
                            self.render_sources(ui);
                            ui.add_space(12.0);
                            self.render_consolidation(ui);
                            ui.add_space(12.0);
                            self.render_mistral(ui);
                            ui.add_space(12.0);
                            self.render_output(ui);
                            ui.add_space(18.0);
                            self.render_action(ui);
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new(
                                    "Entrées : classeurs Excel (.xlsx, .xls, .xlsb) • sortie : .xlsx",
                                )
                                .small()
                                .color(egui::Color32::from_gray(115)),
                            );
                            ui.add_space(24.0);
                        });
                    });
                });
            });
    }
}

fn configure_style(context: &egui::Context) {
    context.set_theme(egui::Theme::Dark);
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BACKGROUND;
    visuals.window_fill = CARD;
    visuals.extreme_bg_color = egui::Color32::from_rgb(10, 14, 21);
    visuals.selection.bg_fill = ACCENT;
    visuals.hyperlink_color = ACCENT;
    context.set_visuals_of(egui::Theme::Dark, visuals);
    context.style_mut_of(egui::Theme::Dark, |style| {
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
    });
}

fn card(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(CARD)
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(47, 58, 75)))
        .corner_radius(10)
        .inner_margin(16)
        .show(ui, content);
}

fn section_title(ui: &mut egui::Ui, number: &str, title: &str, subtitle: &str) {
    ui.horizontal(|ui| {
        egui::Frame::new()
            .fill(ACCENT.gamma_multiply(0.18))
            .corner_radius(7)
            .inner_margin(egui::Margin::symmetric(9, 5))
            .show(ui, |ui| {
                ui.label(egui::RichText::new(number).strong().color(ACCENT));
            });
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(title).size(17.0).strong());
            ui.label(
                egui::RichText::new(subtitle)
                    .small()
                    .color(egui::Color32::from_gray(135)),
            );
        });
    });
    ui.add_space(10.0);
}

fn path_label(ui: &mut egui::Ui, path: Option<&Path>) {
    match path {
        Some(path) => {
            ui.label(egui::RichText::new(display_name(path)).color(SUCCESS))
                .on_hover_text(path.display().to_string());
            ui.label(
                egui::RichText::new(file_size(path))
                    .small()
                    .color(egui::Color32::from_gray(125)),
            );
        }
        None => {
            ui.label(
                egui::RichText::new("Aucun fichier sélectionné")
                    .italics()
                    .color(egui::Color32::from_gray(125)),
            );
        }
    }
}

fn is_supported_file(path: &Path) -> bool {
    path.is_file()
        && path
            .metadata()
            .is_ok_and(|metadata| metadata.len() <= extract::MAX_INPUT_SIZE)
        && path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| {
                SUPPORTED_EXTENSIONS
                    .iter()
                    .any(|supported| extension.eq_ignore_ascii_case(supported))
            })
}

fn has_output_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("xlsx"))
}

fn same_path(first: &Path, second: &Path) -> bool {
    match (first.canonicalize(), second.canonicalize()) {
        (Ok(first), Ok(second)) => first == second,
        _ => first == second,
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("fichier")
        .to_owned()
}

fn file_size(path: &Path) -> String {
    let Ok(bytes) = path.metadata().map(|metadata| metadata.len()) else {
        return "indisponible".into();
    };
    if bytes >= 1024 * 1024 {
        format!("{:.1} Mio", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} Kio", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} o")
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Consolid")
            .with_inner_size([960.0, 860.0])
            .with_resizable(false)
            .with_maximize_button(false),
        ..Default::default()
    };
    eframe::run_native(
        "Consolid",
        options,
        Box::new(|context| {
            configure_style(&context.egui_ctx);
            Ok(Box::<ConsolidApp>::default())
        }),
    )
}
