use crate::config::{SyntaxTheme, TerminalTheme, parse_hex};
use eframe::egui;

// ==========================================
// SYNTAX HIGHLIGHTERS
// ==========================================

#[derive(Clone, Copy, PartialEq)]
enum LexerState {
    Normal,
    Backslash,         // Just saw '\', waiting to decide what's next
    Command,           // Reading alphabetic characters or '@'
    Comment,           // Locked until '\n'
    InlineMath,        // Locked until unescaped '$'
    DisplayMath,       // Locked until '$$' or '\]'
    VerbatimCmdWait,   // Saw \verb, waiting for the delimiter (e.g., '|')
    VerbatimCmd(char), // Locked until the saved delimiter is seen again
    VerbatimBlock,     // Locked until exactly \end{verbatim}, lstlisting, or minted
    EnvNameWait(bool), // Saw \begin or \end, waiting for '{'. bool = is_begin
    EnvName(bool),     // Inside \begin{...}. bool = is_begin
}

pub fn highlight_latex(text: &str, font_size: f32, theme: &SyntaxTheme) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let font = egui::FontId::monospace(font_size);

    let c_norm = parse_hex(&theme.normal);
    let c_cmd = parse_hex(&theme.command);
    let c_comment = parse_hex(&theme.comment);
    let c_bracket = parse_hex(&theme.bracket);
    let c_math = parse_hex(&theme.math);

    let mut state = LexerState::Normal;
    let mut token = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    // Helper closure to instantly dump the current token to the LayoutJob
    let flush = |job: &mut egui::text::LayoutJob, text: &mut String, color: egui::Color32| {
        if !text.is_empty() {
            job.append(text, 0.0, egui::TextFormat::simple(font.clone(), color));
            text.clear();
        }
    };

    while i < chars.len() {
        let c = chars[i];

        match state {
            LexerState::Normal => {
                if c == '%' {
                    flush(&mut job, &mut token, c_norm);
                    token.push(c);
                    state = LexerState::Comment;
                } else if c == '\\' {
                    flush(&mut job, &mut token, c_norm);
                    token.push(c);
                    state = LexerState::Backslash;
                } else if c == '$' {
                    flush(&mut job, &mut token, c_norm);
                    token.push(c);
                    if i + 1 < chars.len() && chars[i + 1] == '$' {
                        token.push(chars[i + 1]);
                        i += 1;
                        state = LexerState::DisplayMath;
                    } else {
                        state = LexerState::InlineMath;
                    }
                } else if c == '{' || c == '}' || c == '[' || c == ']' {
                    flush(&mut job, &mut token, c_norm);
                    token.push(c);
                    flush(&mut job, &mut token, c_bracket);
                } else {
                    token.push(c);
                }
                i += 1;
            }
            LexerState::Backslash => {
                if c.is_alphabetic() || c == '@' {
                    token.push(c);
                    state = LexerState::Command;
                    i += 1;
                } else if c == '[' {
                    // Start of Display Math: \[
                    token.push(c);
                    state = LexerState::DisplayMath;
                    i += 1;
                } else {
                    // Single character escape (e.g., \%, \\, \ )
                    token.push(c);
                    flush(&mut job, &mut token, c_cmd);
                    state = LexerState::Normal;
                    i += 1;
                }
            }
            LexerState::Command => {
                if c.is_alphabetic() || c == '@' {
                    token.push(c);
                    i += 1;
                } else if c == '*' {
                    // Starred command variants (e.g., \section*)
                    token.push(c);
                    i += 1;

                    let is_verb = token == "\\verb*";
                    flush(&mut job, &mut token, c_cmd);

                    if is_verb {
                        state = LexerState::VerbatimCmdWait;
                    } else {
                        state = LexerState::Normal;
                    }
                } else {
                    // End of command
                    let cmd = token.clone();
                    flush(&mut job, &mut token, c_cmd);

                    if cmd == "\\verb" {
                        state = LexerState::VerbatimCmdWait;
                    } else if cmd == "\\begin" {
                        state = LexerState::EnvNameWait(true);
                    } else if cmd == "\\end" {
                        state = LexerState::EnvNameWait(false);
                    } else {
                        state = LexerState::Normal;
                    }
                    // IMPORTANT: Do not increment 'i'.
                    // We must re-evaluate this character in the new state.
                }
            }
            LexerState::Comment => {
                token.push(c);
                if c == '\n' {
                    flush(&mut job, &mut token, c_comment);
                    state = LexerState::Normal;
                }
                i += 1;
            }
            LexerState::InlineMath => {
                token.push(c);
                // Exit on unescaped $
                if c == '$' && token.len() >= 2 && !token.ends_with("\\$") {
                    flush(&mut job, &mut token, c_math);
                    state = LexerState::Normal;
                }
                i += 1;
            }
            LexerState::DisplayMath => {
                token.push(c);
                // Exit on unescaped $$ or \]
                let is_dollar_end = c == '$' && token.ends_with("$$") && !token.ends_with("\\$$");
                let is_bracket_end =
                    c == ']' && token.ends_with("\\]") && !token.ends_with("\\\\]");

                if is_dollar_end || is_bracket_end {
                    flush(&mut job, &mut token, c_math);
                    state = LexerState::Normal;
                }
                i += 1;
            }
            LexerState::VerbatimCmdWait => {
                if c == '*' {
                    token.push(c);
                    flush(&mut job, &mut token, c_cmd);
                    i += 1;
                } else if c.is_whitespace() {
                    token.push(c);
                    flush(&mut job, &mut token, c_norm);
                    i += 1;
                } else {
                    state = LexerState::VerbatimCmd(c);
                    token.push(c);
                    flush(&mut job, &mut token, c_bracket);
                    i += 1;
                }
            }
            LexerState::VerbatimCmd(delim) => {
                token.push(c);
                if c == delim {
                    // Paint the verbatim content as math (code), and the delim as a bracket
                    let content = token[..token.len() - 1].to_string();
                    if !content.is_empty() {
                        job.append(
                            &content,
                            0.0,
                            egui::TextFormat::simple(font.clone(), c_math),
                        );
                    }
                    job.append(
                        &c.to_string(),
                        0.0,
                        egui::TextFormat::simple(font.clone(), c_bracket),
                    );
                    token.clear();
                    state = LexerState::Normal;
                }
                i += 1;
            }
            LexerState::EnvNameWait(is_begin) => {
                if c.is_whitespace() {
                    token.push(c);
                    flush(&mut job, &mut token, c_norm);
                    i += 1;
                } else if c == '{' {
                    token.push(c);
                    flush(&mut job, &mut token, c_bracket);
                    state = LexerState::EnvName(is_begin);
                    i += 1;
                } else {
                    state = LexerState::Normal;
                }
            }
            LexerState::EnvName(is_begin) => {
                if c == '}' {
                    let env_name = token.clone();

                    // Highlight the environment name aggressively
                    flush(&mut job, &mut token, c_cmd);
                    token.push(c);
                    flush(&mut job, &mut token, c_bracket);

                    if env_name == "verbatim" || env_name == "lstlisting" || env_name == "minted" {
                        if is_begin {
                            state = LexerState::VerbatimBlock;
                        } else {
                            state = LexerState::Normal;
                        }
                    } else {
                        state = LexerState::Normal;
                    }
                    i += 1;
                } else if c == '\\' || c == '{' || c == '\n' {
                    // Typo or invalid characters inside the environment name, abort
                    flush(&mut job, &mut token, c_norm);
                    state = LexerState::Normal;
                } else {
                    token.push(c);
                    i += 1;
                }
            }
            LexerState::VerbatimBlock => {
                token.push(c);
                let ends = ["\\end{verbatim}", "\\end{lstlisting}", "\\end{minted}"];
                let mut found_end = false;

                for e in ends {
                    if token.ends_with(e) {
                        // Flush the raw code block content
                        let content = token[..token.len() - e.len()].to_string();
                        if !content.is_empty() {
                            job.append(
                                &content,
                                0.0,
                                egui::TextFormat::simple(font.clone(), c_math),
                            );
                        }
                        token.clear();

                        // Manually construct and highlight the \end tag to keep it clean
                        job.append("\\end", 0.0, egui::TextFormat::simple(font.clone(), c_cmd));
                        job.append("{", 0.0, egui::TextFormat::simple(font.clone(), c_bracket));

                        let env_name = &e[5..e.len() - 1];
                        job.append(env_name, 0.0, egui::TextFormat::simple(font.clone(), c_cmd));
                        job.append("}", 0.0, egui::TextFormat::simple(font.clone(), c_bracket));

                        state = LexerState::Normal;
                        found_end = true;
                        break;
                    }
                }

                if !found_end {
                    i += 1;
                } else {
                    i += 1;
                }
            }
        }
    }

    // Ensure anything left in the buffer at EOF gets painted
    match state {
        LexerState::Normal | LexerState::EnvNameWait(_) => flush(&mut job, &mut token, c_norm),
        LexerState::Backslash | LexerState::Command | LexerState::EnvName(_) => {
            flush(&mut job, &mut token, c_cmd)
        }
        LexerState::Comment => flush(&mut job, &mut token, c_comment),
        LexerState::InlineMath
        | LexerState::DisplayMath
        | LexerState::VerbatimCmdWait
        | LexerState::VerbatimCmd(_)
        | LexerState::VerbatimBlock => flush(&mut job, &mut token, c_math),
    }

    job
}

pub fn highlight_logs(text: &str, font_size: f32, theme: &TerminalTheme) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let font = egui::FontId::monospace(font_size);

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
