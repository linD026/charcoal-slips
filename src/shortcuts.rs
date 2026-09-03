use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AppAction {
    // Global Actions
    SaveFile,
    BuildProject,
    CloseWindowOrFile,
    ZoomIn,
    ZoomOut,
    ToggleSearch,

    // Editor / Panel Specific Actions
    SendToAi,
    ToggleVerticalEdit,
    AbortOrClose,
}

#[derive(Clone)]
pub struct ShortcutDef {
    pub trigger: egui::KeyboardShortcut,
    pub secondary_trigger: Option<egui::KeyboardShortcut>,
    pub action: AppAction,
    pub help: &'static str,
}

impl ShortcutDef {
    pub fn consume(&self, ctx: &egui::Context) -> bool {
        ctx.input_mut(|i| {
            let primary = i.consume_shortcut(&self.trigger);
            let secondary = self
                .secondary_trigger
                .as_ref()
                .map_or(false, |st| i.consume_shortcut(st));
            primary || secondary
        })
    }

    /// Formats the shortcut neatly for UI display (e.g. "Ctrl/Cmd + Alt + V")
    pub fn display_string(&self) -> String {
        let mut s = String::new();
        if self.trigger.modifiers.command {
            s.push_str("Ctrl/Cmd + ");
        }
        if self.trigger.modifiers.alt {
            s.push_str("Alt/Opt + ");
        }
        if self.trigger.modifiers.shift {
            s.push_str("Shift + ");
        }
        s.push_str(self.trigger.logical_key.name());
        s
    }
}

pub struct ShortcutRegistry {
    pub global: Vec<ShortcutDef>,
    pub editor: Vec<ShortcutDef>,
}

impl Default for ShortcutRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ShortcutRegistry {
    pub fn new() -> Self {
        let cmd = egui::Modifiers::COMMAND; // Maps to Ctrl on Win/Linux, Cmd on Mac
        let alt = egui::Modifiers::ALT;
        let none = egui::Modifiers::NONE;

        Self {
            global: vec![
                ShortcutDef {
                    trigger: egui::KeyboardShortcut::new(cmd, egui::Key::S),
                    secondary_trigger: None,
                    action: AppAction::SaveFile,
                    help: "Save the current file",
                },
                ShortcutDef {
                    trigger: egui::KeyboardShortcut::new(cmd, egui::Key::B),
                    secondary_trigger: None,
                    action: AppAction::BuildProject,
                    help: "Execute the build pipeline",
                },
                ShortcutDef {
                    trigger: egui::KeyboardShortcut::new(cmd, egui::Key::W),
                    secondary_trigger: None,
                    action: AppAction::CloseWindowOrFile,
                    help: "Close current file, or exit app if none open",
                },
                ShortcutDef {
                    trigger: egui::KeyboardShortcut::new(cmd, egui::Key::Plus),
                    secondary_trigger: Some(egui::KeyboardShortcut::new(cmd, egui::Key::Equals)),
                    action: AppAction::ZoomIn,
                    help: "Increase editor font size",
                },
                ShortcutDef {
                    trigger: egui::KeyboardShortcut::new(cmd, egui::Key::Minus),
                    secondary_trigger: None,
                    action: AppAction::ZoomOut,
                    help: "Decrease editor font size",
                },
                ShortcutDef {
                    trigger: egui::KeyboardShortcut::new(cmd, egui::Key::F),
                    secondary_trigger: None,
                    action: AppAction::ToggleSearch,
                    help: "Open Find/Replace panel",
                },
            ],
            editor: vec![
                ShortcutDef {
                    trigger: egui::KeyboardShortcut::new(cmd, egui::Key::I),
                    secondary_trigger: None,
                    action: AppAction::SendToAi,
                    help: "Send highlighted text to AI",
                },
                ShortcutDef {
                    trigger: egui::KeyboardShortcut::new(alt, egui::Key::V),
                    secondary_trigger: None,
                    action: AppAction::ToggleVerticalEdit,
                    help: "Toggle Vim-style Vertical Edit mode",
                },
                ShortcutDef {
                    trigger: egui::KeyboardShortcut::new(none, egui::Key::Escape),
                    secondary_trigger: None,
                    action: AppAction::AbortOrClose,
                    help: "Cancel current operation / Close panels",
                },
            ],
        }
    }

    /// Checks if a specific action was triggered and consumes it to prevent OS overlap
    pub fn check_action(&self, ctx: &egui::Context, action: AppAction) -> bool {
        for shortcut in self.global.iter().chain(self.editor.iter()) {
            if shortcut.action == action {
                return shortcut.consume(ctx);
            }
        }
        false
    }
}
