mod ai;
mod autocomplete;
mod config;
mod search_replace;
mod syntax_highlights;

use ai::*;
use autocomplete::*;
use search_replace::*;

use config::{CCslipsConfig, parse_hex};
use syntax_highlights::*;

use eframe::egui;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{Receiver, Sender, channel};

#[derive(PartialEq)]
pub enum RightTab {
    Index,
    Terminal,
    Monitor,
}

pub struct CCslipsApp {
    pub config: CCslipsConfig,
    pub current_file: Option<PathBuf>,
    pub editor_text: String,
    pub terminal_log: String,
    pub active_right_tab: RightTab,
    pub index_entries: Vec<IndexEntry>,
    pub tx_ai: Sender<IndexEntry>,
    pub rx_ai: Receiver<IndexEntry>,
    pub is_generating: bool,
    pub jump_request: Option<(usize, usize)>,

    pub bib_cache: BibCache,
    pub label_cache: LabelCache,
    // (prefix, formatted_display, insert_string, selected_index, start_idx, end_idx)
    pub active_menu: Option<(String, Vec<(String, String)>, usize, usize, usize)>,
    pub dismissed_prefix: Option<String>,

    pub search_state: SearchState,
}

fn render_dir_tree(
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

// ==========================================
// APPLICATION LOGIC & RENDERING
// ==========================================

impl CCslipsApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config_path = "config_charcoal_slips.json";
        let config = if let Ok(data) = fs::read_to_string(config_path) {
            serde_json::from_str(&data).unwrap_or_else(|_| CCslipsConfig::default())
        } else {
            let default_cfg = CCslipsConfig::default();
            let _ = fs::write(
                config_path,
                serde_json::to_string_pretty(&default_cfg).unwrap(),
            );
            default_cfg
        };

        let (tx_ai, rx_ai) = channel();

        let mut app = Self {
            config,
            current_file: None,
            editor_text: String::new(),
            terminal_log: String::new(),
            active_right_tab: RightTab::Index,
            index_entries: Vec::new(),
            tx_ai,
            rx_ai,
            is_generating: false,
            jump_request: None,
            active_menu: None,
            dismissed_prefix: None,
            bib_cache: BibCache::new(),
            label_cache: LabelCache::new(),
            search_state: SearchState::default(),
        };
        app.append_log("[SYSTEM] Charcoal Slips Editor Initialized.");
        app
    }

    fn append_log(&mut self, message: &str) {
        self.terminal_log.push_str(message);
        self.terminal_log.push('\n');
    }

    fn save_current_file(&mut self) {
        if let Some(path) = &self.current_file {
            match fs::write(path, &self.editor_text) {
                Ok(_) => self.append_log(&format!("[FILE] 💾 Saved: {}", path.display())),
                Err(e) => self.append_log(&format!("[ERROR] ❌ Save Failed: {}", e)),
            }
        }
    }

    fn execute_build(&mut self) {
        if self.config.build.auto_save_before_build {
            self.save_current_file();
        }
        let cmd = self.config.build.command.clone();
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        self.append_log(&format!("[BUILD] 🔄 Executing: {}", cmd));
        self.active_right_tab = RightTab::Terminal;

        match Command::new(parts[0])
            .args(&parts[1..])
            .current_dir(&self.config.build.working_directory)
            .output()
        {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                if out.status.success() {
                    self.append_log("[SUCCESS] ✅ Build Completed.");
                } else {
                    self.append_log(&format!("[ERROR] ❌ Build Failed: {}", out.status));
                }
                if !stdout.is_empty() {
                    self.append_log(&format!("[STDOUT]\n{}", stdout));
                }
                if !stderr.is_empty() {
                    self.append_log(&format!("[STDERR]\n{}", stderr));
                }
            }
            Err(e) => self.append_log(&format!("[ERROR] ❌ Pipeline failed: {}", e)),
        }
    }

    // --- Modularized Panel Renderers ---

    fn render_left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(self.config.ui.left_panel_width)
            .show(ctx, |ui| {
                ui.heading("Workspace");
                ui.separator();

                // 1. Lock the Search/Replace panel to the bottom if active
                if self.search_state.is_active {
                    egui::TopBottomPanel::bottom("search_replace_panel")
                        .resizable(false)
                        .show_inside(ui, |ui| {
                            ui.add_space(4.0);
                            self.render_search_replace_panel(ui);
                            ui.add_space(4.0);
                        });
                }

                // 2. The Directory Tree automatically takes up all remaining space in the middle
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        if let Some(clicked_path) = render_dir_tree(
                            ui,
                            Path::new(&self.config.build.working_directory),
                            &self.current_file,
                        ) {
                            if self.current_file.as_ref() != Some(&clicked_path) {
                                if self.current_file.is_some() {
                                    self.save_current_file();
                                }

                                if let Ok(content) = fs::read_to_string(&clicked_path) {
                                    self.editor_text = content;
                                    self.current_file = Some(clicked_path.clone());
                                    self.append_log(&format!(
                                        "[FILE] 📂 Opened: {}",
                                        clicked_path.display()
                                    ));
                                }
                            }
                        }
                    });
                });
            });
    }

    fn render_right_panel(&mut self, ctx: &egui::Context) {
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
                            if self.current_file.as_ref() != Some(&path) {
                                if self.current_file.is_some() {
                                    self.save_current_file();
                                }

                                if let Ok(content) = fs::read_to_string(&path) {
                                    self.editor_text = content;
                                    self.current_file = Some(path.clone());
                                    self.append_log(&format!(
                                        "[FILE] 📂 Auto-opened for jump: {}",
                                        path.display()
                                    ));
                                }
                            }
                            self.jump_request = Some((start, end));
                        }
                    }
                    RightTab::Terminal => {
                        // Extract terminal_theme here safely
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

                            // Editor Metrics
                            ui.group(|ui| {
                                ui.label(egui::RichText::new("📝 Editor Buffer").strong());
                                ui.separator();

                                // O(1) calculations for speed
                                let bytes = self.editor_text.len();
                                // Fast iterators for exact counts
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

                            // Cache Metrics
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

                            // System & AI Metrics
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

    fn render_toolbar(&mut self, ui: &mut egui::Ui, current_selection: Option<(usize, usize)>) {
        // Extract and clone the specific strings we need *before* the closure.
        // This drops the immutable borrow on `self`, keeping the borrow checker perfectly happy.
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
                fs::write(
                    "config_charcoal_slips.json",
                    serde_json::to_string_pretty(&self.config).unwrap(),
                )
                .ok();
            }
            ui.separator();

            if ui.button("A-").clicked() {
                self.config.editor.font_size -= 1.0;
            }
            if ui.button("A+").clicked() {
                self.config.editor.font_size += 1.0;
            }
            ui.separator();

            // AI Index Trigger
            let ai_triggered = ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::I));

            if let Some((start, end)) = current_selection {
                if let Some(path) = &self.current_file {
                    // Pass our cleanly extracted strings into the parser
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
                            selected_str,
                            start,
                            end,
                            self.tx_ai.clone(),
                        );
                        self.active_right_tab = RightTab::Index;
                        self.is_generating = true;
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

    fn render_editor_with_gutters(
        &mut self,
        ui: &mut egui::Ui,
        editor_id: egui::Id,
    ) -> egui::text_edit::TextEditOutput {
        let font = egui::FontId::monospace(self.config.editor.font_size);
        let font_size = self.config.editor.font_size;

        // Extract these values upfront to drop the borrow on `self.config`
        // This prevents E0502 when we mutably borrow `self.editor_text` below.
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

        // Scope mutation: Override visuals exclusively for the text editor.
        // This brings back the translucent highlighting without breaking the file tree!
        ui.visuals_mut().selection.bg_fill = editor_selection_bg;
        ui.visuals_mut().selection.stroke.color = egui::Color32::TRANSPARENT;

        let mut layouter = move |ui: &egui::Ui, string: &str, wrap_width: f32| {
            let mut layout_job = highlight_latex(string, font_size, &syntax_theme);
            layout_job.wrap.max_width = wrap_width;
            ui.fonts(|f| f.layout_job(layout_job))
        };

        // Prevent Alt+Tab Ghost Inputs
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

        // Calculate dynamic gutter width based on true line count
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
                // Paint the number relative to the top-left of the text's galley
                painter.text(
                    pos,
                    egui::Align2::RIGHT_TOP,
                    current_logical_line.to_string(),
                    font.clone(),
                    gutter_color, // Linked to JSON Theme
                );
                current_logical_line += 1;
            }
            // If this visual row ends with a newline, the NEXT visual row will be the start of a
            // new logical line.
            is_start_of_line = row.ends_with_newline;
        }

        // Draw final trailing newline number if the file ends with an empty line
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
                gutter_color, // Linked to JSON Theme
            );
        }

        output
    }

    fn render_highlight_matches(
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

            // Grab the active search theme
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

    fn render_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let editor_id = egui::Id::new("latex_editor");

            // Fetch Current Selection for Toolbar
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

            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // 1. Intercept Navigation FIRST
                    // We catch Arrow keys and Enter/Tab before
                    // the text editor consumes them
                    let (autocomplete_handled, local_jump_request) =
                        self.intercept_autocomplete_navigation(ui, editor_id);

                    // Register any jump requested by the autocomplete insertion
                    if local_jump_request.is_some() {
                        self.jump_request = local_jump_request;
                    }

                    // 2. Render Main Editor
                    // This processes text layout and consumes
                    // remaining keyboard inputs
                    let output = self.render_editor_with_gutters(ui, editor_id);

                    self.render_highlight_matches(ui, &output);

                    // 3. Update Autocomplete State
                    // Check if the user's typing triggered
                    // a new macro/citation/file lookup
                    self.update_autocomplete_state(&output, autocomplete_handled);

                    // 4. Draw the Floating Menu
                    // Overlay the popup at the correct screen
                    // position based on the editor's galley
                    self.draw_autocomplete_popup(ui, &output);

                    // 5. Execute Jumps
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

                            // Grab the physical rectangle of the text cursor and tell the camera to pan to it, dead center.
                            let pos = output.galley.pos_from_ccursor(ccursor_start);
                            let rect = pos.translate(output.galley_pos.to_vec2());
                            ui.scroll_to_rect(rect, Some(egui::Align::Center));
                        }
                    }
                });
        });
    }
}

