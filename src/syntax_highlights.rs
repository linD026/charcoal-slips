use eframe::egui;

// ==========================================
// SYNTAX HIGHLIGHTERS
// ==========================================

pub fn highlight_latex(text: &str, font_size: f32, is_dark: bool) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let font = egui::FontId::monospace(font_size);
    let (c_norm, c_cmd, c_comment, c_bracket, c_math) = if is_dark {
        (
            egui::Color32::LIGHT_GRAY,
            egui::Color32::from_rgb(86, 182, 194),
            egui::Color32::from_rgb(152, 195, 121),
            egui::Color32::from_rgb(229, 192, 123),
            egui::Color32::from_rgb(198, 120, 221),
        )
    } else {
        (
            //egui::Color32::BLACK,
            //egui::Color32::from_rgb(9, 105, 218),
            //egui::Color32::from_rgb(17, 99, 41),
            //egui::Color32::from_rgb(215, 58, 73),
            //egui::Color32::from_rgb(111, 66, 193),
            egui::Color32::from_rgb(36, 41, 46), // Near Black (Normal text)
            egui::Color32::from_rgb(0, 92, 197), // Deep Blue (Commands)
            egui::Color32::from_rgb(106, 115, 125), // Muted Gray-Green (Comments)
            egui::Color32::from_rgb(215, 58, 73), // Crimson Red (Brackets)
            egui::Color32::from_rgb(111, 66, 193), // Royal Purple (Math mode)
        )
    };

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

pub fn highlight_logs(text: &str, font_size: f32, is_dark: bool) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let font = egui::FontId::monospace(font_size);
    for line in text.lines() {
        let color = if line.contains("[ERROR]") || line.contains("[STDERR]") || line.contains("❌")
        {
            if is_dark {
                egui::Color32::from_rgb(224, 108, 117)
            } else {
                egui::Color32::DARK_RED
            }
        } else if line.contains("[SUCCESS]") || line.contains("[FILE]") || line.contains("✅") {
            if is_dark {
                egui::Color32::from_rgb(152, 195, 121)
            } else {
                egui::Color32::from_rgb(17, 99, 41)
            }
        } else if line.contains("[BUILD]") || line.contains("[SYSTEM]") {
            if is_dark {
                egui::Color32::from_rgb(97, 175, 239)
            } else {
                egui::Color32::from_rgb(9, 105, 218)
            }
        } else if line.contains("[AI]") {
            if is_dark {
                egui::Color32::from_rgb(198, 120, 221)
            } else {
                egui::Color32::from_rgb(139, 0, 139)
            }
        } else {
            if is_dark {
                egui::Color32::LIGHT_GRAY
            } else {
                egui::Color32::DARK_GRAY
            }
        };
        job.append(line, 0.0, egui::TextFormat::simple(font.clone(), color));
        job.append("\n", 0.0, egui::TextFormat::simple(font.clone(), color));
    }
    job
}
