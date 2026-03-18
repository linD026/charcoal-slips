use crate::config::{SyntaxTheme, TerminalTheme, parse_hex};
use eframe::egui;

// ==========================================
// SYNTAX HIGHLIGHTERS
// ==========================================

pub fn highlight_latex(text: &str, font_size: f32, theme: &SyntaxTheme) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let font = egui::FontId::monospace(font_size);

    // Parse dynamic colors from JSON Hex strings
    let c_norm = parse_hex(&theme.normal);
    let c_cmd = parse_hex(&theme.command);
    let c_comment = parse_hex(&theme.comment);
    let c_bracket = parse_hex(&theme.bracket);
    let c_math = parse_hex(&theme.math);

    let mut token = String::new();
    let mut state = 0;

    for c in text.chars() {
        if state == 2 {
            token.push(c);
            if c == '\n' {
                job.append(
                    &token,
                    0.0,
                    egui::TextFormat::simple(font.clone(), c_comment),
                );
                token.clear();
                state = 0;
            }
        } else if state == 1 {
            if c.is_alphabetic() {
                token.push(c);
            } else {
                job.append(&token, 0.0, egui::TextFormat::simple(font.clone(), c_cmd));
                token.clear();
                state = 0;
                if c == '%' {
                    state = 2;
                    token.push(c);
                } else if c == '\\' {
                    state = 1;
                    token.push(c);
                } else if c == '{' || c == '}' || c == '[' || c == ']' {
                    job.append(
                        &c.to_string(),
                        0.0,
                        egui::TextFormat::simple(font.clone(), c_bracket),
                    );
                } else if c == '$' {
                    job.append(
                        &c.to_string(),
                        0.0,
                        egui::TextFormat::simple(font.clone(), c_math),
                    );
                } else {
                    token.push(c);
                }
            }
        } else {
            if c == '%' || c == '\\' || c == '{' || c == '}' || c == '[' || c == ']' || c == '$' {
                if !token.is_empty() {
                    job.append(&token, 0.0, egui::TextFormat::simple(font.clone(), c_norm));
                    token.clear();
                }
                if c == '%' {
                    state = 2;
                    token.push(c);
                } else if c == '\\' {
                    state = 1;
                    token.push(c);
                } else if c == '{' || c == '}' || c == '[' || c == ']' {
                    job.append(
                        &c.to_string(),
                        0.0,
                        egui::TextFormat::simple(font.clone(), c_bracket),
                    );
                } else if c == '$' {
                    job.append(
                        &c.to_string(),
                        0.0,
                        egui::TextFormat::simple(font.clone(), c_math),
                    );
                }
            } else {
                token.push(c);
            }
        }
    }
    if !token.is_empty() {
        if state == 2 {
            job.append(
                &token,
                0.0,
                egui::TextFormat::simple(font.clone(), c_comment),
            );
        } else if state == 1 {
            job.append(&token, 0.0, egui::TextFormat::simple(font.clone(), c_cmd));
        } else {
            job.append(&token, 0.0, egui::TextFormat::simple(font.clone(), c_norm));
        }
    }
    job
}

pub fn highlight_logs(text: &str, font_size: f32, theme: &TerminalTheme) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let font = egui::FontId::monospace(font_size);

    // Parse dynamic colors
    let c_success = parse_hex(&theme.success);
    let c_error = parse_hex(&theme.error);
    let c_info = parse_hex(&theme.info);
    let c_ai = parse_hex(&theme.ai);
    let c_text = parse_hex(&theme.text);

    for line in text.lines() {
        let color = if line.contains("[ERROR]") || line.contains("[STDERR]") || line.contains("❌")
        {
            c_error
        } else if line.contains("[SUCCESS]") || line.contains("[FILE]") || line.contains("✅") {
            c_success
        } else if line.contains("[BUILD]") || line.contains("[SYSTEM]") {
            c_info
        } else if line.contains("[AI]") {
            c_ai
        } else {
            c_text
        };
        job.append(line, 0.0, egui::TextFormat::simple(font.clone(), color));
        job.append("\n", 0.0, egui::TextFormat::simple(font.clone(), color));
    }
    job
}
