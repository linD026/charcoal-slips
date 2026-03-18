use crate::CCslipsApp;
use eframe::egui;
use std::fs;
use std::path::PathBuf;

#[derive(Default, Clone, PartialEq)]
pub struct SearchResult {
    pub file: PathBuf,
    pub start: usize,
    pub end: usize,
}

#[derive(Default)]
pub struct SearchState {
    pub is_active: bool,
    pub find_query: String,
    pub replace_query: String,
    pub search_all_files: bool,
    pub matches: Vec<SearchResult>,
    pub current_match_idx: usize,
    pub has_reached_end: bool,
    pub focus_find: bool,
    pub focus_replace: bool,
    pub query_modified: bool,
}

impl CCslipsApp {
    pub fn perform_search(&mut self, keep_proximity: bool, auto_jump: bool) {
        self.search_state.query_modified = false;

        let old_current_match = if keep_proximity {
            self.search_state
                .matches
                .get(self.search_state.current_match_idx)
                .cloned()
        } else {
            None
        };

        self.search_state.matches.clear();
        self.search_state.current_match_idx = 0;
        self.search_state.has_reached_end = false;

        if self.search_state.find_query.is_empty() {
            return;
        }

        let query = &self.search_state.find_query;
        let query_char_len = query.chars().count();
        let current_path = self.current_file.clone();

        if self.search_state.search_all_files {
            let walker = walkdir::WalkDir::new(&self.config.build.working_directory);
            let mut sorted_entries: Vec<_> = walker.into_iter().flatten().collect();
            sorted_entries.sort_by(|a, b| a.path().cmp(b.path()));

            for entry in sorted_entries {
                let path = entry.path();
                if path.is_file() && !path.to_string_lossy().contains(".git") {
                    // BUG FIX: Use the live memory buffer if this is the currently open file!
                    let content = if current_path.as_ref() == Some(&path.to_path_buf()) {
                        self.editor_text.clone()
                    } else {
                        fs::read_to_string(path).unwrap_or_default()
                    };

                    let mut absolute_byte_start = 0;
                    let mut absolute_char_start = 0; // O(n) Optimization

                    while let Some(idx) = content[absolute_byte_start..].find(query) {
                        let match_byte_pos = absolute_byte_start + idx;

                        let skipped_chars =
                            content[absolute_byte_start..match_byte_pos].chars().count();
                        let char_start = absolute_char_start + skipped_chars;

                        self.search_state.matches.push(SearchResult {
                            file: path.to_path_buf(),
                            start: char_start,
                            end: char_start + query_char_len,
                        });

                        absolute_byte_start = match_byte_pos + query.len();
                        absolute_char_start = char_start + query_char_len;
                    }
                }
            }
        } else {
            let path = current_path.unwrap_or_default();
            let content = &self.editor_text;

            let mut absolute_byte_start = 0;
            let mut absolute_char_start = 0;

            while let Some(idx) = content[absolute_byte_start..].find(query) {
                let match_byte_pos = absolute_byte_start + idx;

                let skipped_chars = content[absolute_byte_start..match_byte_pos].chars().count();
                let char_start = absolute_char_start + skipped_chars;

                self.search_state.matches.push(SearchResult {
                    file: path.clone(),
                    start: char_start,
                    end: char_start + query_char_len,
                });

                absolute_byte_start = match_byte_pos + query.len();
                absolute_char_start = char_start + query_char_len;
            }
        }

        if let Some(old_m) = old_current_match {
            let mut closest_dist = usize::MAX;
            for (i, m) in self.search_state.matches.iter().enumerate() {
                if m.file == old_m.file {
                    let dist = (m.start as isize - old_m.start as isize).abs() as usize;
                    if dist < closest_dist {
                        closest_dist = dist;
                        self.search_state.current_match_idx = i;
                    }
                }
            }
        }

        if auto_jump && !self.search_state.matches.is_empty() {
            self.jump_to_current_match();
        }
    }

    pub fn jump_to_current_match(&mut self) {
        if self.search_state.matches.is_empty() {
            return;
        }

        self.search_state.current_match_idx = self
            .search_state
            .current_match_idx
            .min(self.search_state.matches.len().saturating_sub(1));

        let match_item = &self.search_state.matches[self.search_state.current_match_idx].clone();

        if self.current_file.as_ref() != Some(&match_item.file) && match_item.file.exists() {
            // Auto-save before jumping away
            if let Some(path) = &self.current_file {
                let _ = fs::write(path, &self.editor_text);
            }
            if let Ok(content) = fs::read_to_string(&match_item.file) {
                self.editor_text = content;
                self.current_file = Some(match_item.file.clone());
            }
        }

        self.jump_request = Some((match_item.start, match_item.end));
    }

