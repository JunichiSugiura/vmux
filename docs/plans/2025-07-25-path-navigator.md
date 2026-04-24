# Path Navigator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add smart filesystem path completion to the command bar with inline ghost text and directory suggestions.

**Architecture:** WASM command bar emits `PathCompleteRequest` via `cef.emit()`. Desktop observes it, reads the filesystem, and sends `PathCompleteResponse` back via `HostEmitEvent`. WASM renders ghost text in the input and path completions in the result list. Tab accepts ghost text.

**Tech Stack:** Rust, Dioxus (WASM), Bevy ECS, CEF IPC (HostEmitEvent/JsEmitEvent), RON serialization.

---

### Task 1: Add event types for path completion

**Files:**
- Modify: `crates/vmux_command_bar/src/event.rs`

- [ ] **Step 1: Add PathCompleteRequest and PathCompleteResponse structs**

Add to the end of `crates/vmux_command_bar/src/event.rs`:

```rust
pub const PATH_COMPLETE_REQUEST: &str = "path-complete-request";
pub const PATH_COMPLETE_RESPONSE: &str = "path-complete-response";

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PathCompleteRequest {
    pub query: String,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PathEntry {
    pub name: String,
    pub is_dir: bool,
    pub full_path: String,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PathCompleteResponse {
    pub completions: Vec<PathEntry>,
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p vmux_command_bar`

- [ ] **Step 3: Commit**

```bash
git add crates/vmux_command_bar/src/event.rs
git commit -m "feat: add path completion event types"
```

---

### Task 2: Desktop handler — observe PathCompleteRequest and respond

**Files:**
- Modify: `crates/vmux_desktop/src/command_bar.rs`

- [ ] **Step 1: Register the JsEmitEvent plugin and observer**

In `CommandBarInputPlugin::build`, add after the existing `JsEmitEventPlugin::<CommandBarActionEvent>`:

```rust
.add_plugins(JsEmitEventPlugin::<PathCompleteRequest>::default())
.add_observer(on_path_complete_request)
```

Add to imports at the top of the file:

```rust
use vmux_command_bar::event::{
    PathCompleteRequest, PathCompleteResponse, PathEntry,
    PATH_COMPLETE_RESPONSE,
};
```

- [ ] **Step 2: Implement `on_path_complete_request` observer**

Add this function at the end of `command_bar.rs`:

```rust
fn on_path_complete_request(
    trigger: On<Receive<PathCompleteRequest>>,
    modal_q: Query<Entity, With<Modal>>,
    browsers: NonSend<Browsers>,
    mut commands: Commands,
) {
    let query = &trigger.event().payload.query;
    let Ok(modal_e) = modal_q.single() else { return };
    if !browsers.has_browser(modal_e) || !browsers.host_emit_ready(&modal_e) {
        return;
    }

    let completions = complete_path(query);
    let payload = PathCompleteResponse { completions };
    let ron_body = ron::ser::to_string(&payload).unwrap_or_default();
    commands.trigger(HostEmitEvent::new(modal_e, PATH_COMPLETE_RESPONSE, &ron_body));
}

fn complete_path(query: &str) -> Vec<PathEntry> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());

    // Split at last '/' to get parent dir and prefix
    let (parent_str, prefix) = if let Some(pos) = query.rfind('/') {
        (&query[..=pos], &query[pos + 1..])
    } else {
        ("", query)
    };

    // Resolve the parent directory to an absolute path
    let resolved_parent = if parent_str.starts_with("~/") || parent_str == "~/" {
        std::path::PathBuf::from(&home).join(&parent_str[2..])
    } else if parent_str.starts_with('/') {
        std::path::PathBuf::from(parent_str)
    } else if parent_str.is_empty() {
        std::path::PathBuf::from(&home)
    } else {
        std::path::PathBuf::from(&home).join(parent_str)
    };

    let Ok(entries) = std::fs::read_dir(&resolved_parent) else {
        return Vec::new();
    };

    let prefix_lower = prefix.to_lowercase();
    let mut results: Vec<PathEntry> = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

        if !prefix.is_empty() && !name.to_lowercase().starts_with(&prefix_lower) {
            continue;
        }

        let display_name = if is_dir {
            format!("{}/", name)
        } else {
            name.clone()
        };

        let full_path = if parent_str.is_empty() {
            display_name.clone()
        } else {
            format!("{}{}", parent_str, display_name)
        };

        results.push(PathEntry {
            name: display_name,
            is_dir,
            full_path,
        });
    }

    // Sort: dirs first, hidden after visible, then alphabetical
    results.sort_by(|a, b| {
        let a_hidden = a.name.starts_with('.');
        let b_hidden = b.name.starts_with('.');
        b.is_dir.cmp(&a.is_dir)
            .then(a_hidden.cmp(&b_hidden))
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    results.truncate(20);
    results
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p vmux_desktop`

