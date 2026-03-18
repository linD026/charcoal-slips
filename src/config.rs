use eframe::egui;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

// --- NEW: Hex Parsing Utility ---
pub fn parse_hex(hex: &str) -> egui::Color32 {
    let hex = hex.trim_start_matches('#');
    let safe_default = egui::Color32::from_rgb(255, 0, 255); // Hot Pink fallback for errors

    if hex.len() == 6 {
        if let Ok(rgb) = u32::from_str_radix(hex, 16) {
            let r = ((rgb >> 16) & 0xFF) as u8;
            let g = ((rgb >> 8) & 0xFF) as u8;
            let b = (rgb & 0xFF) as u8;
            return egui::Color32::from_rgb(r, g, b);
        }
    } else if hex.len() == 8 {
        if let Ok(rgba) = u32::from_str_radix(hex, 16) {
            let r = ((rgba >> 24) & 0xFF) as u8;
            let g = ((rgba >> 16) & 0xFF) as u8;
            let b = ((rgba >> 8) & 0xFF) as u8;
            let a = (rgba & 0xFF) as u8;
            // Crucial: Use unmultiplied so it acts like a physical highlighter
            return egui::Color32::from_rgba_unmultiplied(r, g, b, a);
        }
    }

    eprintln!(
        "[THEME ERROR] Invalid hex color code: '{}'. Falling back to safe default.",
        hex
    );
    safe_default
}

