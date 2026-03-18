# Charcoal Slips

Charcoal Slips (cclips) is a latex editor with a LLM-based grammar checking.
It targets to help the user to check the grammar for the writing.
cclips provides the basic latex editor features, such as search-replace, syntax highlighting, and autocompletion.
cclips allows tthe user to configure the editor features statistically (need to relaunch the program).

## Build

to build cclips, use the following commands:

```
cargo build
cp ./target/debug/ccslips <your-location>
```

## Configuration

When you run cclips, it will generate the config file, `config_charcoal_slips.json`.
You can modify `config_charcoal_slips.json` to change the configuration, such as color, autocomplete template, and more.

## Shortcuts

### Editors

- `Ctrl`+`S` to save
    > Note that cclips performs auto-save when you switch to another file.
- `Ctrl`+`B` to save the current file and build.
    > You can customize the build command in the configuration.
- `Ctrl`+`I` to send the selected words to LLM.
    > You can customize the system prompt and inference platform in the configuration.

### Search and Replace

- `Ctrl`+`f` to open the panel.
    - `Esc` to close the panel.
    - `Ctrl`+`r` to switch from search input to replace input.
        > Note that you only can use this when you open the panel.

---

## Development

### 1. The Core Editing Engine
* **Context-Aware Syntax Highlighting:** Real-time parsing and coloring for LaTeX commands, comments, brackets, and math modes.
* **Dynamic Line Numbers:** A fully integrated, left-aligned gutter that accurately calculates visual-to-logical line mappings and scales dynamically with the file size.
* **Smart Autocomplete & Snippets:** A floating, context-sensitive popup menu that triggers on specific keystrokes (`\`, etc.). It supports macro insertion, dynamically parses `.bib` files for citation suggestions, and handles label lookups.
* **Zoom & Scale:** Real-time font size scaling using `Ctrl++` and `Ctrl+-`.

### 2. Workspace & Build Pipeline
* **Live File Explorer:** A collapsible, hierarchical directory tree in the left panel. Clicking a file instantly safely loads it into the editor buffer.
* **State-Safe Auto-Save:** The editor intelligently and silently saves your work whenever you switch files, trigger a build, or jump to an AI index result, ensuring zero data loss.
* **Integrated Build System:** Pressing `Ctrl+B` executes a configurable build command (e.g., `make`) directly in the working directory.
* **Color-Coded Terminal:** A dedicated right-panel terminal that captures standard output, system errors, and build results, automatically coloring the text (Red for errors, Green for success) for rapid debugging.

### 3. Advanced Search & Replace (Multi-File)
* **Unicode-Safe Indexing:** A custom $O(n)$ search engine that safely maps byte-indices to character-indices, preventing the editor from crashing on emojis, em-dashes, or foreign characters.
* **Translucent Visual Overlays:** Search matches are painted directly over the UI with an unmultiplied alpha channel, acting like a physical highlighter pen without destroying the syntax colors underneath.
* **The "Safe Advance" Replace:** An anti-recursive replacement engine that mathematically skips over newly inserted text, preventing infinite loops (e.g., turning "cat" into "caterpillarpillar").
* **Global Workspace Scope:** A "Search All Files" toggle that silently reads, replaces, and saves modifications across the entire directory on disk without locking up the UI thread.
* **Centered Camera Tracking:** Jumping between search results automatically pans the `ScrollArea` camera so the targeted word is always dead center on your screen.

### 4. Local AI Integration (The "🧠 AI Index")
* **LLM Grammar & Linguistics Engine:** Highlighting text and pressing `Ctrl+I` pipes the selection to a local AI model (configured for `qwen3` by default) to act as a strict linguistic and grammar editor.
* **Interactive AI Dashboard:** The right panel switches to an AI Index displaying the generated edits alongside timestamps.
* **Contextual Teleportation:** Clicking "Jump to Selection" on an AI result instantly opens the correct file, restores the exact text selection, and centers the camera on the text you need to fix.

### 5. JSON-Driven Theme Architecture
* **Hot-Swappable Modes:** Toggle between Dark Mode (VS Code style) and Light Mode (Overleaf/Apple style) instantly.
* **Total Configuration:** Every visual element—from gutter text and syntax colors to popup backgrounds and AI buttons—is controlled via Hex strings in `config_charcoal_slips.json`. 
* **Safe Fallbacks:** The custom hex parser (`#RRGGBB` and `#RRGGBBAA`) automatically catches typos in the JSON and falls back to a safe default color without crashing the app.
* **Visual Scoping:** The UI intelligently separates global selection styles (solid gray/blue for file trees) from editor selection styles (translucent highlights for text readability).

### 6. Keyboard-Driven UX & Safety Locks
* **The Escape Priority Chain:** Pressing `Escape` respects your context. It dismisses the Autocomplete popup first. If that is closed, it dismisses the Search panel and instantly snaps your focus back to the text editor.
* **Search-on-Enter Synchronization:** To prevent ghost replacements, the search engine only locks in queries when you press `Enter`. Modifying the text box instantly disables the "Replace" buttons until the new query is verified.
* **Borrow-Safe Rendering:** The architecture strictly extracts and clones all dynamic themes *before* touching the `egui` render loop, completely eliminating Rust `E0500`/`E0502` borrow checker panics.
