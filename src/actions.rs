use crate::shortcuts::AppAction;
use crate::{CCslipsApp, RightTab, VerticalCursor};
use eframe::egui;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

impl CCslipsApp {
    pub fn append_log(&mut self, message: &str) {
        self.terminal_log.push_str(message);
        self.terminal_log.push('\n');
    }

    pub fn save_current_file(&mut self) {
        if let Some(path) = &self.current_file {
            match fs::write(path, &self.editor_text) {
                Ok(_) => self.append_log(&format!("[FILE] 💾 Saved: {}", path.display())),
                Err(e) => self.append_log(&format!("[ERROR] ❌ Save Failed: {}", e)),
            }
        }
    }

    pub fn save_config(&self) {
        let _ = fs::write(
            "config_charcoal_slips.json",
            serde_json::to_string_pretty(&self.config).unwrap_or_default(),
        );
    }

    pub fn open_file(&mut self, path: PathBuf, _is_jump: bool) {
        if self.current_file.as_ref() != Some(&path) {
            if self.current_file.is_some() {
                self.save_current_file();
            }

            if let Ok(content) = fs::read_to_string(&path) {
                self.editor_text = content;
                self.current_file = Some(path.clone());
                self.config.editor.last_opened_file = Some(path.to_string_lossy().to_string());
                self.save_config();
            }
        }
    }

    pub fn close_file(&mut self) {
        if self.current_file.is_some() {
            self.save_current_file();
            self.editor_text.clear();
            self.current_file = None;
            self.config.editor.last_opened_file = None;
            self.save_config();
            self.append_log("[SYSTEM] 📁 Closed current file.");
        }
    }

    pub fn execute_build(&mut self) {
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

    // ==========================================
    // VERTICAL EDIT (MULTI-CURSOR) LOGIC
    // ==========================================

    fn char_index_to_line_col(&self, index: usize) -> (usize, usize) {
        let mut current_idx = 0;
        for (line_idx, line) in self.editor_text.split('\n').enumerate() {
            let line_len = line.chars().count();
            if current_idx + line_len >= index {
                return (line_idx, index - current_idx);
            }
            current_idx += line_len + 1; // +1 accounts for the \n char
        }
        (0, 0)
    }

    pub fn line_col_to_char_index(&self, line_idx: usize, col: usize) -> usize {
        let mut abs_index = 0;
        for (i, line) in self.editor_text.split('\n').enumerate() {
            if i < line_idx {
                abs_index += line.chars().count() + 1;
            } else if i == line_idx {
                abs_index += line.chars().count().min(col);
                break;
            }
        }
        abs_index
    }

    pub fn handle_vertical_edit_input(&mut self, ctx: &egui::Context, editor_id: egui::Id) {
        let toggle_mode = self
            .shortcuts
            .check_action(ctx, AppAction::ToggleVerticalEdit);

        if toggle_mode {
            ctx.input_mut(|i| {
                i.events.retain(|e| match e {
                    egui::Event::Paste(_) => false,
                    egui::Event::Text(t) if t.eq_ignore_ascii_case("v") || t == "√" => false,
                    _ => true,
                });
            });

            if self.vertical_cursor.is_some() {
                self.vertical_cursor = None;
                self.append_log("[SYSTEM] Vertical edit mode deactivated.");
            } else {
                if let Some(state) = egui::TextEdit::load_state(ctx, editor_id) {
                    let cursor_index = state
                        .cursor
                        .char_range()
                        .map(|r| r.primary.index)
                        .unwrap_or(0);
                    let (line, col) = self.char_index_to_line_col(cursor_index);

                    self.vertical_cursor = Some(VerticalCursor {
                        anchor_line: line,
                        active_line: line,
                        col,
                    });
                    self.scroll_to_vc = true;
                    self.last_vc_action_time = ctx.input(|i| i.time);
                    self.append_log("[SYSTEM] Vertical edit mode activated.");
                }
            }
            return;
        }

        let mut vc = match self.vertical_cursor {
            Some(v) => v,
            None => return,
        };

        let mut clear_vc = false;
        let mut cursor_moved = false;
        let mut text_changed = false;
        let total_lines = self.editor_text.split('\n').count().max(1);

        ctx.input_mut(|i| {
            let mut unhandled = Vec::new();
            for e in i.events.drain(..) {
                let mut consume = false;
                match &e {
                    egui::Event::PointerButton {
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        ..
                    } => {
                        clear_vc = true;
                    }
                    egui::Event::Key {
                        key, pressed: true, ..
                    } => match key {
                        egui::Key::Escape => {
                            clear_vc = true;
                            consume = true;
                        }
                        egui::Key::ArrowUp => {
                            vc.active_line = vc.active_line.saturating_sub(1);
                            consume = true;
                            cursor_moved = true;
                        }
                        egui::Key::ArrowDown => {
                            vc.active_line =
                                (vc.active_line + 1).min(total_lines.saturating_sub(1));
                            consume = true;
                            cursor_moved = true;
                        }
                        egui::Key::ArrowLeft => {
                            vc.col = vc.col.saturating_sub(1);
                            consume = true;
                            cursor_moved = true;
                        }
                        egui::Key::ArrowRight => {
                            vc.col += 1;
                            consume = true;
                            cursor_moved = true;
                        }
                        egui::Key::Backspace => {
                            vc.col = self.apply_vertical_backspace(&vc);
                            consume = true;
                            text_changed = true;
                            cursor_moved = true;
                        }
                        egui::Key::Enter => {
                            vc.col = self.apply_vertical_insert_text(&vc, "\n");
                            consume = true;
                            text_changed = true;
                            cursor_moved = true;
                        }
                        _ => {}
                    },
                    egui::Event::Text(text) => {
                        if !(text == "\t" && i.modifiers.alt) {
                            vc.col = self.apply_vertical_insert_text(&vc, text);
                            consume = true;
                            text_changed = true;
                            cursor_moved = true;
                        }
                    }
                    egui::Event::Paste(text) => {
                        vc.col = self.apply_vertical_insert_text(&vc, text);
                        consume = true;
                        text_changed = true;
                        cursor_moved = true;
                    }
                    _ => {}
                }
                if !consume {
                    unhandled.push(e);
                }
            }
            i.events = unhandled; // Put back any events we didn't eat
        });

        if clear_vc {
            self.vertical_cursor = None;
            self.append_log("[SYSTEM] Vertical edit mode deactivated.");
        } else {
            let new_total = self.editor_text.split('\n').count().max(1);
            vc.anchor_line = vc.anchor_line.min(new_total.saturating_sub(1));
            vc.active_line = vc.active_line.min(new_total.saturating_sub(1));
            self.vertical_cursor = Some(vc);

            if text_changed || cursor_moved {
                self.last_vc_action_time = ctx.input(|i| i.time); // Reset blink timer!
                self.scroll_to_vc = true; // Tell UI to update the camera!

                // Sync the hidden native cursor to keep logic aligned
                let active_idx = self.line_col_to_char_index(vc.active_line, vc.col);
                if let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id) {
                    let ccursor = egui::text::CCursor::new(active_idx);
                    state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                    egui::TextEdit::store_state(ctx, editor_id, state);
                }
            }
        }
    }