// --- NEW: Theme Structures ---
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UiTheme {
    pub bg_color: String, // Covers both panel and editor for seamless look
    pub selection_bg: String,
    pub cursor: String,
    pub ai_button_bg: String,   // Solid button background
    pub ai_button_text: String, // Solid button text
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SyntaxTheme {
    pub normal: String,
    pub command: String,
    pub comment: String,
    pub bracket: String,
    pub math: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SearchTheme {
    pub match_bg: String,
    pub current_match_bg: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TerminalTheme {
    pub success: String,
    pub error: String,
    pub info: String,
    pub ai: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ThemeConfig {
    pub ui: UiTheme,
    pub syntax: SyntaxTheme,
    pub search: SearchTheme,
    pub terminal: TerminalTheme,
}

// --- Existing Configs ---
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AiConfig {
    pub url: String,
    pub model: String,
    pub system_prompt: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BuildConfig {
    pub command: String,
    pub working_directory: String,
    pub auto_save_before_build: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AutocompleteEntry {
    pub trigger: String,
    pub insert: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EditorConfig {
    pub font_size: f32,
    pub autocomplete_cmds: Vec<AutocompleteEntry>,
    pub bib_dir: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UiConfig {
    pub left_panel_width: f32,
    pub right_panel_width: f32,
    pub dark_mode: bool,
    pub light_theme: ThemeConfig, // NEW
    pub dark_theme: ThemeConfig,  // NEW
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CCslipsConfig {
    pub ai: AiConfig,
    pub build: BuildConfig,
    pub editor: EditorConfig,
    pub ui: UiConfig,
}

impl Default for CCslipsConfig {
    fn default() -> Self {
        let current_path = env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .to_string_lossy()
            .to_string();

        let mut default_autocomplete = Vec::new();
        // ... [KEEP ALL YOUR EXISTING AUTOCOMPLETE SETUP EXACTLY THE SAME] ...
        let brace_cmds = [
            "\\author",
            "\\bibliography",
            "\\caption",
            "\\chapter",
            "\\cite",
            "\\date",
            "\\label",
            "\\paragraph",
            "\\section",
            "\\subsection",
            "\\subsubsection",
            "\\title",
            "\\usepackage",
            "\\ref",
            "\\cref",
            "\\autoref",
            "\\nameref",
        ];
        default_autocomplete.extend(brace_cmds.iter().map(|&cmd| AutocompleteEntry {
            trigger: cmd.into(),
            insert: format!("{}{{$CURSOR$}}", cmd),
        }));
        let no_brace_cmds = [
            "\\centering",
            "\\clearpage",
            "\\end",
            "\\item",
            "\\maketitle",
            "\\newline",
            "\\noindent",
            "\\tableofcontents",
            "\\alpha",
            "\\beta",
            "\\gamma",
            "\\lambda",
            "\\infty",
        ];
        default_autocomplete.extend(no_brace_cmds.iter().map(|&cmd| AutocompleteEntry {
            trigger: cmd.into(),
            insert: cmd.into(),
        }));
        let formatting = [
            ("\\textbf", "textbf"),
            ("\\textit", "textit"),
            ("\\underline", "underline"),
            ("\\texttt", "texttt"),
            ("\\emph", "emph"),
        ];
        default_autocomplete.extend(formatting.iter().map(|&(t, _)| AutocompleteEntry {
            trigger: t.into(),
            insert: format!("{}{{$CURSOR$}}", t),
        }));

        default_autocomplete.push(AutocompleteEntry {
            trigger: "\\frac".into(),
            insert: "\\frac{$CURSOR$}{}".into(),
        });
        default_autocomplete.push(AutocompleteEntry {
            trigger: "\\sum".into(),
            insert: "\\sum_{$CURSOR$}^{}".into(),
        });

        let envs = [
            (
                "\\begin{document}",
                "\\begin{document}\n    $CURSOR$\n\\end{document}",
            ),
            (
                "\\begin{equation}",
                "\\begin{equation}\n    $CURSOR$\n\\end{equation}",
            ),
            (
                "\\begin{itemize}",
                "\\begin{itemize}\n    \\item $CURSOR$\n\\end{itemize}",
            ),
            (
                "\\begin{figure}",
                "\\begin{figure}[htpb]\n    \\centering\n    \\includegraphics[width=0.5\\linewidth]{$CURSOR$}\n    \\caption{Caption}\n    \\label{fig:placeholder}\n\\end{figure}",
            ),
        ];
        default_autocomplete.extend(envs.iter().map(|&(t, i)| AutocompleteEntry {
            trigger: t.into(),
            insert: i.into(),
        }));

        // --- NEW: Default Light Theme (Overleaf / Apple Style) ---
        let default_light_theme = ThemeConfig {
            ui: UiTheme {
                bg_color: "#F9F9F8".into(),       // Seamless warm white
                selection_bg: "#A8CEFF78".into(), // Translucent Apple Blue
                cursor: "#000000".into(),
                ai_button_bg: "#28A745".into(), // Solid GitHub Green
                ai_button_text: "#FFFFFF".into(),
            },
            syntax: SyntaxTheme {
                normal: "#24292E".into(),
                command: "#005CC5".into(),
                comment: "#6A737D".into(),
                bracket: "#D73A49".into(),
                math: "#6F42C1".into(),
            },
            search: SearchTheme {
                match_bg: "#FFD70064".into(),         // Translucent Yellow
                current_match_bg: "#FF8C00B4".into(), // Translucent Dark Orange
            },
            terminal: TerminalTheme {
                success: "#116329".into(),
                error: "#CB2431".into(),
                info: "#0366D6".into(),
                ai: "#8B008B".into(),
                text: "#586069".into(),
            },
        };

        // --- NEW: Default Dark Theme (VS Code Style) ---
        let default_dark_theme = ThemeConfig {
            ui: UiTheme {
                bg_color: "#1E1E1E".into(),     // Seamless Dark Gray
                selection_bg: "#264F78".into(), // VS Code dark selection
                cursor: "#FFFFFF".into(),
                ai_button_bg: "#2EA043".into(),
                ai_button_text: "#FFFFFF".into(),
            },
            syntax: SyntaxTheme {
                normal: "#D4D4D4".into(),
                command: "#56B6C2".into(),
                comment: "#98C379".into(),
                bracket: "#E5C07B".into(),
                math: "#C678DD".into(),
            },
            search: SearchTheme {
                match_bg: "#FFFF0032".into(),         // Faint Yellow
                current_match_bg: "#FFA50096".into(), // Orange
            },
            terminal: TerminalTheme {
                success: "#98C379".into(),
                error: "#E06C75".into(),
                info: "#61AFEF".into(),
                ai: "#C678DD".into(),
                text: "#D3D3D3".into(),
            },
        };

        Self {
            ai: AiConfig {
                url: "http://localhost:11434/api/generate".into(),
                model: "qwen3:0.6b".into(),
                system_prompt: "Act as an expert linguistic editor. Your sole objective is to identify and categorize every linguistic error within the provided text, including grammar, vocabulary, spelling, punctuation, and syntax.\n\n**Strict Operational Rules:**\n1. **No Markdown Formatting:** Do not use bolding, italics, headers, asterisks, or any other Markdown syntax. The output must be 100% raw, plain text.\n2. **Zero Conversational Output:** Do not include greetings, introductions, transitional phrases, or concluding remarks. Begin the output immediately with the error list.\n3. **Error Categorization:** Categorize each error based on its specific type. Use a single hyphen to start each line, followed by the category, a colon, and the verbatim erroneous segment from the input.\n4. **Structural Delimiter:** Use a line consisting of exactly three hyphens (---) to separate the error list from the corrected text.\n5. **Final Correction:** Following the delimiter, provide the complete, fully corrected version of the input text. Ensure it is grammatically perfect and stylistically natural while retaining the original intent.\n\n**Output Template Structure:**\n- [Category]: [Verbatim Error]\n---\n[Full Corrected Text]".into(),
            },
            build: BuildConfig {
                command: "make".into(),
                working_directory: current_path,
                auto_save_before_build: true,
            },
            editor: EditorConfig {
                font_size: 12.0,
                autocomplete_cmds: default_autocomplete,
                bib_dir: "bib/".into(),
            },
            ui: UiConfig {
                left_panel_width: 200.0,
                right_panel_width: 320.0,
                dark_mode: true,
                light_theme: default_light_theme,
                dark_theme: default_dark_theme,
            },
        }
    }
}
