use crate::ai::trigger_ai_indexing;
use crate::config::parse_hex;
use crate::shortcuts::AppAction;
use crate::syntax_highlights::highlight_latex;
use crate::{CCslipsApp, RightTab};
use eframe::egui;

// ==========================================
// ENVIRONMENT SYNCHRONIZATION ENGINE
// ==========================================
// Finds the structurally matching \end or \begin tag, and returns a replacement action if they diverge
fn get_sync_env_edits(
    text: &str,
    cursor_byte_idx: usize,
) -> Option<(std::ops::Range<usize>, String)> {
    let max_lookback = cursor_byte_idx.saturating_sub(100);
    let mut brace_start = None;
    for (i, c) in text[max_lookback..cursor_byte_idx].char_indices().rev() {
        if c == '{' {
            brace_start = Some(max_lookback + i);
            break;
        }
        if c == '}' || c == '\n' {
            return None;
        }
    }
    let brace_start = brace_start?;

    let max_lookforward = (cursor_byte_idx + 100).min(text.len());
    let mut brace_end = None;
    for (i, c) in text[cursor_byte_idx..max_lookforward].char_indices() {
        if c == '}' {
            brace_end = Some(cursor_byte_idx + i);
            break;
        }
        if c == '{' || c == '\n' {
            return None;
        }
    }
    let brace_end = brace_end?;

    let before_brace = &text[..brace_start];
    let is_begin = if before_brace.ends_with("\\begin") {
        true
    } else if before_brace.ends_with("\\end") {
        false
    } else {
        return None;
    };

    let current_env_name = text[brace_start + 1..brace_end].to_string();

    if is_begin {
        // Structurally search forwards for matching \end
        let mut depth = 1;
        let mut search_idx = brace_end + 1;
        while search_idx < text.len() {
            let next_begin = text[search_idx..].find("\\begin{").map(|i| search_idx + i);
            let next_end = text[search_idx..].find("\\end{").map(|i| search_idx + i);

            match (next_begin, next_end) {
                (Some(b), Some(e)) => {
                    if b < e {
                        depth += 1;
                        search_idx = b + 7;
                    } else {
                        depth -= 1;
                        if depth == 0 {
                            if let Some(end_brace_offset) = text[e + 5..].find('}') {
                                let match_start = e + 5;
                                let match_end = match_start + end_brace_offset;
                                if &text[match_start..match_end] != current_env_name {
                                    return Some((match_start..match_end, current_env_name));
                                }
                            }
                            return None;
                        }
                        search_idx = e + 5;
                    }
                }
                (Some(b), None) => {
                    depth += 1;
                    search_idx = b + 7;
                }
                (None, Some(e)) => {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(end_brace_offset) = text[e + 5..].find('}') {
                            let match_start = e + 5;
                            let match_end = match_start + end_brace_offset;
                            if &text[match_start..match_end] != current_env_name {
                                return Some((match_start..match_end, current_env_name));
                            }
                        }
                        return None;
                    }
                    search_idx = e + 5;
                }
                (None, None) => break,
            }
        }
    } else {
        // Structurally search backwards for matching \begin
        let mut depth = 1;
        let mut search_idx = before_brace.rfind("\\end").unwrap_or(0);
        while search_idx > 0 {
            let prev_begin = text[..search_idx].rfind("\\begin{");
            let prev_end = text[..search_idx].rfind("\\end{");

            match (prev_begin, prev_end) {
                (Some(b), Some(e)) => {
                    if e > b {
                        depth += 1;
                        search_idx = e;
                    } else {
                        depth -= 1;
                        if depth == 0 {
                            if let Some(end_brace_offset) = text[b + 7..].find('}') {
                                let match_start = b + 7;
                                let match_end = match_start + end_brace_offset;
                                if &text[match_start..match_end] != current_env_name {
                                    return Some((match_start..match_end, current_env_name));
                                }
                            }
                            return None;
                        }
                        search_idx = b;
                    }
                }
                (Some(b), None) => {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(end_brace_offset) = text[b + 7..].find('}') {
                            let match_start = b + 7;
                            let match_end = match_start + end_brace_offset;
                            if &text[match_start..match_end] != current_env_name {
                                return Some((match_start..match_end, current_env_name));
                            }
                        }
                        return None;
                    }
                    search_idx = b;
                }
                (None, Some(e)) => {
                    depth += 1;
                    search_idx = e;
                }
                (None, None) => break,
            }
        }
    }
    None
}

