use crate::CCslipsApp;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

// Tracks the active file/dir operation modal
#[derive(Clone, PartialEq, Debug)]
pub enum FileOperation {
    None,
    CreateFile(PathBuf),
    CreateDir(PathBuf),
    // tracks multiple selections
    Delete(HashSet<PathBuf>),
}

impl CCslipsApp {
    // ==========================================
    // FILE CREATION & DELETION SYSTEM
    // ==========================================
    pub fn execute_file_operation(&mut self) {
        // Take ownership of the operation so we don't borrow `self` during the loop
        let op = std::mem::replace(&mut self.active_file_op, FileOperation::None);

        match op {
            FileOperation::CreateFile(base_path) => {
                if !self.file_op_input.is_empty() {
                    let target = base_path.join(&self.file_op_input);
                    match fs::File::create(&target) {
                        Ok(_) => {
                            self.append_log(&format!(
                                "[FILE] 📄 Created file: {}",
                                target.display()
                            ));
                            self.open_file(target, false); // Auto-open the new file
                        }
                        Err(e) => {
                            self.append_log(&format!("[ERROR] ❌ Failed to create file: {}", e))
                        }
                    }
                }
            }
            FileOperation::CreateDir(base_path) => {
                if !self.file_op_input.is_empty() {
                    let target = base_path.join(&self.file_op_input);
                    match fs::create_dir_all(&target) {
                        Ok(_) => self.append_log(&format!(
                            "[FILE] 📁 Created directory: {}",
                            target.display()
                        )),
                        Err(e) => self
                            .append_log(&format!("[ERROR] ❌ Failed to create directory: {}", e)),
                    }
                }
            }
            FileOperation::Delete(paths) => {
                for path in paths {
                    if !path.exists() {
                        continue;
                    }

                    if let Some(current) = &self.current_file {
                        if current.starts_with(&path) {
                            self.editor_text.clear();
                            self.current_file = None;
                            self.config.editor.last_opened_file = None;
                            self.save_config();
                        }
                    }

                    if path.is_dir() {
                        match fs::remove_dir_all(&path) {
                            Ok(_) => self.append_log(&format!(
                                "[FILE] 🗑 Deleted directory: {}",
                                path.display()
                            )),
                            Err(e) => self.append_log(&format!(
                                "[ERROR] ❌ Failed to delete directory: {}",
                                e
                            )),
                        }
                    } else {
                        match fs::remove_file(&path) {
                            Ok(_) => self
                                .append_log(&format!("[FILE] 🗑 Deleted file: {}", path.display())),
                            Err(e) => {
                                self.append_log(&format!("[ERROR] ❌ Failed to delete file: {}", e))
                            }
                        }
                    }
                }
            }
            FileOperation::None => {}
        }

        self.file_op_input.clear();
    }
}