    pub fn replace_current_match(&mut self) {
        if self.search_state.matches.is_empty()
            || self.search_state.has_reached_end
            || self.search_state.query_modified
        {
            return;
        }

        self.jump_to_current_match();

        let current_order = self.search_state.current_match_idx;
        let match_item = self.search_state.matches[current_order].clone();

        if match_item.start <= self.editor_text.chars().count()
            && match_item.end <= self.editor_text.chars().count()
        {
            let before: String = self.editor_text.chars().take(match_item.start).collect();
            let after: String = self.editor_text.chars().skip(match_item.end).collect();
            self.editor_text = format!("{}{}{}", before, self.search_state.replace_query, after);
        }

        self.perform_search(false, false);

        let mut target_idx = current_order;
        let inserted_len = self.search_state.replace_query.chars().count();

        if target_idx < self.search_state.matches.len() {
            let new_m = &self.search_state.matches[target_idx];
            if new_m.file == match_item.file
                && new_m.start >= match_item.start
                && new_m.start < match_item.start + inserted_len
            {
                target_idx += 1;
            }
        }

        if target_idx < self.search_state.matches.len() {
            self.search_state.current_match_idx = target_idx;
            self.jump_to_current_match();
        } else {
            self.search_state.has_reached_end = true;
            let new_end = match_item.start + inserted_len;
            self.jump_request = Some((new_end, new_end));
        }
    }

    pub fn replace_all_matches(&mut self) {
        if self.search_state.find_query.is_empty() || self.search_state.query_modified {
            return;
        }

        let query = &self.search_state.find_query;
        let replace_with = &self.search_state.replace_query;

        if self.search_state.search_all_files {
            // Force a save to disk before global replace so we don't miss unsaved memory edits
            if let Some(path) = &self.current_file {
                let _ = fs::write(path, &self.editor_text);
            }

            let walker = walkdir::WalkDir::new(&self.config.build.working_directory);
            for entry in walker.into_iter().flatten() {
                let path = entry.path();
                if path.is_file() && !path.to_string_lossy().contains(".git") {
                    if let Ok(content) = fs::read_to_string(path) {
                        if content.contains(query) {
                            let new_content = content.replace(query, replace_with);
                            let _ = fs::write(path, &new_content);

                            // Synchronize memory buffer if it was the active file
                            if Some(path.to_path_buf()) == self.current_file {
                                self.editor_text = new_content;
                            }
                        }
                    }
                }
            }
        } else {
            self.editor_text = self.editor_text.replace(query, replace_with);
        }

        self.perform_search(false, false);
    }

    pub fn render_search_replace_panel(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Find:   ");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.search_state.find_query)
                        .desired_width(120.0),
                );

                if self.search_state.focus_find {
                    response.request_focus();
                    self.search_state.focus_find = false;
                }

                if response.changed() {
                    self.search_state.query_modified = true;
                    self.search_state.matches.clear();
                    self.search_state.has_reached_end = false;
                }

                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.perform_search(false, true);
                    response.request_focus();
                }

                if self.search_state.query_modified && !self.search_state.find_query.is_empty() {
                    ui.label(
                        egui::RichText::new("⏎ Press Enter to Search")
                            .weak()
                            .italics()
                            .size(11.0),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✖").clicked() {
                        self.search_state.is_active = false;
                        self.search_state.matches.clear();
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.label("Replace:");
                let replace_res = ui.add(
                    egui::TextEdit::singleline(&mut self.search_state.replace_query)
                        .desired_width(120.0),
                );

                if self.search_state.focus_replace {
                    replace_res.request_focus();
                    self.search_state.focus_replace = false;
                }

                if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::R)) {
                    replace_res.request_focus();
                }
            });

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                let match_text = if self.search_state.matches.is_empty() {
                    "0/0".to_string()
                } else if self.search_state.has_reached_end {
                    format!("End ({} total)", self.search_state.matches.len())
                } else {
                    format!(
                        "{}/{}",
                        self.search_state.current_match_idx + 1,
                        self.search_state.matches.len()
                    )
                };

                let navigation_enabled =
                    !self.search_state.query_modified && !self.search_state.matches.is_empty();

                if ui
                    .add_enabled(navigation_enabled, egui::Button::new("△"))
                    .on_hover_text("Previous")
                    .clicked()
                {
                    self.search_state.has_reached_end = false;
                    self.search_state.current_match_idx =
                        if self.search_state.current_match_idx == 0 {
                            self.search_state.matches.len() - 1
                        } else {
                            self.search_state.current_match_idx - 1
                        };
                    self.jump_to_current_match();
                }

                ui.label(match_text);

                if ui
                    .add_enabled(navigation_enabled, egui::Button::new("▽"))
                    .on_hover_text("Next")
                    .clicked()
                {
                    self.search_state.has_reached_end = false;
                    self.search_state.current_match_idx =
                        (self.search_state.current_match_idx + 1) % self.search_state.matches.len();
                    self.jump_to_current_match();
                }

                if ui
                    .checkbox(&mut self.search_state.search_all_files, "All Files")
                    .changed()
                {
                    if !self.search_state.query_modified {
                        self.perform_search(false, true);
                    }
                }
            });

            ui.horizontal(|ui| {
                let replace_enabled = !self.search_state.query_modified
                    && !self.search_state.has_reached_end
                    && !self.search_state.matches.is_empty();

                if ui
                    .add_enabled(replace_enabled, egui::Button::new("Replace"))
                    .clicked()
                {
                    self.replace_current_match();
                }

                if ui
                    .add_enabled(
                        !self.search_state.query_modified
                            && !self.search_state.find_query.is_empty(),
                        egui::Button::new("Replace All"),
                    )
                    .clicked()
                {
                    self.replace_all_matches();
                }
            });
        });
    }
}
