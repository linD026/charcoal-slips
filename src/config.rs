use eframe::egui;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

// --- Hex Parsing Utility ---
pub fn parse_hex(hex: &str) -> egui::Color32 {
    let hex = hex.trim_start_matches('#');
    let safe_default = egui::Color32::from_rgb(255, 0, 255); // Magenta fallback for errors

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
            return egui::Color32::from_rgba_unmultiplied(r, g, b, a);
        }
    }

    eprintln!(
        "[THEME ERROR] Invalid hex color code: '{}'. Falling back to safe default.",
        hex
    );
    safe_default
}

// --- Theme Structures ---
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UiTheme {
    pub bg_color: String,
    pub ui_selection_bg: String,
    pub ui_selection_text: String,
    pub editor_selection_bg: String,
    pub cursor: String,
    pub gutter_text: String,
    pub ai_button_bg: String,
    pub ai_button_text: String,
    pub popup_bg: String,
    pub popup_selected_text: String,
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
    pub last_opened_file: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UiConfig {
    pub left_panel_width: f32,
    pub right_panel_width: f32,
    pub dark_mode: bool,
    pub light_theme: ThemeConfig,
    pub dark_theme: ThemeConfig,
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
        let brace_cmds = [
            // default
            "\\documentclass",
            "\\author",
            "\\bibliography",
            "\\caption",
            "\\chapter",
            "\\date",
            "\\label",
            "\\paragraph",
            "\\subparagraph",
            "\\section",
            "\\subsection",
            "\\subsubsection",
            "\\title",
            "\\usepackage",
            "\\ref",
            "\\cref",
            "\\autoref",
            "\\nameref",
            "\\bibliographystyle",
            "\\bibliography",
            // custom
            "\\para",
            "\\TODO",
            "\\modified",
        ];
        default_autocomplete.extend(brace_cmds.iter().map(|&cmd| AutocompleteEntry {
            trigger: cmd.into(),
            insert: format!("{}{{$CURSOR$}}", cmd),
        }));
        let no_brace_cmds = [
            // default
            "\\tiny",
            "\\scriptsize",
            "\\footnotesize",
            "\\small",
            "\\normalsize",
            "\\large",
            "\\Large",
            "\\LARGE",
            "\\huge",
            "\\Huge",
            "\\centering",
            "\\clearpage",
            "\\end",
            "\\item",
            "\\maketitle",
            "\\newline",
            "\\noindent",
            "\\tableofcontents",
            // custom
            "\\sys",
            "\\kernel",
        ];
        default_autocomplete.extend(no_brace_cmds.iter().map(|&cmd| AutocompleteEntry {
            trigger: cmd.into(),
            insert: cmd.into(),
        }));
        let formatting = [
            ("\\texttt", "texttt"),
            ("\\textrm", "textrm"),
            ("\\textsf", "textsf"),
            ("\\textmd", "textmd"),
            ("\\textbf", "textbf"),
            ("\\textup", "textup"),
            ("\\textit", "textit"),
            ("\\textsl", "textsl"),
            ("\\textsc", "textsc"),
            ("\\underline", "underline"),
            ("\\emph", "emph"),
        ];
        default_autocomplete.extend(formatting.iter().map(|&(t, _)| AutocompleteEntry {
            trigger: t.into(),
            insert: format!("{}{{$CURSOR$}}", t),
        }));

        let bib_cmds = [
            // Standard
            "\\cite",
            "\\nocite",
            // Natbib variations
            "\\citep",
            "\\citet",
            "\\citealt",
            "\\citealp",
            "\\citeauthor",
            "\\citeyear",
            // BibLaTeX variations
            "\\footcite",
            "\\textcite",
            "\\parencite",
        ];

        default_autocomplete.extend(bib_cmds.iter().map(|&cmd| AutocompleteEntry {
            trigger: cmd.into(),
            insert: format!("{}{{$CURSOR$}}", cmd),
        }));

        let math = [
            // Greek Letters (Lowercase)
            ("\\alpha", "\\alpha"),
            ("\\beta", "\\beta"),
            ("\\gamma", "\\gamma"),
            ("\\delta", "\\delta"),
            ("\\epsilon", "\\epsilon"),
            ("\\zeta", "\\zeta"),
            ("\\eta", "\\eta"),
            ("\\theta", "\\theta"),
            ("\\iota", "\\iota"),
            ("\\kappa", "\\kappa"),
            ("\\lambda", "\\lambda"),
            ("\\mu", "\\mu"),
            ("\\nu", "\\nu"),
            ("\\xi", "\\xi"),
            ("\\pi", "\\pi"),
            ("\\rho", "\\rho"),
            ("\\sigma", "\\sigma"),
            ("\\tau", "\\tau"),
            ("\\upsilon", "\\upsilon"),
            ("\\phi", "\\phi"),
            ("\\chi", "\\chi"),
            ("\\psi", "\\psi"),
            ("\\omega", "\\omega"),
            // Greek Letters (Uppercase)
            ("\\Gamma", "\\Gamma"),
            ("\\Delta", "\\Delta"),
            ("\\Theta", "\\Theta"),
            ("\\Lambda", "\\Lambda"),
            ("\\Xi", "\\Xi"),
            ("\\Pi", "\\Pi"),
            ("\\Sigma", "\\Sigma"),
            ("\\Upsilon", "\\Upsilon"),
            ("\\Phi", "\\Phi"),
            ("\\Psi", "\\Psi"),
            ("\\Omega", "\\Omega"),
            // Math Operations & Symbols
            ("\\times", "\\times"),
            ("\\div", "\\div"),
            ("\\pm", "\\pm"),
            ("\\mp", "\\mp"),
            ("\\cdot", "\\cdot"),
            ("\\circ", "\\circ"),
            ("\\infty", "\\infty"),
            ("\\approx", "\\approx"),
            ("\\neq", "\\neq"),
            ("\\leq", "\\leq"),
            ("\\geq", "\\geq"),
            ("\\equiv", "\\equiv"),
            ("\\sim", "\\sim"),
            ("\\simeq", "\\simeq"),
            // Arrows
            ("\\rightarrow", "\\rightarrow"),
            ("\\leftarrow", "\\leftarrow"),
            ("\\Rightarrow", "\\Rightarrow"),
            ("\\Leftarrow", "\\Leftarrow"),
            ("\\leftrightarrow", "\\leftrightarrow"),
            ("\\Leftrightarrow", "\\Leftrightarrow"),
            // Calculus & Sets
            ("\\int", "\\int_{$CURSOR$}^{}"),
            ("\\iint", "\\iint"),
            ("\\oint", "\\oint"),
            ("\\partial", "\\partial"),
            ("\\nabla", "\\nabla"),
            ("\\prod", "\\prod_{$CURSOR$}^{}"),
            ("\\in", "\\in"),
            ("\\notin", "\\notin"),
            ("\\subset", "\\subset"),
            ("\\cup", "\\cup"),
            ("\\cap", "\\cap"),
            ("\\emptyset", "\\emptyset"),
            // Functions
            ("\\sin", "\\sin"),
            ("\\cos", "\\cos"),
            ("\\tan", "\\tan"),
            ("\\ln", "\\ln"),
            ("\\log", "\\log"),
            ("\\exp", "\\exp"),
            // Matrices and Complex Environments
            (
                "\\[",
                "\\[\n    $CURSOR$\n\\]",
            ),
            (
                "\\begin{pmatrix}",
                "\\begin{pmatrix}\n    $CURSOR$\n\\end{pmatrix}",
            ),
            (
                "\\begin{bmatrix}",
                "\\begin{bmatrix}\n    $CURSOR$\n\\end{bmatrix}",
            ),
            (
                "\\begin{vmatrix}",
                "\\begin{vmatrix}\n    $CURSOR$\n\\end{vmatrix}",
            ),
            (
                "\\begin{cases}",
                "\\begin{cases}\n    $CURSOR$ & \\text{if } \\\\\n    & \\text{otherwise}\n\\end{cases}",
            ),
            (
                "\\begin{align}",
                "\\begin{align}\n    $CURSOR$\n\\end{align}",
            ),
            (
                "\\begin{align*}",
                "\\begin{align*}\n    $CURSOR$\n\\end{align*}",
            ),
        ];

        default_autocomplete.extend(math.iter().map(|&(t, i)| AutocompleteEntry {
            trigger: t.into(),
            insert: i.into(),
        }));

        let envs = [
            ("\\begin", "\\begin{}\n    $CURSOR$\n\\end{}"),
            (
                "\\begin{document}",
                "\\begin{document}\n    $CURSOR$\n\\end{document}",
            ),
            (
                "\\begin{algorithm}",
                "\\begin{algorithm}\n    $CURSOR$\n\\end{algorithm}",
            ),
            (
                "\\begin{itemize}",
                "\\begin{itemize}\n    \\item $CURSOR$\n\\end{itemize}",
            ),
            (
                "\\begin{enumerate}",
                "\\begin{enumerate}\n    \\item $CURSOR$\n\\end{enumerate}",
            ),
            (
                "\\begin{table}",
                "\\begin{table}[$CURSOR$]\n    \\centering\n    \\begin{tabular}{c|c}\n         &  \\\\\n         & \n    \\end{tabular}\n    \\caption{Caption}\n    \\label{tab:placeholder}\n\\end{table}",
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

        let bib_types = [(
            "@misc",
            "@misc{$CURSOR$,\n    author = {},\n    key = {},\n    title = {},\n    year = {},\n    note = {}\n}",
        )];
        default_autocomplete.extend(bib_types.iter().map(|&(t, i)| AutocompleteEntry {
            trigger: t.into(),
            insert: i.into(),
        }));

        let default_light_theme = ThemeConfig {
            ui: UiTheme {
                bg_color: "#F9F9F8".into(),
                ui_selection_bg: "#CCE3FF".into(), // Light blue for file tree
                ui_selection_text: "#000000".into(), // Black text for readability
                editor_selection_bg: "#003E8A88".into(), // Translucent for syntax highlighting
                cursor: "#000000".into(),
                gutter_text: "#8A8A8A".into(),
                ai_button_bg: "#28A745".into(),
                ai_button_text: "#FFFFFF".into(),
                popup_bg: "#F9F9F8".into(),
                popup_selected_text: "#005CC5".into(),
            },
            syntax: SyntaxTheme {
                normal: "#24292E".into(),
                command: "#005CC5".into(),
                comment: "#6A737D".into(),
                bracket: "#D73A49".into(),
                math: "#6F42C1".into(),
            },
            search: SearchTheme {
                match_bg: "#FFD70064".into(),
                current_match_bg: "#FF8C00B4".into(),
            },
            terminal: TerminalTheme {
                success: "#116329".into(),
                error: "#CB2431".into(),
                info: "#0366D6".into(),
                ai: "#8B008B".into(),
                text: "#586069".into(),
            },
        };

        let default_dark_theme = ThemeConfig {
            ui: UiTheme {
                bg_color: "#1E1E1E".into(),
                ui_selection_bg: "#37373D".into(), // Distinct gray/blue for file tree
                ui_selection_text: "#FFFFFF".into(), // White text for readability
                editor_selection_bg: "#264F7878".into(), // Translucent for syntax highlighting
                cursor: "#FFFFFF".into(),
                gutter_text: "#858585".into(),
                ai_button_bg: "#2EA043".into(),
                ai_button_text: "#FFFFFF".into(),
                popup_bg: "#303030".into(),
                popup_selected_text: "#56B6C2".into(),
            },
            syntax: SyntaxTheme {
                normal: "#D4D4D4".into(),
                command: "#56B6C2".into(),
                comment: "#98C379".into(),
                bracket: "#E5C07B".into(),
                math: "#C678DD".into(),
            },
            search: SearchTheme {
                match_bg: "#FFFF0032".into(),
                current_match_bg: "#FFA50096".into(),
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
                last_opened_file: None,
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
