mod ai;
mod autocomplete;
mod config;
mod search_replace;
mod syntax_highlights;

// New modules
mod actions;
mod ui;

use ai::*;
use autocomplete::*;
use config::{CCslipsConfig, parse_hex};
use search_replace::*;

use eframe::egui;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};

#[derive(PartialEq)]
pub enum RightTab {
    Index,
    Terminal,
    Monitor,
}

#[derive(Clone, Copy, Debug)]
pub struct VerticalCursor {
    pub start_line: usize,
    pub end_line: usize,
    pub col: usize,
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
    // (prefix, formatted_display, insert_string, type, selected_index, start_idx, end_idx)
    pub active_menu: Option<(String, Vec<(String, String, String)>, usize, usize, usize)>,
    pub dismissed_prefix: Option<String>,

    pub search_state: SearchState,

    // NEW: Vertical Cursor State
    pub vertical_cursor: Option<VerticalCursor>,
    pub alt_drag_start: Option<(usize, usize)>,
}

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
            vertical_cursor: None,
            alt_drag_start: None,
        };
        app.append_log("[SYSTEM] Charcoal Slips Editor Initialized.");

        if let Some(last_file) = &app.config.editor.last_opened_file.clone() {
            let path = PathBuf::from(last_file);
            if path.exists() && path.is_file() {
                if let Ok(content) = fs::read_to_string(&path) {
                    app.editor_text = content;
                    app.current_file = Some(path.clone());
                    app.append_log(&format!("[SYSTEM] 📂 Restored session: {}", path.display()));
                }
            }
        }

        app
    }
}

impl eframe::App for CCslipsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.current_file.is_some() {
                self.save_current_file();
                self.save_config();
            }
        }

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
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::W)) {
            if self.current_file.is_some() {
                self.close_file();
                ctx.memory_mut(|mem| mem.surrender_focus(egui::Id::new("latex_editor")));
            } else {
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
                        self.perform_search(false, true);
                    }
                }
            }
        }

        if self.search_state.is_active && self.active_menu.is_none() {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                self.search_state.is_active = false;
                self.search_state.matches.clear();
                ctx.memory_mut(|mem| mem.request_focus(egui::Id::new("latex_editor")));
            }
        }

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

        // --- RENDER PIPELINE ---
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
