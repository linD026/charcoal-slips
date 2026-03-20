// backend
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir;

// frontend
use crate::CCslipsApp;
use crate::config::parse_hex;
use eframe::egui;

pub enum AutocompleteContext {
    Macro(String),
    Citation(String),
    File(String),
    Label(String),
    None,
}

pub struct BibCache {
    files: HashMap<PathBuf, (SystemTime, Vec<String>)>,
}

impl BibCache {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    pub fn get_metrics(&self) -> (usize, usize) {
        let num_files = self.files.len();
        // Fast O(k) summation of all vectors
        let num_keys: usize = self.files.values().map(|(_, keys)| keys.len()).sum();
        (num_files, num_keys)
    }

    pub fn get_keys(&mut self, workspace: &Path, bib_dir: &str) -> Vec<String> {
        let mut all_keys = Vec::new();
        let full_dir = workspace.join(bib_dir);

        // This Regex matches ANY BibTeX entry type (article, misc, inproceedings, etc.)
        // It safely captures the citation key while ignoring whitespace and case.
        let re = Regex::new(r"@(?i)[a-zA-Z]+\s*\{\s*([^,\s]+)\s*,").unwrap();

        if let Ok(entries) = fs::read_dir(full_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().unwrap_or_default() == "bib" {
                    let modified = fs::metadata(&path)
                        .and_then(|m| m.modified())
                        .unwrap_or(SystemTime::UNIX_EPOCH);

                    let needs_update = match self.files.get(&path) {
                        Some((last_mod, _)) => *last_mod < modified,
                        None => true,
                    };

                    if needs_update {
                        if let Ok(content) = fs::read_to_string(&path) {
                            let keys: Vec<String> = re
                                .captures_iter(&content)
                                .filter_map(|cap| {
                                    let full_match = cap[0].to_lowercase();
                                    // We don't want to autocomplete @string or @comment variables
                                    if full_match.starts_with("@string")
                                        || full_match.starts_with("@comment")
                                    {
                                        None
                                    } else {
                                        Some(cap[1].to_string())
                                    }
                                })
                                .collect();
                            self.files.insert(path.clone(), (modified, keys));
                        }
                    }
                    if let Some((_, cached_keys)) = self.files.get(&path) {
                        all_keys.extend(cached_keys.clone());
                    }
                }
            }
        }
        all_keys
    }
}

pub struct LabelCache {
    files: HashMap<PathBuf, (SystemTime, Vec<String>)>,
}

impl LabelCache {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    pub fn get_metrics(&self) -> (usize, usize) {
        let num_files = self.files.len();
        // Fast O(k) summation of all vectors
        let num_labels: usize = self.files.values().map(|(_, labels)| labels.len()).sum();
        (num_files, num_labels)
    }

    pub fn get_labels(&mut self, workspace: &Path) -> Vec<String> {
        let mut all_labels = Vec::new();
        let re = Regex::new(r"\\label\{([^}]+)\}").unwrap();

        for entry in walkdir::WalkDir::new(workspace).into_iter().flatten() {
            let path = entry.path();
            if path.extension().unwrap_or_default() == "tex" {
                let modified = fs::metadata(path)
                    .and_then(|m| m.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);

                let needs_update = match self.files.get(path) {
                    Some((last_mod, _)) => *last_mod < modified,
                    None => true,
                };

                if needs_update {
                    if let Ok(content) = fs::read_to_string(path) {
                        let labels: Vec<String> = re
                            .captures_iter(&content)
                            .map(|cap| cap[1].to_string())
                            .collect();
                        self.files.insert(path.to_path_buf(), (modified, labels));
                    }
                }
                if let Some((_, cached)) = self.files.get(path) {
                    all_labels.extend(cached.clone());
                }
            }
        }
        all_labels
    }
}

