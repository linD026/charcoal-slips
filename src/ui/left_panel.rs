use crate::CCslipsApp;
use crate::fileops::FileOperation;
use eframe::egui;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

// Action enum to handle right-clicks and regular clicks
pub enum TreeAction {
    OpenFile(PathBuf),
    CreateFile(PathBuf),
    CreateDir(PathBuf),
    Delete(HashSet<PathBuf>),
}

pub fn render_dir_tree(
    ui: &mut egui::Ui,
    path: &Path,
    current_file: &Option<PathBuf>,
) -> Option<TreeAction> {
    let mut action = None;
    if let Ok(entries) = fs::read_dir(path) {
        let mut dirs = Vec::new();
        let mut files = Vec::new();
        for entry in entries.flatten() {
            let p = entry.path();
            if p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .starts_with('.')
            {
                continue;
            }
            if p.is_dir() {
                dirs.push(p);
            } else {
                files.push(p);
            }
        }
        dirs.sort();
        files.sort();

        for d in dirs {
            let name = d
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let header = egui::CollapsingHeader::new(format!("📁 {}", name)).default_open(false);

            let response = header
                .show(ui, |ui| {
                    if let Some(res) = render_dir_tree(ui, &d, current_file) {
                        action = Some(res);
                    }
                })
                .header_response;

            response.context_menu(|ui| {
                if ui.button("📄+ New File Here").clicked() {
                    action = Some(TreeAction::CreateFile(d.clone()));
                    ui.close_menu();
                }
                if ui.button("📁+ New Dir Here").clicked() {
                    action = Some(TreeAction::CreateDir(d.clone()));
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("🗑 Delete Directory").clicked() {
                    let mut set = HashSet::new();
                    set.insert(d.clone());
                    action = Some(TreeAction::Delete(set));
                    ui.close_menu();
                }
            });
        }

        for f in files {
            let name = f
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let is_selected = current_file.as_ref() == Some(&f);

            let response = ui.selectable_label(is_selected, format!("📄 {}", name));
            if response.clicked() {
                action = Some(TreeAction::OpenFile(f.clone()));
            }

            response.context_menu(|ui| {
                if ui.button("🗑 Delete File").clicked() {
                    let mut set = HashSet::new();
                    set.insert(f.clone());
                    action = Some(TreeAction::Delete(set));
                    ui.close_menu();
                }
            });
        }
    }
    action
}

impl CCslipsApp {
    pub fn render_left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(self.config.ui.left_panel_width)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Workspace");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let target_dir = if let Some(current) = &self.current_file {
                            if current.is_dir() {
                                current.clone()
                            } else if let Some(parent) = current.parent() {
                                parent.to_path_buf()
                            } else {
                                PathBuf::from(&self.config.build.working_directory)
                            }
                        } else {
                            PathBuf::from(&self.config.build.working_directory)
                        };

                        if ui.button("🗑").on_hover_text("Delete Items").clicked() {
                            self.active_file_op = FileOperation::Delete(HashSet::new());
                        }
                        if ui
                            .button("📁+")
                            .on_hover_text("New Folder in Current Dir")
                            .clicked()
                        {
                            self.active_file_op = FileOperation::CreateDir(target_dir.clone());
                            self.file_op_input.clear();
                        }
                        if ui
                            .button("📄+")
                            .on_hover_text("New File in Current Dir")
                            .clicked()
                        {
                            self.active_file_op = FileOperation::CreateFile(target_dir);
                            self.file_op_input.clear();
                        }
                    });
                });
                ui.separator();

                if self.search_state.is_active {
                    egui::TopBottomPanel::bottom("search_replace_panel")
                        .resizable(false)
                        .show_inside(ui, |ui| {
                            ui.add_space(4.0);
                            self.render_search_replace_panel(ui);
                            ui.add_space(4.0);
                        });
                }

                egui::CentralPanel::default().show_inside(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        if let Some(action) = render_dir_tree(
                            ui,
                            Path::new(&self.config.build.working_directory),
                            &self.current_file,
                        ) {
                            match action {
                                TreeAction::OpenFile(path) => self.open_file(path, false),
                                TreeAction::CreateFile(path) => {
                                    self.active_file_op = FileOperation::CreateFile(path);
                                    self.file_op_input.clear();
                                }
                                TreeAction::CreateDir(path) => {
                                    self.active_file_op = FileOperation::CreateDir(path);
                                    self.file_op_input.clear();
                                }
                                TreeAction::Delete(path) => {
                                    self.active_file_op = FileOperation::Delete(path);
                                }
                            }
                        }
                    });
                });
            });
    }
}
