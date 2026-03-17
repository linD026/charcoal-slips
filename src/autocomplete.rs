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
                        let mut keys = Vec::new();
                        if let Ok(content) = fs::read_to_string(&path) {
                            for line in content.lines() {
                                let line = line.trim();
                                if line.starts_with('@')
                                    && line.contains('{')
                                    && line.ends_with(',')
                                {
                                    if let Some(start) = line.find('{') {
                                        keys.push(line[start + 1..line.len() - 1].to_string());
                                    }
                                }
                            }
                        }
                        self.files.insert(path.clone(), (modified, keys));
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
    // Shared structure for Bib and Label scanning
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
        // Regex to find \label{anything_here}
        let re = Regex::new(r"\\label\{([^}]+)\}").unwrap();

        // Recursively walk workspace for .tex files
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
    if let Some(brace_idx) = text_up_to_cursor.rfind('{') {
        let text_after_brace = &text_up_to_cursor[brace_idx..];
        if !text_after_brace.contains('}') {
            let before_brace = &text_up_to_cursor[..brace_idx];
            let cmd_search_area = if let Some(bracket_start) = before_brace.rfind('[') {
                &before_brace[..bracket_start]
            } else {
                before_brace
            };

            let search_term = text_up_to_cursor[brace_idx + 1..].to_string();

            // Check for Citation
            if cmd_search_area.ends_with("\\cite") {
                return AutocompleteContext::Citation(search_term);
            }
            // Check for Labels (Standard, Cleveref, Autoref, Nameref)
            else if cmd_search_area.ends_with("\\ref")
                || cmd_search_area.ends_with("\\cref")
                || cmd_search_area.ends_with("\\autoref")
                || cmd_search_area.ends_with("\\nameref")
            {
                return AutocompleteContext::Label(search_term);
            }
            // Check for Files
            else if cmd_search_area.ends_with("\\includegraphics")
                || cmd_search_area.ends_with("\\input")
            {
                return AutocompleteContext::File(search_term);
            }
        }
    }

    if let Some(idx) = text_up_to_cursor.rfind('\\') {
        let slice = &text_up_to_cursor[idx..];
        if !slice.contains(|c: char| c.is_whitespace() || c == '{' || c == '}') {
            return AutocompleteContext::Macro(slice.to_string());
        }
    }

    AutocompleteContext::None
}
