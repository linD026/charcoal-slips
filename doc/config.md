### Part I: The JSON Theme Architecture

Hardcoded `egui` visuals have been entirely replaced by a highly configurable JSON structure (`ThemeConfig`) supporting dual Light/Dark modes, Hex string parsing, and distinct alpha-channel transparency.

* **Hex Parsing & Fallbacks:** The engine supports 6-character (`#RRGGBB`) and 8-character (`#RRGGBBAA`) hex codes. If an invalid string is provided in the JSON, the parser intercepts the failure, logs a terminal error, and gracefully falls back to a high-contrast default (Magenta) to prevent application crashes.
* **Visual Scoping (The "Translucent Conflict"):** Setting a global transparent selection background makes text highlighting beautiful but renders the File Tree selection invisible in Light Mode. 
    * *Global Scope:* The app applies a solid `ui_selection_bg` globally for the file tree and menus.
    * *Local Scope:* Directly before rendering the `TextEdit`, the app calls `ui.visuals_mut()` to temporarily override the selection background with the translucent `editor_selection_bg`.
* **Borrow-Safe Rendering Extraction:** Because `egui` requires mutable access to text buffers, passing configuration colors dynamically can trigger `E0500` / `E0502` borrow panics. The architecture resolves this by extracting and `.clone()`-ing the required `SyntaxTheme` and `TerminalTheme` values at the top of the render functions, cleanly dropping the read-lock on `self.config` before mutable UI building begins.

---


