//! Command registry and keyboard shortcut system.
//!
//! Every user-facing action is a `Command` with an ID, display name,
//! keyboard shortcut, and context-dependent availability.

use std::collections::HashMap;

// ── Shortcut representation ─────────────────────────────────────

/// Keyboard modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub command: bool, // ⌘ on macOS
}

impl Modifiers {
    pub const NONE: Self = Self {
        ctrl: false,
        shift: false,
        alt: false,
        command: false,
    };
    pub const CMD: Self = Self {
        ctrl: false,
        shift: false,
        alt: false,
        command: true,
    };
    pub const CMD_SHIFT: Self = Self {
        ctrl: false,
        shift: true,
        alt: false,
        command: true,
    };
    pub const CMD_ALT: Self = Self {
        ctrl: false,
        shift: false,
        alt: true,
        command: true,
    };
    pub const SHIFT: Self = Self {
        ctrl: false,
        shift: true,
        alt: false,
        command: false,
    };
}

/// A keyboard shortcut (modifier + key).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Shortcut {
    pub modifiers: Modifiers,
    pub key: String,
}

impl Shortcut {
    pub fn new(modifiers: Modifiers, key: impl Into<String>) -> Self {
        Self {
            modifiers,
            key: key.into(),
        }
    }

    /// Format for display: "⌘S", "⌘⇧Z", etc.
    pub fn display(&self) -> String {
        let mut s = String::new();
        if self.modifiers.ctrl {
            s.push('⌃');
        }
        if self.modifiers.alt {
            s.push('⌥');
        }
        if self.modifiers.shift {
            s.push('⇧');
        }
        if self.modifiers.command {
            s.push('⌘');
        }
        s.push_str(&self.key.to_uppercase());
        s
    }
}

// ── Command context ─────────────────────────────────────────────

/// Contexts in which a command may be available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandContext {
    /// Always available.
    Global,
    /// Only when the timeline has focus.
    Timeline,
    /// Only when the viewer has focus.
    Viewer,
    /// Only when a clip is selected.
    ClipSelected,
    /// Only during playback.
    Playing,
    /// Only when stopped.
    Stopped,
}

// ── Command definition ──────────────────────────────────────────

/// A registered command.
#[derive(Debug, Clone)]
pub struct Command {
    /// Unique command ID (e.g., "edit.undo").
    pub id: &'static str,
    /// Display name (e.g., "Undo").
    pub name: &'static str,
    /// Category for command palette grouping.
    pub category: &'static str,
    /// Keyboard shortcut (if any).
    pub shortcut: Option<Shortcut>,
    /// Contexts in which this command is available.
    pub contexts: &'static [CommandContext],
}

// ── Registry ────────────────────────────────────────────────────

/// Central registry of all commands with fuzzy search.
pub struct CommandRegistry {
    commands: Vec<Command>,
    by_id: HashMap<&'static str, usize>,
    by_shortcut: HashMap<Shortcut, usize>,
}

impl CommandRegistry {
    /// Create a new registry with all built-in commands.
    pub fn new() -> Self {
        let mut reg = Self {
            commands: Vec::new(),
            by_id: HashMap::new(),
            by_shortcut: HashMap::new(),
        };
        reg.register_builtins();
        reg
    }

    /// Register a command.
    pub fn register(&mut self, cmd: Command) {
        let idx = self.commands.len();
        self.by_id.insert(cmd.id, idx);
        if let Some(ref shortcut) = cmd.shortcut {
            self.by_shortcut.insert(shortcut.clone(), idx);
        }
        self.commands.push(cmd);
    }

    /// Look up a command by ID.
    pub fn get(&self, id: &str) -> Option<&Command> {
        self.by_id.get(id).map(|&i| &self.commands[i])
    }

    /// Look up a command by shortcut.
    pub fn get_by_shortcut(&self, shortcut: &Shortcut) -> Option<&Command> {
        self.by_shortcut.get(shortcut).map(|&i| &self.commands[i])
    }

    /// All registered commands.
    pub fn all(&self) -> &[Command] {
        &self.commands
    }

