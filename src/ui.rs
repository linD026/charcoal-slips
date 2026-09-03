use crate::ai::trigger_ai_indexing;
use crate::config::parse_hex;
use crate::shortcuts::AppAction;
use crate::syntax_highlights::{highlight_latex, highlight_logs};
use crate::{CCslipsApp, RightTab, VerticalCursor};

use eframe::egui;
use std::fs;
use std::path::{Path, PathBuf};

pub fn render_dir_tree(
    ui: &mut egui::Ui,
    path: &Path,
    current_file: &Option<PathBuf>,
) -> Option<PathBuf> {
    let mut clicked = None;
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
            egui::CollapsingHeader::new(format!("📁 {}", name))
                .default_open(false)
                .show(ui, |ui| {
                    if let Some(res) = render_dir_tree(ui, &d, current_file) {
                        clicked = Some(res);
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
            if ui
                .selectable_label(is_selected, format!("📄 {}", name))
                .clicked()
            {
                clicked = Some(f);
            }
        }
    }
    clicked
}

impl CCslipsApp {
    pub fn render_left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(self.config.ui.left_panel_width)
            .show(ctx, |ui| {
                ui.heading("Workspace");
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
                        if let Some(clicked_path) = render_dir_tree(
                            ui,
                            Path::new(&self.config.build.working_directory),
                            &self.current_file,
                        ) {
                            self.open_file(clicked_path, false);
                        }
                    });
                });
            });
    }

    pub fn render_right_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("right_panel")
            .resizable(true)
            .default_width(self.config.ui.right_panel_width)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let is_index = self.active_right_tab == RightTab::Index;
                    let is_term = self.active_right_tab == RightTab::Terminal;
                    let is_monitor = self.active_right_tab == RightTab::Monitor;

                    let index_text = if is_index {
                        egui::RichText::new("🧠 AI Index").strong()
                    } else {
                        egui::RichText::new("🧠 AI Index").weak()
                    };
                    if ui.add(egui::Button::new(index_text).frame(false)).clicked() {
                        self.active_right_tab = RightTab::Index;
                    }

                    let term_text = if is_term {
                        egui::RichText::new("💻 Terminal").strong()
                    } else {
                        egui::RichText::new("💻 Terminal").weak()
                    };
                    if ui.add(egui::Button::new(term_text).frame(false)).clicked() {
                        self.active_right_tab = RightTab::Terminal;
                    }

                    let monitor_text = if is_monitor {
                        egui::RichText::new("📊 Monitor").strong()
                    } else {
                        egui::RichText::new("📊 Monitor").weak()
                    };
                    if ui
                        .add(egui::Button::new(monitor_text).frame(false))
                        .clicked()
                    {
                        self.active_right_tab = RightTab::Monitor;
                    }
                });
                ui.separator();

                match self.active_right_tab {
                    RightTab::Index => {
                        let mut trigger_jump = None;
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for entry in &self.index_entries {
                                ui.group(|ui| {
                                    ui.label(
                                        egui::RichText::new(&entry.ai_summary).strong().size(15.0),
                                    );
                                    let preview = if entry.selected_text.len() > 60 {
                                        format!("\"{}...\"", &entry.selected_text[..60])
                                    } else {
                                        format!("\"{}\"", entry.selected_text)
                                    };
                                    ui.label(egui::RichText::new(preview).weak().italics());
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(
                                                    entry.timestamp.format("%H:%M:%S").to_string(),
                                                )
                                                .weak(),
                                            );
                                        },
                                    );
                                    if ui.button("⮐ Jump to Selection").clicked() {
                                        trigger_jump = Some((
                                            entry.file_path.clone(),
                                            entry.start_idx,
                                            entry.end_idx,
                                        ));
                                    }
                                });
                            }
                        });

                        if let Some((path, start, end)) = trigger_jump {
                            self.open_file(path, true);
                            self.jump_request = Some((start, end));
                        }
                    }
                    RightTab::Terminal => {
                        let terminal_theme = if self.config.ui.dark_mode {
                            self.config.ui.dark_theme.terminal.clone()
                        } else {
                            self.config.ui.light_theme.terminal.clone()
                        };

                        egui::ScrollArea::both()
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                let mut layouter =
                                    move |ui: &egui::Ui, string: &str, wrap_width: f32| {
                                        let mut job = highlight_logs(string, 12.0, &terminal_theme);
                                        job.wrap.max_width = wrap_width;
                                        ui.fonts(|f| f.layout_job(job))
                                    };
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.terminal_log)
                                        .desired_width(f32::INFINITY)
                                        .frame(false)
                                        .layouter(&mut layouter),
                                );
                            });
                    }
                    RightTab::Monitor => {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.add_space(4.0);
                            ui.group(|ui| {
                                ui.label(egui::RichText::new("📝 Editor Buffer").strong());
                                ui.separator();
                                let bytes = self.editor_text.len();
                                let chars = self.editor_text.chars().count();
                                let lines = self.editor_text.lines().count();

                                egui::Grid::new("editor_metrics_grid")
                                    .num_columns(2)
                                    .striped(true)
                                    .show(ui, |ui| {
                                        ui.label("Lines:");
                                        ui.label(lines.to_string());
                                        ui.end_row();
                                        ui.label("Characters:");
                                        ui.label(chars.to_string());
                                        ui.end_row();
                                        ui.label("Est. Memory:");
                                        ui.label(format!("{:.2} KB", bytes as f64 / 1024.0));
                                        ui.end_row();
                                    });
                            });
                            ui.add_space(8.0);
                            ui.group(|ui| {
                                ui.label(egui::RichText::new("🗄️ Internal Caches").strong());
                                ui.separator();
                                let (bib_files, bib_keys) = self.bib_cache.get_metrics();
                                let (lbl_files, lbl_keys) = self.label_cache.get_metrics();

                                egui::Grid::new("cache_metrics_grid")
                                    .num_columns(2)
                                    .striped(true)
                                    .show(ui, |ui| {
                                        ui.label("BibTeX Files Tracked:");
                                        ui.label(bib_files.to_string());
                                        ui.end_row();
                                        ui.label("BibTeX Keys Loaded:");
                                        ui.label(bib_keys.to_string());
                                        ui.end_row();
                                        ui.label("LaTeX Files Tracked:");
                                        ui.label(lbl_files.to_string());
                                        ui.end_row();
                                        ui.label("LaTeX Labels Loaded:");
                                        ui.label(lbl_keys.to_string());
                                        ui.end_row();
                                    });
                            });
                            ui.add_space(8.0);
                            ui.group(|ui| {
                                ui.label(egui::RichText::new("🔍 Subsystems").strong());
                                ui.separator();
                                egui::Grid::new("search_ai_metrics")
                                    .num_columns(2)
                                    .striped(true)
                                    .show(ui, |ui| {
                                        ui.label("Active Search Matches:");
                                        ui.label(self.search_state.matches.len().to_string());
                                        ui.end_row();
                                        ui.label("AI Index Entries:");
                                        ui.label(self.index_entries.len().to_string());
                                        ui.end_row();
                                        ui.label("Terminal Log Size:");
                                        ui.label(format!(
                                            "{:.2} KB",
                                            self.terminal_log.len() as f64 / 1024.0
                                        ));
                                        ui.end_row();
                                    });
                            });
                        });
                    }
                }
            });
    }

    pub fn render_toolbar(&mut self, ui: &mut egui::Ui, current_selection: Option<(usize, usize)>) {
        let (ai_bg_hex, ai_fg_hex) = if self.config.ui.dark_mode {
            (
                self.config.ui.dark_theme.ui.ai_button_bg.clone(),
                self.config.ui.dark_theme.ui.ai_button_text.clone(),
            )
        } else {
            (
                self.config.ui.light_theme.ui.ai_button_bg.clone(),
                self.config.ui.light_theme.ui.ai_button_text.clone(),
            )
        };

        ui.horizontal(|ui| {
            if ui.button("💾 Save (Ctrl+S)").clicked() {
                self.save_current_file();
            }
            if ui.button("🚀 Build (Ctrl+B)").clicked() {
                self.execute_build();
            }
            ui.separator();

            let theme_icon = if self.config.ui.dark_mode {
                "🌙 Dark"
            } else {
                "☀️  Light"
            };
            if ui.button(theme_icon).clicked() {
                self.config.ui.dark_mode = !self.config.ui.dark_mode;
                self.save_config();
            }
            ui.separator();

            if ui.button("A-").clicked() {
                self.config.editor.font_size -= 1.0;
            }
            if ui.button("A+").clicked() {
                self.config.editor.font_size += 1.0;
            }
            ui.separator();

            // Check AI using the centralized Shortcut Registry
            let ai_triggered = self.shortcuts.check_action(ui.ctx(), AppAction::SendToAi);

            if let Some((start, end)) = current_selection {
                if let Some(path) = &self.current_file {
                    let ai_bg = parse_hex(&ai_bg_hex);
                    let ai_fg = parse_hex(&ai_fg_hex);
                    let ai_btn = egui::Button::new(
                        egui::RichText::new("🧠 Send to AI (Ctrl+I)").color(ai_fg),
                    )
                    .fill(ai_bg);

                    if ui.add(ai_btn).clicked() || ai_triggered {
                        let selected_str: String = self
                            .editor_text
                            .chars()
                            .skip(start)
                            .take(end - start)
                            .collect();
                        trigger_ai_indexing(
                            self.config.ai.clone(),
                            path.clone(),
                            selected_str.clone(),
                            start,
                            end,
                            self.tx_ai.clone(),
                        );
                        self.active_right_tab = RightTab::Index;
                        self.is_generating = true;

                        let clean_str = selected_str.replace('\n', " ");
                        let preview = if clean_str.len() > 50 {
                            format!("{}...", &clean_str[..50])
                        } else {
                            clean_str
                        };
                        self.append_log(&format!(
                            "[AI] 📡 Sending request to backend: \"{}\"",
                            preview
                        ));
                    }
                } else {
                    ui.add_enabled(false, egui::Button::new("Save file first to use AI"));
                }
            } else {
                ui.add_enabled(false, egui::Button::new("Highlight text to index..."));
            }
        });
        ui.separator();
    }

    pub fn render_editor_with_gutters(
        &mut self,
        ui: &mut egui::Ui,
        editor_id: egui::Id,
    ) -> egui::text_edit::TextEditOutput {
        let font = egui::FontId::monospace(self.config.editor.font_size);
        let font_size = self.config.editor.font_size;

        let (syntax_theme, gutter_color, editor_selection_bg) = if self.config.ui.dark_mode {
            (
                self.config.ui.dark_theme.syntax.clone(),
                parse_hex(&self.config.ui.dark_theme.ui.gutter_text),
                parse_hex(&self.config.ui.dark_theme.ui.editor_selection_bg),
            )
        } else {
            (
                self.config.ui.light_theme.syntax.clone(),
                parse_hex(&self.config.ui.light_theme.ui.gutter_text),
                parse_hex(&self.config.ui.light_theme.ui.editor_selection_bg),
            )
        };

        ui.visuals_mut().selection.bg_fill = editor_selection_bg;
        ui.visuals_mut().selection.stroke.color = egui::Color32::TRANSPARENT;

        let mut layouter = move |ui: &egui::Ui, string: &str, wrap_width: f32| {
            let mut layout_job = highlight_latex(string, font_size, &syntax_theme);
            layout_job.wrap.max_width = wrap_width;
            ui.fonts(|f| f.layout_job(layout_job))
        };

        let mut window_just_focused = false;
        ui.input(|i| {
            for e in &i.events {
                if let egui::Event::WindowFocused(true) = e {
                    window_just_focused = true;
                }
            }
        });

        ui.input_mut(|i| {
            i.events.retain(|e| {
                if let egui::Event::Text(text) = e {
                    if text == "\t" && (i.modifiers.alt || window_just_focused) {
                        return false;
                    }
                }
                if let egui::Event::Key {
                    key: egui::Key::Tab,
                    ..
                } = e
                {
                    if i.modifiers.alt || window_just_focused {
                        return false;
                    }
                }
                true
            });
        });

        let total_lines = self.editor_text.split('\n').count();
        let gutter_width = ui
            .fonts(|f| {
                f.layout_no_wrap(
                    total_lines.to_string(),
                    font.clone(),
                    ui.visuals().text_color(),
                )
            })
            .rect
            .width()
            + 15.0;

        let output = ui
            .horizontal_top(|ui| {
                ui.add_space(gutter_width);
                egui::TextEdit::multiline(&mut self.editor_text)
                    .id(editor_id)
                    .font(font.clone())
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .frame(false)
                    .margin(egui::vec2(0.0, 0.0))
                    .layouter(&mut layouter)
                    .show(ui)
            })
            .inner;

        let padding_height = font_size * 1.5 * 40.0;
        ui.add_space(padding_height);

        let painter = ui.painter();
        let galley = &output.galley;

        let mut current_logical_line = 1;
        let mut is_start_of_line = true;

        for row in &galley.rows {
            if is_start_of_line {
                let pos = egui::pos2(
                    output.galley_pos.x - 10.0,
                    output.galley_pos.y + row.rect.min.y,
                );
                painter.text(
                    pos,
                    egui::Align2::RIGHT_TOP,
                    current_logical_line.to_string(),
                    font.clone(),
                    gutter_color,
                );
                current_logical_line += 1;
            }
            is_start_of_line = row.ends_with_newline;
        }

        if self.editor_text.ends_with('\n') {
            let pos = egui::pos2(
                output.galley_pos.x - 10.0,
                output.galley_pos.y + galley.mesh_bounds.max.y,
            );
            painter.text(
                pos,
                egui::Align2::RIGHT_TOP,
                current_logical_line.to_string(),
                font,
                gutter_color,
            );
        }

        // ==========================================
        // VERTICAL EDIT (MULTI-CURSOR) RENDERING
        // ==========================================

        if let Some(vc) = &self.vertical_cursor {
            let cursor_color = if self.config.ui.dark_mode {
                parse_hex(&self.config.ui.dark_theme.ui.cursor)
            } else {
                parse_hex(&self.config.ui.light_theme.ui.cursor)
            };

            // Clear native egui selection right away so it doesn't flicker/interfere
            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), editor_id) {
                state.cursor.set_char_range(None);
                egui::TextEdit::store_state(ui.ctx(), editor_id, state);
            }

            let time = ui.input(|i| i.time);
            let blink_on = (time * 2.0).fract() < 0.5;

            let start_l = vc.anchor_line.min(vc.active_line);
            let end_l = vc.anchor_line.max(vc.active_line);

            if blink_on {
                for line_idx in start_l..=end_l {
                    let line_str = self.editor_text.split('\n').nth(line_idx).unwrap_or("");
                    let actual_col = line_str.chars().count().min(vc.col);

                    let mut abs_index = 0;
                    for (i, line) in self.editor_text.split('\n').enumerate() {
                        if i < line_idx {
                            abs_index += line.chars().count() + 1; // +1 for the newline
                        } else if i == line_idx {
                            abs_index += actual_col;
                            break;
                        }
                    }

                    let ccursor = egui::text::CCursor::new(abs_index);
                    let cursor_pos = galley.pos_from_ccursor(ccursor);
                    let rect = cursor_pos.translate(output.galley_pos.to_vec2());

                    painter
                        .line_segment([rect.min, rect.max], egui::Stroke::new(2.0, cursor_color));
                }
            }
        }

        output
    }

    pub fn render_highlight_matches(
        &mut self,
        ui: &mut egui::Ui,
        output: &egui::text_edit::TextEditOutput,
    ) {
        if output.response.changed() && self.search_state.is_active {
            self.perform_search(false, false);
        }

        if self.search_state.is_active && !self.search_state.find_query.is_empty() {
            let current_file_path = self.current_file.clone().unwrap_or_default();
            let painter = ui.painter();

            let theme = if self.config.ui.dark_mode {
                &self.config.ui.dark_theme
            } else {
                &self.config.ui.light_theme
            };
            let c_match = parse_hex(&theme.search.match_bg);
            let c_current = parse_hex(&theme.search.current_match_bg);

            for (i, match_item) in self.search_state.matches.iter().enumerate() {
                if match_item.file == current_file_path {
                    let is_current = i == self.search_state.current_match_idx;
                    let color = if is_current { c_current } else { c_match };

                    let start_pos = output
                        .galley
                        .pos_from_ccursor(egui::text::CCursor::new(match_item.start));
                    let end_pos = output
                        .galley
                        .pos_from_ccursor(egui::text::CCursor::new(match_item.end));

                    let rect = egui::Rect::from_min_max(
                        output.galley_pos + start_pos.min.to_vec2(),
                        output.galley_pos + end_pos.max.to_vec2(),
                    );
                    painter.rect_filled(rect, 2.0, color);
                }
            }
        }
    }

    pub fn render_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let editor_id = egui::Id::new("latex_editor");

            let mut current_selection = None;
            if let Some(state) = egui::TextEdit::load_state(ui.ctx(), editor_id) {
                if let Some(range) = state.cursor.char_range() {
                    let start = range.primary.index.min(range.secondary.index);
                    let end = range.primary.index.max(range.secondary.index);
                    if start != end {
                        current_selection = Some((start, end));
                    }
                }
            }

            self.render_toolbar(ui, current_selection);

            // Execute Vertical Edition Input processing BEFORE UI renders `TextEdit`
            self.handle_vertical_edit_input(ctx, editor_id);

            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let (autocomplete_handled, local_jump_request) =
                        self.intercept_autocomplete_navigation(ui, editor_id);
                    if local_jump_request.is_some() {
                        self.jump_request = local_jump_request;
                    }

                    let output = self.render_editor_with_gutters(ui, editor_id);

                    self.render_highlight_matches(ui, &output);
                    self.update_autocomplete_state(&output, autocomplete_handled);
                    self.draw_autocomplete_popup(ui, &output);

                    if let Some((start, end)) = self.jump_request.take() {
                        if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), editor_id) {
                            let ccursor_start = egui::text::CCursor::new(start);
                            let ccursor_end = egui::text::CCursor::new(end);
                            state
                                .cursor
                                .set_char_range(Some(egui::text::CCursorRange::two(
                                    ccursor_start,
                                    ccursor_end,
                                )));
                            egui::TextEdit::store_state(ui.ctx(), editor_id, state);
                            output.response.request_focus();

                            let pos = output.galley.pos_from_ccursor(ccursor_start);
                            let rect = pos.translate(output.galley_pos.to_vec2());
                            ui.scroll_to_rect(rect, Some(egui::Align::Center));
                        }
                    }
                });
        });
    }
}