pub fn get_file_suggestions(workspace: &Path, prefix: &str) -> Vec<String> {
    let mut suggestions = Vec::new();
    let (dir_part, file_part) = if let Some(last_slash) = prefix.rfind('/') {
        (&prefix[..=last_slash], &prefix[last_slash + 1..])
    } else {
        ("", prefix)
    };

    let search_dir = workspace.join(dir_part);
    if let Ok(entries) = fs::read_dir(&search_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(file_part) && !name.starts_with('.') {
                let suffix = if entry.path().is_dir() { "/" } else { "" };
                suggestions.push(format!("{}{}{}", dir_part, name, suffix));
            }
        }
    }
    suggestions
}

pub fn detect_context(text_up_to_cursor: &str) -> AutocompleteContext {
    // 1. Detect environment triggers (cite, ref, input)
    if let Some(brace_idx) = text_up_to_cursor.rfind('{') {
        let text_after_brace = &text_up_to_cursor[brace_idx..];

        // Ensure we are actively typing inside the brace
        if !text_after_brace.contains('}') {
            let before_brace = text_up_to_cursor[..brace_idx].trim_end();

            // Safely isolate the command without greedy-searching the whole file
            let mut cmd_search_area = before_brace;

            // Only look for a '[' if the string immediately before our '{' ends with ']'
            if cmd_search_area.ends_with(']') {
                if let Some(bracket_start) = cmd_search_area.rfind('[') {
                    // Safety check: ensure we didn't jump across unrelated brackets
                    if !cmd_search_area[bracket_start..].contains('{')
                        && !cmd_search_area[bracket_start..].contains('}')
                    {
                        cmd_search_area = cmd_search_area[..bracket_start].trim_end();
                    }
                }
            }

            let mut search_term = text_up_to_cursor[brace_idx + 1..].to_string();

            // Check for Citation
            if cmd_search_area.ends_with("\\cite")
                || cmd_search_area.ends_with("\\citep")
                || cmd_search_area.ends_with("\\citet")
                || cmd_search_area.ends_with("\\citealt")
                || cmd_search_area.ends_with("\\citealp")
                || cmd_search_area.ends_with("\\citeauthor")
                || cmd_search_area.ends_with("\\citeyear")
                || cmd_search_area.ends_with("\\footcite")
                || cmd_search_area.ends_with("\\textcite")
                || cmd_search_area.ends_with("\\parencite")
                || cmd_search_area.ends_with("\\nocite")
            {
                if let Some(last_comma) = search_term.rfind(',') {
                    search_term = search_term[last_comma + 1..].trim_start().to_string();
                }
                return AutocompleteContext::Citation(search_term);
            }
            // Check for Labels
            else if cmd_search_area.ends_with("\\ref")
                || cmd_search_area.ends_with("\\cref")
                || cmd_search_area.ends_with("\\autoref")
                || cmd_search_area.ends_with("\\nameref")
            {
                if let Some(last_comma) = search_term.rfind(',') {
                    search_term = search_term[last_comma + 1..].trim_start().to_string();
                }
                return AutocompleteContext::Label(search_term);
            }
            // Check for Files
            else if cmd_search_area.ends_with("\\includegraphics")
                || cmd_search_area.ends_with("\\input")
                || cmd_search_area.ends_with("\\bibliographystyle")
                || cmd_search_area.ends_with("\\bibliography")
            {
                return AutocompleteContext::File(search_term);
            }
        }
    }

    // 2. Detect Macro triggers (e.g., typing \tex...)
    // Restrict macro detection to the CURRENT line.
    // This prevents a runaway '\' from 10 lines up from crashing the context engine.
    let current_line = text_up_to_cursor.lines().last().unwrap_or("");
    if let (Some(idx), _) | (_, Some(idx)) = (current_line.rfind('\\'), current_line.rfind("@")) {
        let slice = &current_line[idx..];
        // Ensure the user is actively typing a macro (no spaces or braces allowed yet)
        if !slice.contains(|c: char| c.is_whitespace() || c == '{' || c == '}') {
            return AutocompleteContext::Macro(slice.to_string());
        }
    }

    AutocompleteContext::None
}

