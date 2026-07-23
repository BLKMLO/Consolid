mod anonymize;
mod extract;
mod mistral;
mod workflow;

use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use zeroize::Zeroize;

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "csv", "xlsx", "xls", "xlsb", "ods", "docx", "json", "txt", "md",
];

enum JobMessage {
    Finished(Result<workflow::RunResult, workflow::WorkflowError>),
}

struct ConsolidApp {
    sources: Vec<PathBuf>,
    consolidation: Option<PathBuf>,
    output: Option<PathBuf>,
    api_key: String,
    model: String,
    status: String,
    running: bool,
    receiver: Option<Receiver<JobMessage>>,
}

impl Default for ConsolidApp {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            consolidation: None,
            output: None,
            api_key: String::new(),
            model: "mistral-small-latest".into(),
            status: "Sélectionnez les pièces sources et la consolidation à vérifier.".into(),
            running: false,
            receiver: None,
        }
    }
}

impl Drop for ConsolidApp {
    fn drop(&mut self) {
        self.api_key.zeroize();
    }
}

impl ConsolidApp {
    fn select_sources(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("Documents pris en charge", SUPPORTED_EXTENSIONS)
            .pick_files()
        {
            for path in paths {
                if !self.sources.contains(&path) {
                    self.sources.push(path);
                }
            }
        }
    }

    fn select_consolidation(&mut self) {
        self.consolidation = rfd::FileDialog::new()
            .add_filter("Documents pris en charge", SUPPORTED_EXTENSIONS)
            .pick_file();
    }

    fn select_output(&mut self) {
        self.output = rfd::FileDialog::new()
            .add_filter("Markdown", &["md"])
            .add_filter("Texte", &["txt"])
            .set_file_name("consolidation_verifiee.md")
            .save_file();
    }

    fn start(&mut self) {
        let Some(consolidation) = self.consolidation.clone() else {
            self.status = "Erreur : sélectionnez la consolidation à vérifier.".into();
            return;
        };
        let Some(output) = self.output.clone() else {
            self.status = "Erreur : sélectionnez le fichier de sortie.".into();
            return;
        };
        if self.sources.is_empty() {
            self.status = "Erreur : ajoutez au moins une pièce source.".into();
            return;
        }
        if self.api_key.trim().is_empty() {
            self.status = "Erreur : renseignez la clé API Mistral.".into();
            return;
        }

        let config = workflow::RunConfig {
            sources: self.sources.clone(),
            consolidation,
            output,
            api_key: self.api_key.clone(),
            model: self.model.clone(),
        };
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        self.running = true;
        self.status = "Extraction et pseudonymisation locales, puis analyse Mistral…".into();
        std::thread::spawn(move || {
            let _ = sender.send(JobMessage::Finished(workflow::run(config)));
        });
    }

    fn poll_job(&mut self, context: &egui::Context) {
        let message = self
            .receiver
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        if let Some(JobMessage::Finished(result)) = message {
            self.running = false;
            self.receiver = None;
            self.status = match result {
                Ok(result) => format!(
                    "Terminé : {} ({} correspondances restaurées).",
                    display_name(&result.output),
                    result.replacements
                ),
                Err(error) => format!("Erreur : {error}"),
            };
        }
        if self.running {
            context.request_repaint_after(std::time::Duration::from_millis(150));
        }
    }
}

impl eframe::App for ConsolidApp {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_job(root.ctx());
        egui::CentralPanel::default().show(root, |ui| {
            ui.heading("Consolid");
            ui.label("Audit de consolidation avec pseudonymisation locale");
            ui.separator();

            ui.horizontal(|ui| {
                ui.heading("1. Pièces sources");
                if ui
                    .add_enabled(!self.running, egui::Button::new("Ajouter…"))
                    .clicked()
                {
                    self.select_sources();
                }
                if ui
                    .add_enabled(
                        !self.running && !self.sources.is_empty(),
                        egui::Button::new("Vider"),
                    )
                    .clicked()
                {
                    self.sources.clear();
                }
            });
            egui::ScrollArea::vertical()
                .max_height(130.0)
                .show(ui, |ui| {
                    let mut remove = None;
                    for (index, path) in self.sources.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{} — {}", index + 1, display_name(path)));
                            if ui
                                .add_enabled(!self.running, egui::Button::new("Retirer"))
                                .clicked()
                            {
                                remove = Some(index);
                            }
                        });
                    }
                    if let Some(index) = remove {
                        self.sources.remove(index);
                    }
                });

            ui.separator();
            ui.horizontal(|ui| {
                ui.heading("2. Consolidation existante");
                if ui
                    .add_enabled(!self.running, egui::Button::new("Sélectionner…"))
                    .clicked()
                {
                    self.select_consolidation();
                }
            });
            ui.label(
                self.consolidation
                    .as_deref()
                    .map(display_name)
                    .unwrap_or_else(|| "Aucun fichier".into()),
            );

            ui.separator();
            ui.heading("3. Mistral");
            ui.horizontal(|ui| {
                ui.label("Clé API");
                ui.add_enabled(
                    !self.running,
                    egui::TextEdit::singleline(&mut self.api_key)
                        .password(true)
                        .desired_width(360.0),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Modèle");
                ui.add_enabled(
                    !self.running,
                    egui::TextEdit::singleline(&mut self.model).desired_width(240.0),
                );
            });
            ui.small("La clé et la table de correspondance restent uniquement en mémoire.");

            ui.separator();
            ui.horizontal(|ui| {
                ui.heading("4. Résultat");
                if ui
                    .add_enabled(!self.running, egui::Button::new("Choisir la sortie…"))
                    .clicked()
                {
                    self.select_output();
                }
            });
            ui.label(
                self.output
                    .as_deref()
                    .map(display_name)
                    .unwrap_or_else(|| "Aucun fichier".into()),
            );

            ui.add_space(12.0);
            if ui
                .add_enabled(
                    !self.running,
                    egui::Button::new("Vérifier la consolidation"),
                )
                .clicked()
            {
                self.start();
            }
            if self.running {
                ui.spinner();
            }
            ui.separator();
            ui.label(&self.status);
            ui.small("Formats : CSV, XLSX/XLS/XLSB/ODS, DOCX, JSON, TXT et MD. PDF refusé.");
        });
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("fichier")
        .to_owned()
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Consolid")
            .with_inner_size([760.0, 720.0])
            .with_min_inner_size([620.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Consolid",
        options,
        Box::new(|_context| Ok(Box::<ConsolidApp>::default())),
    )
}