    /// Fuzzy search commands by name. Returns matching commands sorted by relevance.
    pub fn search(&self, query: &str, contexts: &[CommandContext]) -> Vec<&Command> {
        if query.is_empty() {
            return self
                .commands
                .iter()
                .filter(|cmd| is_available(cmd, contexts))
                .collect();
        }

        let query_lower = query.to_lowercase();
        let mut results: Vec<(&Command, i32)> = self
            .commands
            .iter()
            .filter(|cmd| is_available(cmd, contexts))
            .filter_map(|cmd| {
                let name_lower = cmd.name.to_lowercase();
                let id_lower = cmd.id.to_lowercase();

                // Exact prefix match (highest priority)
                if name_lower.starts_with(&query_lower) {
                    return Some((cmd, 100));
                }
                // Word start match
                if name_lower
                    .split_whitespace()
                    .any(|w| w.starts_with(&query_lower))
                {
                    return Some((cmd, 80));
                }
                // Subsequence match
                if is_subsequence(&query_lower, &name_lower) {
                    return Some((cmd, 60));
                }
                // ID match
                if id_lower.contains(&query_lower) {
                    return Some((cmd, 40));
                }
                None
            })
            .collect();

        results.sort_by(|a, b| b.1.cmp(&a.1));
        results.into_iter().map(|(cmd, _)| cmd).collect()
    }