// ==========================================
// BRACKET PAIR MATCHING
// ==========================================
fn find_matching_brackets(text: &str, cursor_idx: usize) -> Option<(usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let pairs = [('(', ')'), ('[', ']'), ('{', '}')];

    let check_idx = |idx: usize| -> Option<(usize, usize)> {
        if idx >= chars.len() {
            return None;
        }
        let c = chars[idx];

        for &(open, close) in &pairs {
            if c == open {
                let mut depth = 1;
                for i in (idx + 1)..chars.len() {
                    if chars[i] == open {
                        depth += 1;
                    } else if chars[i] == close {
                        depth -= 1;
                    }
                    if depth == 0 {
                        return Some((idx, i));
                    }
                }
            }
        }

        for &(open, close) in &pairs {
            if c == close {
                let mut depth = 1;
                for i in (0..idx).rev() {
                    if chars[i] == close {
                        depth += 1;
                    } else if chars[i] == open {
                        depth -= 1;
                    }
                    if depth == 0 {
                        return Some((i, idx));
                    }
                }
            }
        }
        None
    };

    check_idx(cursor_idx).or_else(|| cursor_idx.checked_sub(1).and_then(check_idx))
}

impl CCslipsApp {
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
            //if ui.button("💾 Save & Build (Ctrl+S)").clicked() {
            if ui.button("💾 Save (Ctrl+S)").clicked() {
                self.save_current_file();
                self.execute_build();
            }
            if ui.button("🚀 Build").clicked() {
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

            if ui.button("🔍-").on_hover_text("Zoom Out UI").clicked() {
                self.config.ui.zoom_factor = (self.config.ui.zoom_factor - 0.1).clamp(0.5, 3.0);
            }
            if ui.button("🔍+").on_hover_text("Zoom In UI").clicked() {
                self.config.ui.zoom_factor = (self.config.ui.zoom_factor + 0.1).clamp(0.5, 3.0);
            }
            ui.separator();

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

            if ui.button("❓ Help").clicked() {
                self.show_help_window = !self.show_help_window;
            }
            ui.separator();
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

        let label_cmds = self.config.editor.label_cmds.clone();

        if self.vertical_cursor.is_some() {
            ui.visuals_mut().selection.bg_fill = egui::Color32::TRANSPARENT;
            ui.visuals_mut().text_cursor.color = egui::Color32::TRANSPARENT;
        } else {
            ui.visuals_mut().selection.bg_fill = editor_selection_bg;
        }
        ui.visuals_mut().selection.stroke.color = egui::Color32::TRANSPARENT;

        let mut layouter = move |ui: &egui::Ui, string: &str, wrap_width: f32| {
            let mut layout_job = highlight_latex(string, font_size, &syntax_theme, &label_cmds);
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

        if let Some(vc) = &self.vertical_cursor {
            let cursor_color = if self.config.ui.dark_mode {
                parse_hex(&self.config.ui.dark_theme.ui.cursor)
            } else {
                parse_hex(&self.config.ui.light_theme.ui.cursor)
            };
            let time = ui.input(|i| i.time);
            let time_since_action = time - self.last_vc_action_time;
            let blink_on = if time_since_action < 0.5 {
                true
            } else {
                (time * 2.0).fract() < 0.5
            };

            let start_l = vc.anchor_line.min(vc.active_line);
            let end_l = vc.anchor_line.max(vc.active_line);

            if blink_on {
                for line_idx in start_l..=end_l {
                    let line_str = self.editor_text.split('\n').nth(line_idx).unwrap_or("");
                    let actual_col = line_str.chars().count().min(vc.col);
                    let abs_index = self.line_col_to_char_index(line_idx, actual_col);
                    let ccursor = egui::text::CCursor::new(abs_index);
                    let cursor_pos = galley.pos_from_ccursor(ccursor);
                    let rect = cursor_pos.translate(output.galley_pos.to_vec2());
                    painter
                        .line_segment([rect.min, rect.max], egui::Stroke::new(2.0, cursor_color));
                }
            }

            if self.scroll_to_vc {
                let active_line_str = self
                    .editor_text
                    .split('\n')
                    .nth(vc.active_line)
                    .unwrap_or("");
                let active_actual_col = active_line_str.chars().count().min(vc.col);
                let active_abs_idx = self.line_col_to_char_index(vc.active_line, active_actual_col);
                let ccursor = egui::text::CCursor::new(active_abs_idx);
                let cursor_pos = galley.pos_from_ccursor(ccursor);
                let rect = cursor_pos.translate(output.galley_pos.to_vec2());
                ui.scroll_to_rect(rect, None);
                self.scroll_to_vc = false;
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

    pub fn render_bracket_matches(
        &mut self,
        ui: &mut egui::Ui,
        output: &egui::text_edit::TextEditOutput,
        editor_id: egui::Id,
    ) {
        if let Some(state) = egui::TextEdit::load_state(ui.ctx(), editor_id) {
            if let Some(range) = state.cursor.char_range() {
                if range.primary.index == range.secondary.index {
                    if let Some((m1, m2)) =
                        find_matching_brackets(&self.editor_text, range.primary.index)
                    {
                        let syntax_theme = if self.config.ui.dark_mode {
                            &self.config.ui.dark_theme.syntax
                        } else {
                            &self.config.ui.light_theme.syntax
                        };
                        let bracket_color = parse_hex(&syntax_theme.bracket);

                        let start1 = output.galley.pos_from_ccursor(egui::text::CCursor::new(m1));
                        let end1 = output
                            .galley
                            .pos_from_ccursor(egui::text::CCursor::new(m1 + 1));
                        let start2 = output.galley.pos_from_ccursor(egui::text::CCursor::new(m2));
                        let end2 = output
                            .galley
                            .pos_from_ccursor(egui::text::CCursor::new(m2 + 1));

                        let get_rect = |start: egui::Rect, end: egui::Rect| {
                            let mut width = end.min.x - start.min.x;
                            if end.min.y > start.min.y || width <= 0.0 {
                                width = (start.max.y - start.min.y) * 0.6;
                            }
                            egui::Rect::from_min_max(
                                output.galley_pos + start.min.to_vec2(),
                                output.galley_pos
                                    + egui::pos2(start.min.x + width, start.max.y).to_vec2(),
                            )
                            .expand(1.5)
                        };

                        let rect1 = get_rect(start1, end1);
                        let rect2 = get_rect(start2, end2);

                        let bg_color = egui::Color32::from_rgba_unmultiplied(
                            bracket_color.r(),
                            bracket_color.g(),
                            bracket_color.b(),
                            40,
                        );

                        ui.painter().rect(
                            rect1,
                            2.0,
                            bg_color,
                            egui::Stroke::new(1.0, bracket_color),
                        );
                        ui.painter().rect(
                            rect2,
                            2.0,
                            bg_color,
                            egui::Stroke::new(1.0, bracket_color),
                        );
                    }
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

                    // Intercept text changes to trigger synchronous paired environment updating
                    if output.response.changed() || autocomplete_handled {
                        if let Some(cursor_range) = output.cursor_range {
                            let c_idx = cursor_range.primary.ccursor.index;
                            let byte_idx = self
                                .editor_text
                                .char_indices()
                                .nth(c_idx)
                                .map(|(i, _)| i)
                                .unwrap_or(self.editor_text.len());

                            if let Some((replace_range, new_name)) =
                                get_sync_env_edits(&self.editor_text, byte_idx)
                            {
                                let old_len = replace_range.len();
                                let new_len = new_name.len();
                                self.editor_text
                                    .replace_range(replace_range.clone(), &new_name);

                                // If the replacement shifted the string BEFORE the cursor, we must update the cursor location
                                if replace_range.end <= byte_idx {
                                    let diff = new_len as isize - old_len as isize;
                                    let new_byte_idx = (byte_idx as isize + diff).max(0) as usize;
                                    let new_char_pos =
                                        self.editor_text[..new_byte_idx].chars().count();

                                    if let Some(mut state) =
                                        egui::TextEdit::load_state(ui.ctx(), editor_id)
                                    {
                                        let ccursor = egui::text::CCursor::new(new_char_pos);
                                        state.cursor.set_char_range(Some(
                                            egui::text::CCursorRange::one(ccursor),
                                        ));
                                        egui::TextEdit::store_state(ui.ctx(), editor_id, state);
                                    }
                                }
                            }
                        }
                    }

                    self.render_highlight_matches(ui, &output);
                    self.render_bracket_matches(ui, &output, editor_id);
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
