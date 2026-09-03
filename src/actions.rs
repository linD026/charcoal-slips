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

    pub fn open_file(&mut self, path: PathBuf, is_jump: bool) {
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

    pub fn handle_vertical_edit_input(&mut self, ctx: &egui::Context) {
        let mut vc = match self.vertical_cursor {
            Some(v) => v,
            None => return,
        };

        let mut clear_vc = false;
        let mut new_col = vc.col;
        let mut text_changed = false;

        ctx.input_mut(|i| {
            let mut unhandled = Vec::new();
            for e in i.events.drain(..) {
                let mut consume = false;
                match &e {
                    egui::Event::Text(text) => {
                        // Let alt+tabs pass through if needed, else eat it
                        if !(text == "\t" && i.modifiers.alt) {
                            new_col = self.apply_vertical_insert_text(&vc, text);
                            consume = true;
                            text_changed = true;
                        }
                    }
                    egui::Event::Paste(text) => {
                        new_col = self.apply_vertical_insert_text(&vc, text);
                        consume = true;
                        text_changed = true;
                    }
                    egui::Event::Key {
                        key, pressed: true, ..
                    } => match key {
                        egui::Key::Backspace => {
                            new_col = self.apply_vertical_backspace(&vc);
                            consume = true;
                            text_changed = true;
                        }
                        egui::Key::Enter => {
                            new_col = self.apply_vertical_insert_text(&vc, "\n");
                            consume = true;
                            text_changed = true;
                        }
                        egui::Key::Escape
                        | egui::Key::ArrowUp
                        | egui::Key::ArrowDown
                        | egui::Key::ArrowLeft
                        | egui::Key::ArrowRight => {
                            clear_vc = true;
                        }
                        _ => {}
                    },
                    _ => {}
                }
                if !consume {
                    unhandled.push(e);
                }
            }
            i.events = unhandled;
        });

        if clear_vc {
            self.vertical_cursor = None;
        } else if text_changed {
            vc.col = new_col;
            self.vertical_cursor = Some(vc);
        }
    }

    fn apply_vertical_insert_text(&mut self, vc: &VerticalCursor, text: &str) -> usize {
        let mut lines: Vec<&str> = self.editor_text.split('\n').collect();
        let mut new_text = String::new();
        let resulting_col = vc.col + text.chars().count();

        for (idx, line) in lines.iter().enumerate() {
            if idx >= vc.start_line && idx <= vc.end_line {
                let char_count = line.chars().count();
                // OPTION B: if line is too short, we append to the end.
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
        let mut lines: Vec<&str> = self.editor_text.split('\n').collect();
        let mut new_text = String::new();
        let resulting_col = vc.col.saturating_sub(1);

        for (idx, line) in lines.iter().enumerate() {
            if idx >= vc.start_line && idx <= vc.end_line {
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