    /// Register all built-in commands.
    fn register_builtins(&mut self) {
        use CommandContext::*;

        // ── File commands ────────────────────────────
        self.register(Command {
            id: "file.new",
            name: "New Project",
            category: "File",
            shortcut: Some(Shortcut::new(Modifiers::CMD, "N")),
            contexts: &[Global],
        });
        self.register(Command {
            id: "file.open",
            name: "Open Project",
            category: "File",
            shortcut: Some(Shortcut::new(Modifiers::CMD, "O")),
            contexts: &[Global],
        });
        self.register(Command {
            id: "file.save",
            name: "Save Project",
            category: "File",
            shortcut: Some(Shortcut::new(Modifiers::CMD, "S")),
            contexts: &[Global],
        });
        self.register(Command {
            id: "file.save_as",
            name: "Save As...",
            category: "File",
            shortcut: Some(Shortcut::new(Modifiers::CMD_SHIFT, "S")),
            contexts: &[Global],
        });
        self.register(Command {
            id: "file.import",
            name: "Import Media",
            category: "File",
            shortcut: Some(Shortcut::new(Modifiers::CMD, "I")),
            contexts: &[Global],
        });

        // ── Edit commands ────────────────────────────
        self.register(Command {
            id: "edit.undo",
            name: "Undo",
            category: "Edit",
            shortcut: Some(Shortcut::new(Modifiers::CMD, "Z")),
            contexts: &[Global],
        });
        self.register(Command {
            id: "edit.redo",
            name: "Redo",
            category: "Edit",
            shortcut: Some(Shortcut::new(Modifiers::CMD_SHIFT, "Z")),
            contexts: &[Global],
        });
        self.register(Command {
            id: "edit.cut",
            name: "Cut",
            category: "Edit",
            shortcut: Some(Shortcut::new(Modifiers::CMD, "X")),
            contexts: &[ClipSelected],
        });
        self.register(Command {
            id: "edit.copy",
            name: "Copy",
            category: "Edit",
            shortcut: Some(Shortcut::new(Modifiers::CMD, "C")),
            contexts: &[ClipSelected],
        });
        self.register(Command {
            id: "edit.paste",
            name: "Paste",
            category: "Edit",
            shortcut: Some(Shortcut::new(Modifiers::CMD, "V")),
            contexts: &[Timeline],
        });
        self.register(Command {
            id: "edit.delete",
            name: "Delete",
            category: "Edit",
            shortcut: Some(Shortcut::new(Modifiers::NONE, "Delete")),
            contexts: &[ClipSelected],
        });
        self.register(Command {
            id: "edit.select_all",
            name: "Select All",
            category: "Edit",
            shortcut: Some(Shortcut::new(Modifiers::CMD, "A")),
            contexts: &[Timeline],
        });

        // ── Transport commands ───────────────────────
        self.register(Command {
            id: "transport.play_pause",
            name: "Play/Pause",
            category: "Transport",
            shortcut: Some(Shortcut::new(Modifiers::NONE, "Space")),
            contexts: &[Global],
        });
        self.register(Command {
            id: "transport.stop",
            name: "Stop",
            category: "Transport",
            shortcut: Some(Shortcut::new(Modifiers::NONE, "Escape")),
            contexts: &[Playing],
        });
        self.register(Command {
            id: "transport.jkl_reverse",
            name: "Reverse Shuttle (J)",
            category: "Transport",
            shortcut: Some(Shortcut::new(Modifiers::NONE, "J")),
            contexts: &[Global],
        });
        self.register(Command {
            id: "transport.jkl_pause",
            name: "Pause Shuttle (K)",
            category: "Transport",
            shortcut: Some(Shortcut::new(Modifiers::NONE, "K")),
            contexts: &[Global],
        });
        self.register(Command {
            id: "transport.jkl_forward",
            name: "Forward Shuttle (L)",
            category: "Transport",
            shortcut: Some(Shortcut::new(Modifiers::NONE, "L")),
            contexts: &[Global],
        });
        self.register(Command {
            id: "transport.prev_frame",
            name: "Previous Frame",
            category: "Transport",
            shortcut: Some(Shortcut::new(Modifiers::NONE, "Left")),
            contexts: &[Global],
        });
        self.register(Command {
            id: "transport.next_frame",
            name: "Next Frame",
            category: "Transport",
            shortcut: Some(Shortcut::new(Modifiers::NONE, "Right")),
            contexts: &[Global],
        });
        self.register(Command {
            id: "transport.goto_start",
            name: "Go to Start",
            category: "Transport",
            shortcut: Some(Shortcut::new(Modifiers::NONE, "Home")),
            contexts: &[Global],
        });
        self.register(Command {
            id: "transport.goto_end",
            name: "Go to End",
            category: "Transport",
            shortcut: Some(Shortcut::new(Modifiers::NONE, "End")),
            contexts: &[Global],
        });

        // ── Timeline commands ────────────────────────
        self.register(Command {
            id: "timeline.mark_in",
            name: "Set In Point",
            category: "Timeline",
            shortcut: Some(Shortcut::new(Modifiers::NONE, "I")),
            contexts: &[Timeline],
        });
        self.register(Command {
            id: "timeline.mark_out",
            name: "Set Out Point",
            category: "Timeline",
            shortcut: Some(Shortcut::new(Modifiers::NONE, "O")),
            contexts: &[Timeline],
        });
        self.register(Command {
            id: "timeline.split",
            name: "Split at Playhead",
            category: "Timeline",
            shortcut: Some(Shortcut::new(Modifiers::CMD, "B")),
            contexts: &[Timeline],
        });
        self.register(Command {
            id: "timeline.ripple_delete",
            name: "Ripple Delete",
            category: "Timeline",
            shortcut: Some(Shortcut::new(Modifiers::SHIFT, "Delete")),
            contexts: &[ClipSelected],
        });
        self.register(Command {
            id: "timeline.zoom_in",
            name: "Zoom In",
            category: "Timeline",
            shortcut: Some(Shortcut::new(Modifiers::CMD, "=")),
            contexts: &[Timeline],
        });
        self.register(Command {
            id: "timeline.zoom_out",
            name: "Zoom Out",
            category: "Timeline",
            shortcut: Some(Shortcut::new(Modifiers::CMD, "-")),
            contexts: &[Timeline],
        });
        self.register(Command {
            id: "timeline.zoom_fit",
            name: "Zoom to Fit",
            category: "Timeline",
            shortcut: Some(Shortcut::new(Modifiers::CMD_SHIFT, "0")),
            contexts: &[Timeline],
        });

        // ── View commands ────────────────────────────
        self.register(Command {
            id: "view.command_palette",
            name: "Command Palette",
            category: "View",
            shortcut: Some(Shortcut::new(Modifiers::CMD, "K")),
            contexts: &[Global],
        });
        self.register(Command {
            id: "view.toggle_inspector",
            name: "Toggle Inspector",
            category: "View",
            shortcut: None,
            contexts: &[Global],
        });
        self.register(Command {
            id: "view.toggle_color_wheels",
            name: "Toggle Color Wheels",
            category: "View",
            shortcut: None,
            contexts: &[Global],
        });
        self.register(Command {
            id: "view.toggle_audio_mixer",
            name: "Toggle Audio Mixer",
            category: "View",
            shortcut: None,
            contexts: &[Global],
        });
        self.register(Command {
            id: "view.fullscreen",
            name: "Toggle Fullscreen",
            category: "View",
            shortcut: Some(Shortcut::new(Modifiers::CMD, "F")),
            contexts: &[Global],
        });

        // ── Clip commands ────────────────────────────
        self.register(Command {
            id: "clip.speed",
            name: "Change Clip Speed",
            category: "Clip",
            shortcut: Some(Shortcut::new(Modifiers::CMD, "R")),
            contexts: &[ClipSelected],
        });
        self.register(Command {
            id: "clip.freeze_frame",
            name: "Freeze Frame",
            category: "Clip",
            shortcut: Some(Shortcut::new(Modifiers::CMD_SHIFT, "F")),
            contexts: &[ClipSelected],
        });
        self.register(Command {
            id: "clip.nest",
            name: "Nest Clip",
            category: "Clip",
            shortcut: Some(Shortcut::new(Modifiers::CMD_ALT, "N")),
            contexts: &[ClipSelected],
        });
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a command is available in the given contexts.
fn is_available(cmd: &Command, active_contexts: &[CommandContext]) -> bool {
    // Global commands are always available
    if cmd.contexts.contains(&CommandContext::Global) {
        return true;
    }
    // Otherwise, at least one command context must match an active context
    cmd.contexts.iter().any(|c| active_contexts.contains(c))
}

/// Check if `needle` is a subsequence of `haystack`.
fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let mut haystack_chars = haystack.chars();
    for needle_char in needle.chars() {
        loop {
            match haystack_chars.next() {
                Some(c) if c == needle_char => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_lookup() {
        let reg = CommandRegistry::new();
        let cmd = reg.get("edit.undo").unwrap();
        assert_eq!(cmd.name, "Undo");
        assert_eq!(cmd.category, "Edit");
    }

    #[test]
    fn test_shortcut_lookup() {
        let reg = CommandRegistry::new();
        let shortcut = Shortcut::new(Modifiers::CMD, "Z");
        let cmd = reg.get_by_shortcut(&shortcut).unwrap();
        assert_eq!(cmd.id, "edit.undo");
    }

    #[test]
    fn test_shortcut_display() {
        let s = Shortcut::new(Modifiers::CMD_SHIFT, "Z");
        assert_eq!(s.display(), "⇧⌘Z");
    }

    #[test]
    fn test_fuzzy_search() {
        let reg = CommandRegistry::new();
        let results = reg.search("undo", &[CommandContext::Global]);
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "edit.undo");
    }

    #[test]
    fn test_fuzzy_search_subsequence() {
        let reg = CommandRegistry::new();
        let results = reg.search("splt", &[CommandContext::Global, CommandContext::Timeline]);
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "timeline.split");
    }

    #[test]
    fn test_context_filtering() {
        let reg = CommandRegistry::new();
        // "Delete" only available when a clip is selected
        let results = reg.search("Delete", &[CommandContext::Timeline]);
        let has_delete = results.iter().any(|c| c.id == "edit.delete");
        assert!(!has_delete); // Timeline context doesn't include ClipSelected

        let results = reg.search(
            "Delete",
            &[CommandContext::ClipSelected, CommandContext::Timeline],
        );
        let has_delete = results.iter().any(|c| c.id == "edit.delete");
        assert!(has_delete);
    }

    #[test]
    fn test_is_subsequence() {
        assert!(is_subsequence("udo", "undo"));
        assert!(is_subsequence("splt", "split at playhead"));
        assert!(!is_subsequence("xyz", "undo"));
    }

    #[test]
    fn test_all_commands_have_unique_ids() {
        let reg = CommandRegistry::new();
        let mut ids: Vec<&str> = reg.all().iter().map(|c| c.id).collect();
        let count = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), count, "duplicate command IDs found");
    }
}