// ==========================================
// MAIN LOOP
// ==========================================

impl eframe::App for CCslipsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Safely parse colors directly, releasing the lock on `self.config`
        // Extract both UI and Editor specific selection colors
        let (bg_color, ui_selection_bg, ui_selection_text, cursor_color) =
            if self.config.ui.dark_mode {
                let t = &self.config.ui.dark_theme.ui;
                (
                    parse_hex(&t.bg_color),
                    parse_hex(&t.ui_selection_bg),
                    parse_hex(&t.ui_selection_text),
                    parse_hex(&t.cursor),
                )
            } else {
                let t = &self.config.ui.light_theme.ui;
                (
                    parse_hex(&t.bg_color),
                    parse_hex(&t.ui_selection_bg),
                    parse_hex(&t.ui_selection_text),
                    parse_hex(&t.cursor),
                )
            };

        let mut visuals = if self.config.ui.dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };

        visuals.panel_fill = bg_color;
        visuals.window_fill = bg_color;
        visuals.extreme_bg_color = bg_color;

        // Apply UI-specific selections globally to fix the vague text bug
        visuals.selection.bg_fill = ui_selection_bg;
        visuals.selection.stroke.color = ui_selection_text;
        visuals.text_cursor.color = cursor_color;

        ctx.set_visuals(visuals);

        // Global Keyboard Shortcuts
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
            self.save_current_file();
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::B)) {
            self.execute_build();
        }
        // Cascading Close (Ctrl+W)
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::W)) {
            if self.current_file.is_some() {
                // 1. A file is open: Save it, clear the buffer, and deselect the file
                self.save_current_file();
                self.editor_text.clear();
                self.current_file = None;
                self.append_log("[SYSTEM] 📁 Closed current file.");

                // Unfocus the text editor so the user doesn't accidentally type into the void
                ctx.memory_mut(|mem| mem.surrender_focus(egui::Id::new("latex_editor")));
            } else {
                // 2. No file is open: Send the termination signal to the OS window manager
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if ctx.input(|i| {
            i.modifiers.command
                && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals))
        }) {
            self.config.editor.font_size = (self.config.editor.font_size + 1.0).clamp(8.0, 48.0);
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Minus)) {
            self.config.editor.font_size = (self.config.editor.font_size - 1.0).clamp(8.0, 48.0);
        }
        // Find/Replace Shortcuts
        // The Ctrl+R shortcut to focus Replace is handled inside `render_search_replace_panel`
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F)) {
            self.search_state.is_active = true;
            self.search_state.focus_find = true;

            let editor_id = egui::Id::new("latex_editor");
            if let Some(state) = egui::TextEdit::load_state(ctx, editor_id) {
                if let Some(range) = state.cursor.char_range() {
                    let start = range.primary.index.min(range.secondary.index);
                    let end = range.primary.index.max(range.secondary.index);
                    if start != end {
                        self.search_state.find_query = self
                            .editor_text
                            .chars()
                            .skip(start)
                            .take(end - start)
                            .collect();
                        self.perform_search(false, true); // No proximity required, jump camera
                    }
                }
            }
        }

        // Safety Lock: We check `self.active_menu.is_none()` so we don't accidentally
        // close the Search panel when you are just trying to dismiss the Autocomplete popup!
        if self.search_state.is_active && self.active_menu.is_none() {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                self.search_state.is_active = false;
                self.search_state.matches.clear();

                // Instantly snap the keyboard focus back to the main text editor
                ctx.memory_mut(|mem| mem.request_focus(egui::Id::new("latex_editor")));
            }
        }

        // Process AI Callbacks
        if let Ok(entry) = self.rx_ai.try_recv() {
            if entry.ai_summary.starts_with("Error:") {
                self.append_log(&format!("[AI] ❌ Failed: {}", entry.ai_summary));
                self.active_right_tab = RightTab::Terminal;
            } else {
                self.append_log(&format!("[AI] ✅ Generated index '{}'", entry.ai_summary));
                self.index_entries.push(entry);
            }
            self.is_generating = false;
        }

        // Render UI
        self.render_left_panel(ctx);
        self.render_right_panel(ctx);
        self.render_central_panel(ctx);
    }
}

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Charcoal Slips",
        eframe::NativeOptions::default(),
        Box::new(|cc| Box::new(CCslipsApp::new(cc))),
    )
}
