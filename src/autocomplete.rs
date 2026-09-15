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

        // Sort and Deduplicate to completely eliminate repeated citation suggestions
        all_keys.sort();
        all_keys.dedup();
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

    // Fast O(k) summation of all vectors
    pub fn get_metrics(&self) -> (usize, usize) {
        let num_files = self.files.len();
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

        // Sort and Deduplicate to completely eliminate repeated label suggestions
        all_labels.sort();
        all_labels.dedup();
        all_labels
    }
}

pub fn get_file_suggestions(workspace: &Path, prefix: &str) -> Vec<String> {
    let mut suggestions = Vec::new();
    let search_term = prefix.to_lowercase();

    // Use walkdir to recursively search all files in the workspace.
    // We explicitly skip hidden directories (.git) and build folders (target, build, out)
    // to prevent duplicate matches and keep the UI lightning fast.
    let walker = walkdir::WalkDir::new(workspace)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.') && name != "target" && name != "build" && name != "out"
        });

    for entry in walker.flatten() {
        let path = entry.path();

        if path.is_file() {
            if let Ok(rel_path) = path.strip_prefix(workspace) {
                // Standardize slashes for cross-platform consistency
                let rel_path_str = rel_path.to_string_lossy().replace('\\', "/");

                // Allow substring matching in any position! (e.g., "fig11" matches "figs/data/fig11.png")
                if rel_path_str.to_lowercase().contains(&search_term) {
                    suggestions.push(rel_path_str);
                }
            }
        }
    }

    // Remove duplicate paths if any exist
    suggestions.sort();
    suggestions.dedup();
    suggestions
}