- [ ] **Step 4: Commit**

```bash
git add crates/vmux_desktop/src/command_bar.rs
git commit -m "feat: desktop handler for path completion requests"
```

---

### Task 3: WASM — send PathCompleteRequest and receive PathCompleteResponse

**Files:**
- Modify: `crates/vmux_command_bar/src/app.rs`

- [ ] **Step 1: Add imports and signals for path completion**

Add to imports at top of `app.rs`:

```rust
use vmux_command_bar::event::{
    PathCompleteRequest, PathCompleteResponse, PathEntry,
    PATH_COMPLETE_REQUEST, PATH_COMPLETE_RESPONSE,
};
```

Note: `PATH_COMPLETE_REQUEST` is used by the emit call, `PATH_COMPLETE_RESPONSE` by the listener.

Inside the `App` component, after the existing signal declarations (`is_open`, etc.), add:

```rust
let mut path_completions = use_signal(Vec::<PathEntry>::new);
let mut pending_path_query = use_signal(String::new);
```

- [ ] **Step 2: Add listener for PathCompleteResponse**

After the existing `use_event_listener::<CommandBarOpenEvent, _>(...)` block, add:

```rust
let _path_listener =
    use_event_listener::<PathCompleteResponse, _>(PATH_COMPLETE_RESPONSE, move |data| {
        path_completions.set(data.completions);
    });
```

- [ ] **Step 3: Add a use_effect to debounce and send PathCompleteRequest**

After the path_listener, add:

```rust
use_effect(move || {
    let q = query();
    if !looks_like_path(q.trim()) {
        path_completions.set(Vec::new());
        return;
    }
    let _ = try_cef_emit_serde(&PathCompleteRequest {
        query: q.trim().to_string(),
    });
});
```

