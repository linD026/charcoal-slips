use crate::config::parse_hex;
use crate::syntax_highlights::highlight_logs;
use crate::{CCslipsApp, RightTab};
use eframe::egui;

// ==========================================
// INLINE DIFF ENGINE (LCS ALGORITHM)
// ==========================================
#[derive(Clone, PartialEq)]
enum DiffKind {
    Keep,
    Insert,
    Delete,
}

fn compute_diff(old: &str, new: &str) -> Vec<(DiffKind, String)> {
    fn tokenize(s: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut buf = String::new();
        for c in s.chars() {
            if c.is_alphanumeric() {
                buf.push(c);
            } else {
                if !buf.is_empty() {
                    tokens.push(buf.clone());
                    buf.clear();
                }
                tokens.push(c.to_string());
            }
        }
        if !buf.is_empty() {
            tokens.push(buf);
        }
        tokens
    }

    let old_tokens = tokenize(old);
    let new_tokens = tokenize(new);
    let mut dp = vec![vec![0; new_tokens.len() + 1]; old_tokens.len() + 1];

    for i in 1..=old_tokens.len() {
        for j in 1..=new_tokens.len() {
            if old_tokens[i - 1] == new_tokens[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    let mut i = old_tokens.len();
    let mut j = new_tokens.len();
    let mut result = Vec::new();

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old_tokens[i - 1] == new_tokens[j - 1] {
            result.push((DiffKind::Keep, old_tokens[i - 1].clone()));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            result.push((DiffKind::Insert, new_tokens[j - 1].clone()));
            j -= 1;
        } else if i > 0 && (j == 0 || dp[i][j - 1] < dp[i - 1][j]) {
            result.push((DiffKind::Delete, old_tokens[i - 1].clone()));
            i -= 1;
        }
    }
    result.reverse();

    let mut compacted: Vec<(DiffKind, String)> = Vec::new();
    for (kind, text) in result {
        if let Some((last_kind, last_text)) = compacted.last_mut() {
            if *last_kind == kind {
                last_text.push_str(&text);
                continue;
            }
        }
        compacted.push((kind, text));
    }
    compacted
}

impl CCslipsApp {
    pub fn render_ai_index_tab(&mut self, ui: &mut egui::Ui) {
        let mut trigger_jump = None;
        let (c_err, c_succ, c_info) = if self.config.ui.dark_mode {
            (
                parse_hex(&self.config.ui.dark_theme.terminal.error),
                parse_hex(&self.config.ui.dark_theme.terminal.success),
                parse_hex(&self.config.ui.dark_theme.terminal.info),
            )
        } else {
            (
                parse_hex(&self.config.ui.light_theme.terminal.error),
                parse_hex(&self.config.ui.light_theme.terminal.success),
                parse_hex(&self.config.ui.light_theme.terminal.info),
            )
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            for entry in &self.index_entries {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        if ui.button("⮐ Jump to Selection").clicked() {
                            trigger_jump =
                                Some((entry.file_path.clone(), entry.start_idx, entry.end_idx));
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(entry.timestamp.format("%H:%M:%S").to_string())
                                    .weak(),
                            );
                        });
                    });

                    ui.separator();

                    let preview = if entry.selected_text.len() > 80 {
                        format!("\"{}...\"", &entry.selected_text[..80])
                    } else {
                        format!("\"{}\"", entry.selected_text)
                    };
                    ui.label(egui::RichText::new("Original Text:").small().color(c_info));
                    ui.label(egui::RichText::new(preview).weak().italics());
                    ui.add_space(8.0);

                    let summary = &entry.ai_summary;
                    let split_marker = if summary.contains("**Improved Text:**") {
                        Some("**Improved Text:**")
                    } else if summary.contains("Improved Text:") {
                        Some("Improved Text:")
                    } else {
                        None
                    };

                    if let Some(marker) = split_marker {
                        let parts: Vec<&str> = summary.split(marker).collect();
                        let errors_part = parts[0].trim();
                        let improved_part = parts.get(1).unwrap_or(&"").trim();

                        let clean_errors = errors_part
                            .trim_start_matches("**Errors found:**")
                            .trim_start_matches("Errors found:")
                            .trim();

                        if !clean_errors.is_empty() {
                            ui.label(egui::RichText::new("Errors Found:").strong().color(c_err));
                            let mut err_idx = 1;
                            for line in clean_errors.lines() {
                                let line = line.trim();
                                if line.is_empty() {
                                    continue;
                                }

                                let cleaned_line =
                                    line.trim_start_matches('-').trim_start_matches('*').trim();
                                if let Some((title, context)) = cleaned_line.split_once(':') {
                                    let clean_title = title.replace("**", "").trim().to_string();
                                    let clean_context =
                                        context.replace("**", "").trim().to_string();

                                    ui.horizontal_wrapped(|ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        ui.label(
                                            egui::RichText::new(format!("{}.", err_idx))
                                                .strong()
                                                .color(c_err),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!("{}:", clean_title))
                                                .strong(),
                                        );
                                        ui.label(clean_context);
                                    });
                                } else {
                                    let clean_line = cleaned_line.replace("**", "");
                                    ui.horizontal_wrapped(|ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        ui.label(
                                            egui::RichText::new(format!("{}.", err_idx))
                                                .strong()
                                                .color(c_err),
                                        );
                                        ui.label(clean_line);
                                    });
                                }
                                err_idx += 1;
                            }
                            ui.add_space(8.0);
                        }

                        if !improved_part.is_empty() {
                            ui.label(
                                egui::RichText::new("Improved Text (Diff):")
                                    .strong()
                                    .color(c_succ),
                            );

                            let diffs = compute_diff(&entry.selected_text, improved_part);
                            let mut job = egui::text::LayoutJob::default();

                            let font_id = egui::FontId::proportional(self.config.editor.font_size);
                            let default_color = ui.visuals().text_color();

                            for (kind, text) in diffs {
                                let (color, bg_color, is_strike) = match kind {
                                    DiffKind::Keep => {
                                        (default_color, egui::Color32::TRANSPARENT, false)
                                    }
                                    DiffKind::Insert => (
                                        c_succ,
                                        egui::Color32::from_rgba_unmultiplied(
                                            c_succ.r(),
                                            c_succ.g(),
                                            c_succ.b(),
                                            40,
                                        ),
                                        false,
                                    ),
                                    DiffKind::Delete => (
                                        c_err,
                                        egui::Color32::from_rgba_unmultiplied(
                                            c_err.r(),
                                            c_err.g(),
                                            c_err.b(),
                                            40,
                                        ),
                                        true,
                                    ),
                                };

                                let mut format = egui::TextFormat::simple(font_id.clone(), color);
                                format.background = bg_color;
                                if is_strike {
                                    format.strikethrough = egui::Stroke::new(1.0, color);
                                }
                                job.append(&text, 0.0, format);
                            }

                            egui::Frame::none()
                                .fill(ui.visuals().faint_bg_color)
                                .inner_margin(6.0)
                                .show(ui, |ui| {
                                    ui.add(egui::Label::new(job).wrap(true));
                                });
                            ui.add_space(4.0);

                            if ui.button("📋 Copy Improved Text").clicked() {
                                ui.output_mut(|o| o.copied_text = improved_part.to_string());
                            }
                        }
                    } else {
                        ui.label(egui::RichText::new("AI Response:").strong().color(c_info));
                        ui.label(summary);
                    }
                });
            }
        });

        if let Some((path, start, end)) = trigger_jump {
            self.open_file(path, true);
            self.jump_request = Some((start, end));
        }
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
                        self.render_ai_index_tab(ui);
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
}