pub fn detect_context(text_up_to_cursor: &str) -> AutocompleteContext {
    // 1. Detect environment triggers (cite, ref, input, begin)
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
                || cmd_search_area.ends_with("\\label")
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
    let current_line = text_up_to_cursor.lines().last().unwrap_or("");

    // Safely grab the closest trigger symbol to avoid long-distance false positives
    let slash_idx = current_line.rfind('\\');
    let at_idx = current_line.rfind('@');
    let idx = match (slash_idx, at_idx) {
        (Some(s), Some(a)) => s.max(a),
        (Some(s), None) => s,
        (None, Some(a)) => a,
        (None, None) => return AutocompleteContext::None,
    };

    let slice = &current_line[idx..];
    if !slice.contains(|c: char| c.is_whitespace() || c == '{' || c == '}') {
        return AutocompleteContext::Macro(slice.to_string());
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

        if let Some((prefix, matches, mut selected_idx, start_byte, end_byte)) =
            self.active_menu.clone()
        {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
                // Wrap to top
                selected_idx = (selected_idx + 1) % matches.len();
                self.active_menu = Some((prefix, matches, selected_idx, start_byte, end_byte));
                autocomplete_handled = true;
            } else if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
                selected_idx = if selected_idx == 0 {
                    matches.len() - 1
                } else {
                    selected_idx - 1
                }; // Wrap to bottom
                self.active_menu = Some((prefix, matches, selected_idx, start_byte, end_byte));
                autocomplete_handled = true;
            } else if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab))
                || ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
            {
                // Unpack the 3-element tuple (display, insert, kind)
                let (_, insert_raw, _) = &matches[selected_idx];
                let mut insert_str = insert_raw.clone();
                let cursor_offset = if let Some(idx) = insert_str.find("$CURSOR$") {
                    let offset = insert_str.len() - (idx + "$CURSOR$".len());
                    insert_str = insert_str.replace("$CURSOR$", "");
                    offset
                } else {
                    0
                };

                // Because active_menu now stores byte indices, replace_range is completely safe
                self.editor_text
                    .replace_range(start_byte..end_byte, &insert_str);

                let new_byte_pos = start_byte + insert_str.len() - cursor_offset;
                let new_char_pos = self.editor_text[..new_byte_pos].chars().count();

                local_jump_request = Some((new_char_pos, new_char_pos));
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
            if cursor_range.primary.ccursor.index != cursor_range.secondary.ccursor.index {
                self.active_menu = None;
            }
        }

        let evaluate_autocomplete = self.active_menu.is_some() || output.response.changed();

        if evaluate_autocomplete && output.response.has_focus() && !autocomplete_handled {
            if let Some(cursor_range) = output.cursor_range {
                // egui provides a Character index
                let c_idx = cursor_range.primary.ccursor.index;
                if cursor_range.primary.ccursor.index == cursor_range.secondary.ccursor.index {
                    // Safely convert character index to byte index to avoid slicing panics on multi-byte chars
                    let byte_idx = self
                        .editor_text
                        .char_indices()
                        .nth(c_idx)
                        .map(|(i, _)| i)
                        .unwrap_or(self.editor_text.len());

                    let text_up_to_cursor = &self.editor_text[..byte_idx];
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
                        if active_prefix == &current_prefix && *active_end == byte_idx {
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
                                let search_term = prefix.to_lowercase();

                                let mut matches: Vec<(String, String, String)> = keys
                                    .into_iter()
                                    .filter(|k| k.to_lowercase().contains(&search_term))
                                    .map(|k| (k.clone(), k, "bib".to_string()))
                                    .collect();

                                matches.sort_by(|(a, _, _), (b, _, _)| {
                                    let a_lower = a.to_lowercase();
                                    let b_lower = b.to_lowercase();
                                    let a_starts = a_lower.starts_with(&search_term);
                                    let b_starts = b_lower.starts_with(&search_term);
                                    if a_starts && !b_starts {
                                        std::cmp::Ordering::Less
                                    } else if !a_starts && b_starts {
                                        std::cmp::Ordering::Greater
                                    } else {
                                        a_lower.cmp(&b_lower)
                                    }
                                });
                                matches.dedup_by(|a, b| a.0 == b.0);
                                matches.truncate(12);

                                if !matches.is_empty() {
                                    self.active_menu = Some((
                                        prefix.clone(),
                                        matches,
                                        0,
                                        byte_idx - prefix.len(),
                                        byte_idx,
                                    ));
                                } else {
                                    self.active_menu = None;
                                }
                            }
                            AutocompleteContext::Label(prefix) => {
                                let keys = self
                                    .label_cache
                                    .get_labels(Path::new(&self.config.build.working_directory));
                                let search_term = prefix.to_lowercase();

                                let mut matches: Vec<(String, String, String)> = keys
                                    .into_iter()
                                    .filter(|k| k.to_lowercase().contains(&search_term))
                                    .map(|k| (k.clone(), k, "label".to_string()))
                                    .collect();

                                matches.sort_by(|(a, _, _), (b, _, _)| {
                                    let a_lower = a.to_lowercase();
                                    let b_lower = b.to_lowercase();
                                    let a_starts = a_lower.starts_with(&search_term);
                                    let b_starts = b_lower.starts_with(&search_term);
                                    if a_starts && !b_starts {
                                        std::cmp::Ordering::Less
                                    } else if !a_starts && b_starts {
                                        std::cmp::Ordering::Greater
                                    } else {
                                        a_lower.cmp(&b_lower)
                                    }
                                });
                                matches.dedup_by(|a, b| a.0 == b.0);
                                matches.truncate(12);

                                if !matches.is_empty() {
                                    self.active_menu = Some((
                                        prefix.clone(),
                                        matches,
                                        0,
                                        byte_idx - prefix.len(),
                                        byte_idx,
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
                                let search_term = prefix.to_lowercase();

                                let mut matches: Vec<(String, String, String)> = files
                                    .into_iter()
                                    .map(|f| (f.clone(), f, "file".to_string()))
                                    .collect();

                                matches.sort_by(|(a, _, _), (b, _, _)| {
                                    let a_lower = a.to_lowercase();
                                    let b_lower = b.to_lowercase();

                                    // Extract the actual filename for smarter relevance scoring
                                    let a_name = Path::new(a)
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_lowercase();
                                    let b_name = Path::new(b)
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_lowercase();

                                    let a_name_starts = a_name.starts_with(&search_term);
                                    let b_name_starts = b_name.starts_with(&search_term);
                                    let a_name_contains = a_name.contains(&search_term);
                                    let b_name_contains = b_name.contains(&search_term);

                                    // 1. Filename STARTS with search term
                                    if a_name_starts && !b_name_starts {
                                        std::cmp::Ordering::Less
                                    } else if !a_name_starts && b_name_starts {
                                        std::cmp::Ordering::Greater
                                    }
                                    // 2. Filename CONTAINS search term
                                    else if a_name_contains && !b_name_contains {
                                        std::cmp::Ordering::Less
                                    } else if !a_name_contains && b_name_contains {
                                        std::cmp::Ordering::Greater
                                    }
                                    // 3. Fallback to full path alphabetical
                                    else {
                                        a_lower.cmp(&b_lower)
                                    }
                                });
                                matches.dedup_by(|a, b| a.0 == b.0);
                                matches.truncate(12);

                                if !matches.is_empty() {
                                    self.active_menu = Some((
                                        prefix.clone(),
                                        matches,
                                        0,
                                        byte_idx - prefix.len(),
                                        byte_idx,
                                    ));
                                } else {
                                    self.active_menu = None;
                                }
                            }
                            AutocompleteContext::Macro(prefix) => {
                                if self.dismissed_prefix.as_ref() != Some(&prefix) {
                                    let is_at_trigger = prefix.starts_with('@');
                                    let search_term = prefix
                                        .trim_start_matches(|c| c == '\\' || c == '@')
                                        .to_lowercase();

                                    let mut matches: Vec<_> = self
                                        .config
                                        .editor
                                        .autocomplete_cmds
                                        .iter()
                                        .filter(|c| {
                                            let c_is_at = c.trigger.starts_with('@');
                                            if is_at_trigger != c_is_at {
                                                return false;
                                            }

                                            // Stripping the symbol lets 'text' match inside '\mytextmacro'
                                            let c_name = c
                                                .trigger
                                                .trim_start_matches(|ch| ch == '\\' || ch == '@')
                                                .to_lowercase();
                                            c_name.contains(&search_term)
                                        })
                                        .map(|c| {
                                            (
                                                c.trigger.clone(),
                                                c.insert.clone(),
                                                "macro".to_string(),
                                            )
                                        })
                                        .collect();

                                    matches.sort_by(|(a, _, _), (b, _, _)| {
                                        let a_name = a
                                            .trim_start_matches(|ch| ch == '\\' || ch == '@')
                                            .to_lowercase();
                                        let b_name = b
                                            .trim_start_matches(|ch| ch == '\\' || ch == '@')
                                            .to_lowercase();

                                        let a_starts = a_name.starts_with(&search_term);
                                        let b_starts = b_name.starts_with(&search_term);

                                        if a_starts && !b_starts {
                                            std::cmp::Ordering::Less
                                        } else if !a_starts && b_starts {
                                            std::cmp::Ordering::Greater
                                        } else {
                                            a_name.cmp(&b_name)
                                        }
                                    });
                                    matches.dedup_by(|a, b| a.0 == b.0);
                                    matches.truncate(12);

                                    if !matches.is_empty() {
                                        self.active_menu = Some((
                                            prefix.clone(),
                                            matches,
                                            0,
                                            byte_idx - prefix.len(),
                                            byte_idx,
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
                let galley = &output.galley;
                let pos_in_galley = galley.pos_from_ccursor(cursor_range.primary.ccursor);
                let screen_pos = output.galley_pos
                    + pos_in_galley.min.to_vec2()
                    + egui::vec2(0.0, self.config.editor.font_size * 1.5);

                egui::Area::new(egui::Id::new("autocomplete_popup"))
                    .fixed_pos(screen_pos)
                    .order(egui::Order::Tooltip)
                    .show(ui.ctx(), |ui| {
                        egui::Frame::popup(ui.style())
                            .fill(bg_color)
                            .show(ui, |ui| {
                                ui.set_min_width(40.0);

                                // auto_shrink([false, true]) allows the width to expand for the longest word,
                                // but keeps it from shrinking horizontally below the min_width.
                                egui::ScrollArea::vertical()
                                    .max_height(500.0)
                                    .auto_shrink([true, true])
                                    .show(ui, |ui| {
                                        for (i, (display, _, kind)) in matches.iter().enumerate() {
                                            let is_selected = i == *selected_idx;

                                            // 1. Format the raw strings first.
                                            // Pad to 5 chars ("label", "macro", "bib  ", "file ")
                                            let prefix_marker =
                                                if is_selected { "▶" } else { " " };

                                            let kind_padded =
                                                format!("{} {:<5}", prefix_marker, kind);
                                            let word_string = format!("{}", display);

                                            // 2. Apply egui::RichText styling
                                            // Using .monospace() ensures the spaces actually align perfectly
                                            let mut type_text = egui::RichText::new(kind_padded)
                                                .size(12.0)
                                                .monospace();

                                            let mut word_text =
                                                egui::RichText::new(word_string).size(14.0);

                                            if is_selected {
                                                type_text = type_text
                                                    .color(highlight_color.linear_multiply(0.7));
                                                word_text =
                                                    word_text.color(highlight_color).strong();
                                            } else {
                                                type_text = type_text.weak();
                                            }

                                            // 3. The Left-Aligned Row Layout
                                            let row_resp = ui
                                                .horizontal(|ui| {
                                                    ui.label(type_text);
                                                    ui.label(word_text);
                                                })
                                                .response;

                                            // 3. Scroll Tracking
                                            if is_selected {
                                                row_resp.scroll_to_me(Some(egui::Align::Center));
                                            }
                                        }
                                    });
                            });
                    });
            }
        }
    }
}