impl CCslipsApp {
    pub fn intercept_autocomplete_navigation(
        &mut self,
        ui: &mut egui::Ui,
        editor_id: egui::Id,
    ) -> (bool, Option<(usize, usize)>) {
        let mut autocomplete_handled = false;
        let mut local_jump_request = None;

        if let Some((prefix, matches, mut selected_idx, start, end)) = self.active_menu.clone() {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
                selected_idx = (selected_idx + 1) % matches.len(); // Wrap to top
                self.active_menu = Some((prefix, matches, selected_idx, start, end));
                autocomplete_handled = true;
            } else if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
                selected_idx = if selected_idx == 0 {
                    matches.len() - 1
                } else {
                    selected_idx - 1
                }; // Wrap to bottom
                self.active_menu = Some((prefix, matches, selected_idx, start, end));
                autocomplete_handled = true;
            } else if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab))
                || ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
            {
                let (_, insert_raw) = &matches[selected_idx];
                let mut insert_str = insert_raw.clone();
                let cursor_offset = if let Some(idx) = insert_str.find("$CURSOR$") {
                    let offset = insert_str.len() - (idx + "$CURSOR$".len());
                    insert_str = insert_str.replace("$CURSOR$", "");
                    offset
                } else {
                    0
                };

                self.editor_text.replace_range(start..end, &insert_str);
                let new_pos = start + insert_str.len() - cursor_offset;
                local_jump_request = Some((new_pos, new_pos));
                self.active_menu = None;
                self.dismissed_prefix = None;
                autocomplete_handled = true;
            } else if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                self.dismissed_prefix = Some(prefix);
                self.active_menu = None;
                autocomplete_handled = true;
                ui.ctx().memory_mut(|mem| mem.request_focus(editor_id));
            }
        }
        (autocomplete_handled, local_jump_request)
    }

    pub fn update_autocomplete_state(
        &mut self,
        output: &egui::text_edit::TextEditOutput,
        autocomplete_handled: bool,
    ) {
        if let Some(cursor_range) = output.cursor_range {
            // Dismiss if text is selected
            if cursor_range.primary.ccursor.index != cursor_range.secondary.ccursor.index {
                self.active_menu = None;
            }
        }

        // Only evaluate if already open, OR if user actually typed something
        let evaluate_autocomplete = self.active_menu.is_some() || output.response.changed();

        if evaluate_autocomplete && output.response.has_focus() && !autocomplete_handled {
            if let Some(cursor_range) = output.cursor_range {
                let c_idx = cursor_range.primary.ccursor.index;
                if c_idx <= self.editor_text.len()
                    && cursor_range.primary.ccursor.index == cursor_range.secondary.ccursor.index
                {
                    let text_up_to_cursor = &self.editor_text[..c_idx];
                    let context = detect_context(text_up_to_cursor);

                    let current_prefix = match &context {
                        AutocompleteContext::Macro(p)
                        | AutocompleteContext::Citation(p)
                        | AutocompleteContext::Label(p)
                        | AutocompleteContext::File(p) => p.clone(),
                        AutocompleteContext::None => String::new(),
                    };

                    let mut needs_update = true;
                    if let Some((active_prefix, _, _, _, active_end)) = &self.active_menu {
                        if active_prefix == &current_prefix && *active_end == c_idx {
                            needs_update = false;
                        }
                    }

                    if needs_update {
                        match context {
                            AutocompleteContext::Citation(prefix) => {
                                let keys = self.bib_cache.get_keys(
                                    Path::new(&self.config.build.working_directory),
                                    &self.config.editor.bib_dir,
                                );
                                let matches: Vec<(String, String)> = keys
                                    .into_iter()
                                    .filter(|k| k.to_lowercase().contains(&prefix.to_lowercase()))
                                    .map(|k| (k.clone(), k))
                                    .take(8)
                                    .collect();
                                if !matches.is_empty() {
                                    self.active_menu = Some((
                                        prefix.clone(),
                                        matches,
                                        0,
                                        c_idx - prefix.len(),
                                        c_idx,
                                    ));
                                } else {
                                    self.active_menu = None;
                                }
                            }
                            AutocompleteContext::Label(prefix) => {
                                let keys = self
                                    .label_cache
                                    .get_labels(Path::new(&self.config.build.working_directory));
                                let matches: Vec<(String, String)> = keys
                                    .into_iter()
                                    .filter(|k| k.to_lowercase().contains(&prefix.to_lowercase()))
                                    .map(|k| (k.clone(), k))
                                    .take(8)
                                    .collect();
                                if !matches.is_empty() {
                                    self.active_menu = Some((
                                        prefix.clone(),
                                        matches,
                                        0,
                                        c_idx - prefix.len(),
                                        c_idx,
                                    ));
                                } else {
                                    self.active_menu = None;
                                }
                            }
                            AutocompleteContext::File(prefix) => {
                                let files = get_file_suggestions(
                                    Path::new(&self.config.build.working_directory),
                                    &prefix,
                                );
                                let matches: Vec<(String, String)> =
                                    files.into_iter().map(|f| (f.clone(), f)).take(8).collect();
                                if !matches.is_empty() {
                                    self.active_menu = Some((
                                        prefix.clone(),
                                        matches,
                                        0,
                                        c_idx - prefix.len(),
                                        c_idx,
                                    ));
                                } else {
                                    self.active_menu = None;
                                }
                            }
                            AutocompleteContext::Macro(prefix) => {
                                if self.dismissed_prefix.as_ref() != Some(&prefix) {
                                    let mut matches: Vec<_> = self
                                        .config
                                        .editor
                                        .autocomplete_cmds
                                        .iter()
                                        .filter(|c| {
                                            c.trigger
                                                .to_lowercase()
                                                .contains(&prefix.to_lowercase())
                                        })
                                        .map(|c| (c.trigger.clone(), c.insert.clone()))
                                        .collect();
                                    matches.sort_by(|(a, _), (b, _)| {
                                        let a_starts = a.starts_with(&prefix);
                                        let b_starts = b.starts_with(&prefix);
                                        if a_starts && !b_starts {
                                            std::cmp::Ordering::Less
                                        } else if !a_starts && b_starts {
                                            std::cmp::Ordering::Greater
                                        } else {
                                            a.cmp(b)
                                        }
                                    });
                                    matches.truncate(8);
                                    if !matches.is_empty() {
                                        self.active_menu = Some((
                                            prefix.clone(),
                                            matches,
                                            0,
                                            c_idx - prefix.len(),
                                            c_idx,
                                        ));
                                    } else {
                                        self.active_menu = None;
                                    }
                                }
                            }
                            AutocompleteContext::None => {
                                self.active_menu = None;
                                self.dismissed_prefix = None;
                            }
                        }
                    }
                }
            }
        }
    }
    pub fn draw_autocomplete_popup(
        &self,
        ui: &mut egui::Ui,
        output: &egui::text_edit::TextEditOutput,
    ) {
        if let Some((_, matches, selected_idx, _, _)) = &self.active_menu {
            let theme = if self.config.ui.dark_mode {
                &self.config.ui.dark_theme
            } else {
                &self.config.ui.light_theme
            };

            let bg_color = parse_hex(&theme.ui.popup_bg);
            let highlight_color = parse_hex(&theme.ui.popup_selected_text);

            if let Some(cursor_range) = output.cursor_range {
                // Restored the actual geometry math so the popup tracks the cursor
                let galley = &output.galley;
                let pos_in_galley = galley.pos_from_ccursor(cursor_range.primary.ccursor);
                let screen_pos = output.galley_pos
                    + pos_in_galley.min.to_vec2()
                    + egui::vec2(0.0, self.config.editor.font_size * 1.5);

                egui::Area::new(egui::Id::new("autocomplete_popup"))
                    .fixed_pos(screen_pos)
                    .order(egui::Order::Tooltip)
                    .show(ui.ctx(), |ui| {
                        // Use our custom background frame
                        egui::Frame::popup(ui.style())
                            .fill(bg_color)
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    for (i, (display, _)) in matches.iter().enumerate() {
                                        if i == *selected_idx {
                                            ui.label(
                                                egui::RichText::new(format!("▶ {}", display))
                                                    .color(highlight_color)
                                                    .strong(),
                                            );
                                        } else {
                                            ui.label(egui::RichText::new(format!("  {}", display)));
                                        }
                                    }
                                });
                            });
                    });
            }
        }
    }
}
