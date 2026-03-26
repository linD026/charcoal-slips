use crate::config::AiConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct IndexEntry {
    pub file_path: PathBuf,
    pub start_idx: usize,
    pub end_idx: usize,
    pub selected_text: String,
    pub ai_summary: String,
    pub timestamp: chrono::DateTime<chrono::Local>,
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    system: String,
    think: bool,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

pub fn trigger_ai_indexing(
    config: AiConfig,
    path: PathBuf,
    selected_str: String,
    start_idx: usize,
    end_idx: usize,
    tx: Sender<IndexEntry>,
) {
    // We spawn a background thread so the UI doesn't freeze while the AI thinks
    thread::spawn(move || {
        let client = match reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(IndexEntry {
                    file_path: path.clone(),
                    start_idx,
                    end_idx,
                    selected_text: selected_str.clone(),
                    ai_summary: format!("Error: Failed to build HTTP client: {}", e),
                    timestamp: chrono::Local::now(),
                });
                return;
            }
        };

        let req_body = OllamaRequest {
            model: config.model.clone(),
            prompt: selected_str.clone(),
            system: config.system_prompt.clone(),
            think: config.think.clone(),
            stream: false,
        };

        // Execute the request using our patient client
        let response = client.post(&config.url).json(&req_body).send();

        match response {
            Ok(res) => {
                if res.status().is_success() {
                    if let Ok(json) = res.json::<OllamaResponse>() {
                        let _ = tx.send(IndexEntry {
                            file_path: path,
                            start_idx,
                            end_idx,
                            selected_text: selected_str,
                            ai_summary: json.response,
                            timestamp: chrono::Local::now(),
                        });
                    } else {
                        let _ = tx.send(IndexEntry {
                            file_path: path,
                            start_idx,
                            end_idx,
                            selected_text: selected_str,
                            ai_summary: "Error: Failed to parse JSON response from Ollama".into(),
                            timestamp: chrono::Local::now(),
                        });
                    }
                } else {
                    // --- ENHANCED DEBUGGING: Capture exact HTTP failures ---
                    let status = res.status();
                    let error_text = res.text().unwrap_or_default();
                    let _ = tx.send(IndexEntry {
                        file_path: path,
                        start_idx,
                        end_idx,
                        selected_text: selected_str,
                        ai_summary: format!(
                            "Error: Ollama returned HTTP {} - {}",
                            status, error_text
                        ),
                        timestamp: chrono::Local::now(),
                    });
                }
            }
            Err(e) => {
                // Granular Network Error Breakdown
                let mut debug_info = format!("Network connection failed: {}", e);

                if e.is_timeout() {
                    debug_info = format!(
                        "Timeout: The {} model took longer than 300 seconds to respond. It may be too large for your hardware. Details: {}",
                        config.model, e
                    );
                } else if e.is_connect() {
                    debug_info = format!(
                        "Connection Refused: Is Ollama currently running on your machine? Details: {}",
                        e
                    );
                }

                let _ = tx.send(IndexEntry {
                    file_path: path,
                    start_idx,
                    end_idx,
                    selected_text: selected_str,
                    ai_summary: format!("Error: {}", debug_info),
                    timestamp: chrono::Local::now(),
                });
            }
        }
    });
}
