use crate::CCslipsApp;
use crate::fileops::FileOperation;
use eframe::egui;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

// Renders a checkbox tree for the delete modal
pub fn render_delete_tree(ui: &mut egui::Ui, path: &Path, selected: &mut HashSet<PathBuf>) {
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
            ui.horizontal(|ui| {
                let mut is_checked = selected.contains(&d);
                if ui.checkbox(&mut is_checked, "").clicked() {
                    if is_checked {
                        selected.insert(d.clone());
                    } else {
                        selected.remove(&d);
                    }
                }
                egui::CollapsingHeader::new(format!("📁 {}", name))
                    .default_open(false)
                    .show(ui, |ui| {
                        render_delete_tree(ui, &d, selected);
                    });
            });
        }
        for f in files {
            let name = f
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            ui.horizontal(|ui| {
                let mut is_checked = selected.contains(&f);
                if ui
                    .checkbox(&mut is_checked, format!("📄 {}", name))
                    .clicked()
                {
                    if is_checked {
                        selected.insert(f.clone());
                    } else {
                        selected.remove(&f);
                    }
                }
            });
        }
    }
}

// Renders a radio-button tree for selecting a target directory
pub fn render_select_dir_tree(ui: &mut egui::Ui, path: &Path, selected: &mut PathBuf) {
    if let Ok(entries) = fs::read_dir(path) {
        let mut dirs = Vec::new();
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir()
                && !p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .starts_with('.')
            {
                dirs.push(p);
            }
        }
        dirs.sort();

        for d in dirs {
            let name = d
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            ui.horizontal(|ui| {
                if ui.radio(*selected == d, "").clicked() {
                    *selected = d.clone();
                }
                egui::CollapsingHeader::new(format!("📁 {}", name))
                    .default_open(false)
                    .show(ui, |ui| {
                        render_select_dir_tree(ui, &d, selected);
                    });
            });
        }
    }
}