    fn apply_vertical_insert_text(&mut self, vc: &VerticalCursor, text: &str) -> usize {
        let lines: Vec<&str> = self.editor_text.split('\n').collect();
        let mut new_text = String::new();
        let resulting_col = vc.col + text.chars().count();

        let start_l = vc.anchor_line.min(vc.active_line);
        let end_l = vc.anchor_line.max(vc.active_line);

        for (idx, line) in lines.iter().enumerate() {
            if idx >= start_l && idx <= end_l {
                let char_count = line.chars().count();
                let insert_pos = char_count.min(vc.col);

                let (left, right) = split_at_char_index(line, insert_pos);
                new_text.push_str(left);
                new_text.push_str(text);
                new_text.push_str(right);
            } else {
                new_text.push_str(line);
            }
            if idx < lines.len() - 1 {
                new_text.push('\n');
            }
        }
        self.editor_text = new_text;
        resulting_col
    }

    fn apply_vertical_backspace(&mut self, vc: &VerticalCursor) -> usize {
        if vc.col == 0 {
            return 0;
        }
        let lines: Vec<&str> = self.editor_text.split('\n').collect();
        let mut new_text = String::new();
        let resulting_col = vc.col.saturating_sub(1);

        let start_l = vc.anchor_line.min(vc.active_line);
        let end_l = vc.anchor_line.max(vc.active_line);

        for (idx, line) in lines.iter().enumerate() {
            if idx >= start_l && idx <= end_l {
                let char_count = line.chars().count();
                let remove_pos = char_count.min(vc.col);

                if remove_pos > 0 {
                    let (left, right) = split_at_char_index(line, remove_pos);
                    let left_trimmed = slice_chars(left, 0, left.chars().count() - 1);
                    new_text.push_str(left_trimmed);
                    new_text.push_str(right);
                } else {
                    new_text.push_str(line);
                }
            } else {
                new_text.push_str(line);
            }
            if idx < lines.len() - 1 {
                new_text.push('\n');
            }
        }
        self.editor_text = new_text;
        resulting_col
    }
}

// Helpers for safe multi-byte unicode string slicing
fn split_at_char_index(s: &str, char_idx: usize) -> (&str, &str) {
    let byte_idx = s
        .char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    s.split_at(byte_idx)
}

fn slice_chars(s: &str, start: usize, end: usize) -> &str {
    let start_byte = s
        .char_indices()
        .nth(start)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    let end_byte = s.char_indices().nth(end).map(|(i, _)| i).unwrap_or(s.len());
    &s[start_byte..end_byte]
}
