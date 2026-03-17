use crate::config::AiConfig;
use chrono::{DateTime, Local};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::thread;

pub struct IndexEntry {
    pub file_path: PathBuf,
    pub selected_text: String,
    pub ai_summary: String,
    pub start_idx: usize,
    pub end_idx: usize,
    pub timestamp: DateTime<Local>, // NEW: Added timestamp
}

pub fn trigger_ai_indexing(
    ai_cfg: AiConfig,
    file_path: PathBuf,
    text: String,
    start: usize,
    end: usize,
    tx: Sender<IndexEntry>,
) {
    thread::spawn(move || {
        let client = reqwest::blocking::Client::new();
        let payload = serde_json::json!({
            "model": ai_cfg.model,
            "prompt": format!("{}\n\nText: {}", ai_cfg.system_prompt, text),
            "stream": false
        });

        let summary = match client.post(&ai_cfg.url).json(&payload).send() {
            Ok(res) => {
                if res.status().is_success() {
                    match res.json::<serde_json::Value>() {
                        Ok(json) => json["response"]
                            .as_str()
                            .unwrap_or("Error: Invalid JSON")
                            .to_string(),
                        Err(e) => format!("Error: Failed to parse JSON - {}", e),
                    }
                } else {
                    format!("Error: HTTP Status {}", res.status())
                }
            }
            Err(e) => format!("Error: Network connection failed - {}", e),
        };

        let _ = tx.send(IndexEntry {
            file_path,
            selected_text: text,
            ai_summary: summary.trim().to_string(),
            start_idx: start,
            end_idx: end,
            timestamp: Local::now(), // NEW: Capture current time
        });
    });
}
