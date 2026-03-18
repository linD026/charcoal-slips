use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir;

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