impl CCslipsApp {
    // Renders the Help window overlay floating on top of the UI
    pub fn render_help_window(&mut self, ctx: &egui::Context) {
        let mut is_open = self.show_help_window;

        egui::Window::new("❓ Keyboard Shortcuts & Help")
            .open(&mut is_open)
            .collapsible(false)
            .resizable(true)
            .default_width(450.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Global Shortcuts");
                    ui.separator();
                    egui::Grid::new("global_shortcuts_grid").num_columns(2).spacing([40.0, 8.0]).striped(true).show(ui, |ui| {
                        for shortcut in &self.shortcuts.global {
                            ui.label(egui::RichText::new(shortcut.display_string()).strong());
                            ui.label(shortcut.help);
                            ui.end_row();
                        }
                    });
                    ui.add_space(20.0);

                    ui.heading("Editor Shortcuts");
                    ui.separator();
                    egui::Grid::new("editor_shortcuts_grid").num_columns(2).spacing([40.0, 8.0]).striped(true).show(ui, |ui| {
                        for shortcut in &self.shortcuts.editor {
                            ui.label(egui::RichText::new(shortcut.display_string()).strong());
                            ui.label(shortcut.help);
                            ui.end_row();
                        }
                    });

                    ui.add_space(20.0);

                    // Vim Vertical Mode Tutorial
                    ui.group(|ui| {
                        ui.heading("💡 How to use Vertical Edit Mode");
                        ui.separator();
                        ui.label("Vertical Edit mode allows you to edit multiple lines of code simultaneously (Vim-style block selection).");
                        ui.add_space(8.0);
                        ui.horizontal(|ui| { ui.label(egui::RichText::new("1.").strong()); ui.label("Move your cursor to the starting position."); });
                        ui.horizontal(|ui| { ui.label(egui::RichText::new("2.").strong()); ui.label("Press"); ui.label(egui::RichText::new("Alt + V").strong().code()); ui.label("to drop the anchor cursor."); });
                        ui.horizontal(|ui| { ui.label(egui::RichText::new("3.").strong()); ui.label("Use the"); ui.label(egui::RichText::new("Up").strong().code()); ui.label("and"); ui.label(egui::RichText::new("Down").strong().code()); ui.label("arrow keys to expand the block."); });
                        ui.horizontal(|ui| { ui.label(egui::RichText::new("4.").strong()); ui.label("Begin typing to push text to all lines simultaneously."); });
                        ui.horizontal(|ui| { ui.label(egui::RichText::new("5.").strong()); ui.label("Press"); ui.label(egui::RichText::new("Escape").strong().code()); ui.label("or click anywhere to return to normal editing."); });
                    });
                });
            });

        self.show_help_window = is_open;
    }

    pub fn render_file_operation_modal(&mut self, ctx: &egui::Context) {
        if self.active_file_op == FileOperation::None {
            return;
        }

        let mut is_open = true;
        let mut trigger_execute = false;
        let mut trigger_cancel = false;

        let mut current_op = std::mem::replace(&mut self.active_file_op, FileOperation::None);

        let title = match &current_op {
            FileOperation::CreateFile(_) => "📄 Create New File",
            FileOperation::CreateDir(_) => "📁 Create New Directory",
            FileOperation::Delete(_) => "🗑 Select Items to Delete",
            FileOperation::None => "",
        };

        egui::Window::new(title)
            .collapsible(false)
            .resizable(true)
            .default_size([450.0, 400.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.screen_rect().center())
            .open(&mut is_open)
            .show(ctx, |ui| match &mut current_op {
                FileOperation::CreateFile(path) | FileOperation::CreateDir(path) => {
                    egui::TopBottomPanel::bottom("create_bottom_panel")
                        .resizable(false)
                        .frame(egui::Frame::none())
                        .show_inside(ui, |ui| {
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.label("Name:");
                                let response = ui.add(
                                    egui::TextEdit::singleline(&mut self.file_op_input)
                                        .desired_width(f32::INFINITY),
                                );
                                response.request_focus();
                                if response.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    trigger_execute = true;
                                }
                            });
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui.button("Create").clicked() {
                                    trigger_execute = true;
                                }
                                if ui.button("Cancel").clicked() {
                                    trigger_cancel = true;
                                }
                            });
                        });

                    egui::CentralPanel::default()
                        .frame(egui::Frame::none())
                        .show_inside(ui, |ui| {
                            ui.label(egui::RichText::new("Target Directory:").strong());
                            let working_dir = PathBuf::from(&self.config.build.working_directory);
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        if ui
                                            .radio(*path == working_dir, "📁 (Workspace Root)")
                                            .clicked()
                                        {
                                            *path = working_dir.clone();
                                        }
                                    });
                                    ui.indent("dir_tree", |ui| {
                                        render_select_dir_tree(ui, &working_dir, path);
                                    });
                                });
                        });
                }
                FileOperation::Delete(selected) => {
                    egui::TopBottomPanel::bottom("delete_bottom_panel")
                        .resizable(false)
                        .frame(egui::Frame::none())
                        .show_inside(ui, |ui| {
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                let btn_text = format!("🗑 Delete ({})", selected.len());
                                let del_btn = ui.add_enabled(
                                    !selected.is_empty(),
                                    egui::Button::new(
                                        egui::RichText::new(btn_text).color(egui::Color32::RED),
                                    ),
                                );
                                if del_btn.clicked() {
                                    trigger_execute = true;
                                }
                                if ui.button("Cancel").clicked() {
                                    trigger_cancel = true;
                                }
                            });
                        });

                    egui::CentralPanel::default()
                        .frame(egui::Frame::none())
                        .show_inside(ui, |ui| {
                            ui.label(
                                "Select the files and directories you want to permanently delete:",
                            );
                            ui.add_space(8.0);
                            let working_dir = PathBuf::from(&self.config.build.working_directory);
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    render_delete_tree(ui, &working_dir, selected);
                                });
                        });
                }
                FileOperation::None => {}
            });

        self.active_file_op = current_op;

        if trigger_execute {
            self.execute_file_operation();
        } else if !is_open || trigger_cancel {
            self.active_file_op = FileOperation::None;
        }
    }
}
