use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

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
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LocalleafConfig {
    pub ai: AiConfig,
    pub build: BuildConfig,
    pub editor: EditorConfig,
    pub ui: UiConfig,
}

impl Default for LocalleafConfig {
    fn default() -> Self {
        let current_path = env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .to_string_lossy()
            .to_string();

        let mut default_autocomplete = Vec::new();
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

        Self {
            ai: AiConfig {
                url: "http://localhost:11434/api/generate".into(),
                model: "qwen3:0.6b".into(),
                system_prompt: "Identify all grammar and vocabulary errors in the provided text.\n\nRules:\n1. Output plain text only. Do not use markdown formatting like asterisks or hash symbols.\n2. Zero conversational text. Do not include greetings, explanations, or conclusions.\n3. You must strictly follow the exact output template below. Use a hyphen for each error and three hyphens to separate the errors from the corrected text.\n\nTemplate:\n- [Error Type]: [The exact mistake from the text]\n- [Error Type]: [The exact mistake from the text]\n---\n[Correct context]\n\nExample Input:\nHe do not likes the much big apples.\n\nExample Output:\n- grammar: do not likes\n- vocabulary: much big\n---\nHe does not like the very big apples.".into(),
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
            },
        }
    }
}
