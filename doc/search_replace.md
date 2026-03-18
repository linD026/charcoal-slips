### Part I: The Search & Replace Engine

The Search engine is driven by a centralized `SearchState` struct. It treats the text buffer as the ultimate source of truth, heavily recalculating state upon any mutation to prevent "Index Shift" bugs.

* **The Unicode Byte-to-Char Fix:** Rust’s native `String::find()` returns byte indices. However, `egui::text::CCursor` requires character indices. To prevent the editor from crashing or misaligning when encountering multi-byte characters (emojis, em-dashes), the engine implements an $O(n)$ sliding window to strictly map byte positions to character counts before storing them in the `matches` vector.
* **"Search-on-Enter" vs. "Search-on-Type":** To prevent state desynchronization (e.g., replacing a word that no longer matches the text box), modifying the Find input instantly raises a `query_modified` flag, clears the cache, and locks all Replace buttons. The engine only executes when the `Enter` key yields focus.
* **The "Safe Advance" Mutation Logic:** Replacing text fundamentally alters the array length and positioning. When a replacement occurs:
    1.  The engine edits the live buffer.
    2.  It re-runs `perform_search` from scratch to rebuild perfectly accurate indices.
    3.  It scans the *new* cache to find the first match whose start index is strictly greater than the insertion point. This prevents "recursive replacement" infinite loops (e.g., accidentally turning `"cat"` into `"caterpillarpillarpillar"`).
* **Memory vs. Disk Synchronization (All Files):** When iterating through external files for global searches or replacements, the engine checks if the target `Path` matches the currently open file. If it does, it intercepts the disk read and reads directly from the live `self.editor_text` memory buffer, preventing the loss of unsaved changes.

---

### Part II: Immediate-Mode Rendering & Navigation

Managing layout geometry dynamically while a user hops between files requires strict execution ordering.

* **Centered Viewport Scrolling:** Standard cursor jumps only move the blinking text caret, leaving the camera behind. The engine extracts the physical screen `Rect` of the target character index from the text `Galley`, translates it to global coordinates, and calls `ui.scroll_to_rect(rect, Align::Center)` to perfectly track the user's jumps.
* **Translucent Visual Overlays:** Instead of permanently altering the `LayoutJob` (which would corrupt LaTeX syntax highlighting), search matches are rendered using a painter overlay. The engine loops through the active `matches` and paints an `unmultiplied` translucent bounding box directly over the text geometry, mimicking a physical highlighter pen.
* **The Focus Flag System:** In `egui`, a UI element cannot be assigned focus before it is drawn. Keyboard shortcuts (`Ctrl+F`, `Ctrl+R`) raise boolean flags (`focus_find`). During the render pass, the text input consumes the flag and instantly requests focus, creating zero-latency keyboard navigation.

---

### Part III: Event Interception & Priority Chains

Global keyboard shortcuts must respect the context of active UI elements to prevent catastrophic multi-action triggers.

* **The Escape Priority Chain:** Pressing `Escape` executes based on a strict UI hierarchy:
    1.  If the Autocomplete popup is active, `Escape` strictly dismisses the popup and ignores the Search panel.
    2.  If Autocomplete is closed, `Escape` consumes the keystroke, closes the Search panel, clears the match cache, and explicitly requests keyboard focus back to the `latex_editor` ID to maintain an uninterrupted typing flow.