Note: This fires on every query change. True debounce would require `setTimeout` via `web_sys`, but the desktop handler is fast enough (local fs read) that per-keystroke requests are acceptable. If needed, debounce can be added later.

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p vmux_command_bar`
(This triggers `dx build` via build.rs which may take 60-90s.)

- [ ] **Step 5: Commit**

```bash
git add crates/vmux_command_bar/src/app.rs
git commit -m "feat: WASM path completion request/response wiring"
```

---

### Task 4: WASM — render ghost text in the input

**Files:**
- Modify: `crates/vmux_command_bar/src/app.rs`

- [ ] **Step 1: Compute ghost text from completions**

Inside the `App` component, after computing `results` and `sel`, add:

```rust
let ghost_text = {
    let q = query();
    let completions = path_completions();
    if let Some(first) = completions.first() {
        let full = &first.full_path;
        if full.to_lowercase().starts_with(&q.trim().to_lowercase()) {
            full[q.trim().len()..].to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    }
};
```

- [ ] **Step 2: Wrap the input in a relative container with ghost text overlay**

Replace the current `input` element with a wrapper that includes ghost text. The current input is inside `div { class: "flex items-center gap-2 rounded-lg bg-white/5 px-3", ... }`.

Replace the `input { ... }` block with:

```rust
div { class: "relative flex-1",
    if !ghost_text.is_empty() {
        div {
            class: "pointer-events-none absolute inset-0 flex items-center",
            span { class: "invisible text-base", "{q}" }
            span { class: "text-base text-muted-foreground/40", "{ghost_text}" }
        }
    }
    input {
        id: "command-bar-input",
        r#type: "text",
        class: "w-full py-2.5 text-base text-foreground bg-transparent outline-none placeholder:text-muted-foreground",
        // ... rest of input props unchanged
    }
}
```

Keep all existing input attributes (placeholder, value, autofocus, oninput, onkeydown) exactly as they are.

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p vmux_command_bar`

- [ ] **Step 4: Commit**

```bash
git add crates/vmux_command_bar/src/app.rs
git commit -m "feat: ghost text overlay for path completion"
```

---

### Task 5: WASM — Tab key accepts ghost text

**Files:**
- Modify: `crates/vmux_command_bar/src/app.rs`

- [ ] **Step 1: Add Tab key handling in onkeydown**

In the `onkeydown` handler, before the existing `if go_down` check, add:

```rust
if e.key() == Key::Tab {
    e.prevent_default();
    let gt = ghost_text.clone();
    if !gt.is_empty() {
        let new_val = format!("{}{}", q, gt);
        query.set(new_val.clone());
        selected.set(0);
        // Set the input element's value directly for immediate feedback
        if let Some(el) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("command-bar-input"))
        {
            let input: web_sys::HtmlInputElement = el.unchecked_into();
            input.set_value(&new_val);
            let len = new_val.len() as u32;
            let _ = input.set_selection_range(len, len);
        }
    }
    return;
}
```

Note: `ghost_text` is computed in the render scope. To use it in the closure, it needs to be captured. Since `ghost_text` is a `String` (not a signal), clone it into the closure:

The `ghost_text` variable is computed each render and the `onkeydown` closure captures it. Since `ghost_text` is a plain `String`, the closure captures a clone automatically.

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p vmux_command_bar`

- [ ] **Step 3: Commit**

```bash
git add crates/vmux_command_bar/src/app.rs
git commit -m "feat: Tab key accepts ghost text in command bar"
```

---

### Task 6: WASM — show path completions in result list

**Files:**
- Modify: `crates/vmux_command_bar/src/app.rs`

- [ ] **Step 1: Insert path completions into the results list**

In the `filter_results` function, the path completions come from the signal, not from `filter_results` itself. Instead, modify the result list construction in the `App` component.

After computing `results` from `filter_results(...)`, insert path completions at the top:

```rust
let results = {
    let mut r = filter_results(&q, &tabs, &commands, is_new_tab);
    let completions = path_completions();
    if !completions.is_empty() {
        // Insert path completions at the top, before other results, max 5
        let path_items: Vec<ResultItem> = completions.iter()
            .filter(|e| e.is_dir)
            .take(5)
            .map(|e| ResultItem::Terminal { path: e.full_path.clone() })
            .collect();
        // Remove the existing Terminal path item if present (avoid duplicates)
        r.retain(|item| !matches!(item, ResultItem::Terminal { path } if !path.is_empty()));
        let mut combined = path_items;
        combined.extend(r);
        combined
    } else {
        r
    }
};
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p vmux_command_bar`

- [ ] **Step 3: Commit**

```bash
git add crates/vmux_command_bar/src/app.rs
git commit -m "feat: show path completions in command bar result list"
```

---

### Task 7: Build and verify end-to-end

**Files:**
- None (verification only)

- [ ] **Step 1: Full build**

Run: `cargo build`
Expected: Compiles with no errors. WASM apps rebuilt.

- [ ] **Step 2: Commit all remaining changes**

```bash
git add -A
git commit -m "feat: path navigator for command bar"
```
