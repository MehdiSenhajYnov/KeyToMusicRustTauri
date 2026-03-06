use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Node in the combo Trie
#[derive(Debug, Default)]
struct TrieNode {
    /// If true, a binding exists for the combo up to this node
    is_binding: bool,
    /// Children keyed by the next key in the combo
    children: HashMap<String, TrieNode>,
}

/// Trie (prefix tree) for storing key combos
/// Allows efficient lookup and leaf detection
#[derive(Debug, Default)]
pub struct ComboTrie {
    root: TrieNode,
}

impl ComboTrie {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a Trie from a list of key codes (bindings)
    /// Each key code can be:
    /// - "KeyA" (single key)
    /// - "Ctrl+KeyA" (modifier + key)
    /// - "KeyA+KeyZ" (multi-key chord)
    /// - "Ctrl+KeyA+KeyZ" (modifier + multi-key)
    pub fn build_from_bindings(bindings: &[String]) -> Self {
        let mut trie = Self::new();
        for binding in bindings {
            trie.insert(binding);
        }
        trie
    }

    /// Insert a combo into the Trie
    /// Combo format: "Ctrl+Shift+KeyA+KeyZ" where modifiers come first,
    /// then base keys sorted alphabetically
    fn insert(&mut self, combo: &str) {
        let parts = Self::normalize_combo(combo);
        let mut node = &mut self.root;

        for part in parts {
            node = node.children.entry(part).or_default();
        }
        node.is_binding = true;
    }

    /// Normalize a combo string into sorted parts
    /// "Ctrl+KeyZ+KeyA" -> ["Ctrl", "KeyA", "KeyZ"]
    /// Modifiers always come first (in order Ctrl, Shift, Alt), then base keys alphabetically
    pub fn normalize_combo(combo: &str) -> Vec<String> {
        let parts: Vec<&str> = combo.split('+').collect();

        let mut modifiers = Vec::new();
        let mut base_keys = Vec::new();

        for part in parts {
            match part {
                "Ctrl" | "Shift" | "Alt" => modifiers.push(part.to_string()),
                _ => base_keys.push(part.to_string()),
            }
        }

        // Sort modifiers in canonical order: Ctrl > Shift > Alt
        modifiers.sort_by(|a, b| {
            let order = |s: &str| match s {
                "Ctrl" => 0,
                "Shift" => 1,
                "Alt" => 2,
                _ => 3,
            };
            order(a).cmp(&order(b))
        });

        // Sort base keys alphabetically
        base_keys.sort();

        // Combine: modifiers first, then base keys
        modifiers.extend(base_keys);
        modifiers
    }

    /// Find a node for the given combo parts
    fn find_node(&self, parts: &[String]) -> Option<&TrieNode> {
        let mut node = &self.root;
        for part in parts {
            node = node.children.get(part)?;
        }
        Some(node)
    }

    /// Check if a binding exists for this exact combo
    pub fn has_binding(&self, parts: &[String]) -> bool {
        self.find_node(parts).map(|n| n.is_binding).unwrap_or(false)
    }

    /// Check if this combo is a leaf (no extensions possible)
    pub fn is_leaf(&self, parts: &[String]) -> bool {
        self.find_node(parts)
            .map(|n| n.children.is_empty())
            .unwrap_or(true) // Non-existent combos are considered leaves
    }

    /// Check if extensions exist for this combo prefix
    #[cfg(test)]
    pub fn has_extensions(&self, parts: &[String]) -> bool {
        self.find_node(parts)
            .map(|n| !n.children.is_empty())
            .unwrap_or(false)
    }

    /// Find the longest matching binding for the given parts
    /// Returns the parts that form the binding, or None if no binding matches
    pub fn find_best_match(&self, parts: &[String]) -> Option<Vec<String>> {
        let mut node = &self.root;
        let mut best_match: Option<Vec<String>> = None;
        let mut current_parts = Vec::new();

        for part in parts {
            if let Some(child) = node.children.get(part) {
                current_parts.push(part.clone());
                if child.is_binding {
                    best_match = Some(current_parts.clone());
                }
                node = child;
            } else {
                break;
            }
        }

        best_match
    }
}

/// Result of chord detection
#[derive(Debug, Clone)]
pub enum ChordResult {
    /// Trigger this combo immediately
    Trigger(String),
    /// Wait for more keys or timer
    Pending,
    /// No binding matches the current combo
    NoMatch,
}

/// Chord detector state
/// Tracks pressed keys and determines when to trigger combos
pub struct ChordDetector {
    /// Currently pressed keys (excluding modifiers which are tracked separately)
    pressed_base_keys: HashSet<String>,
    /// Currently pressed modifiers
    pressed_modifiers: HashSet<String>,
    /// Trie of known bindings
    trie: ComboTrie,
    /// When the last key was pressed (for timer logic)
    last_key_time: Instant,
    /// Chord window in milliseconds
    chord_window_ms: u32,
    /// The combo that was pending (waiting for timer)
    pending_combo: Option<Vec<String>>,
}

impl ChordDetector {
    pub fn new(chord_window_ms: u32) -> Self {
        Self {
            pressed_base_keys: HashSet::new(),
            pressed_modifiers: HashSet::new(),
            trie: ComboTrie::new(),
            last_key_time: Instant::now(),
            chord_window_ms,
            pending_combo: None,
        }
    }

    /// Update the Trie with new bindings
    pub fn set_bindings(&mut self, bindings: &[String]) {
        self.trie = ComboTrie::build_from_bindings(bindings);
        // Clear any pending state when bindings change
        self.pressed_base_keys.clear();
        self.pressed_modifiers.clear();
        self.pending_combo = None;
    }

    /// Set the chord window duration
    pub fn set_chord_window(&mut self, ms: u32) {
        self.chord_window_ms = ms;
    }

    /// Check if a key is a modifier
    fn is_modifier(key: &str) -> bool {
        matches!(
            key,
            "ControlLeft"
                | "ControlRight"
                | "ShiftLeft"
                | "ShiftRight"
                | "AltLeft"
                | "AltRight"
                | "MetaLeft"
                | "MetaRight"
        )
    }

    /// Build the current combo string from pressed keys
    fn build_current_combo(&self) -> Vec<String> {
        let mut parts = Vec::new();

        // Add modifiers in canonical order
        let has_ctrl = self
            .pressed_modifiers
            .iter()
            .any(|m| m == "ControlLeft" || m == "ControlRight");
        let has_shift = self
            .pressed_modifiers
            .iter()
            .any(|m| m == "ShiftLeft" || m == "ShiftRight");
        let has_alt = self
            .pressed_modifiers
            .iter()
            .any(|m| m == "AltLeft" || m == "AltRight");

        if has_ctrl {
            parts.push("Ctrl".to_string());
        }
        if has_shift {
            parts.push("Shift".to_string());
        }
        if has_alt {
            parts.push("Alt".to_string());
        }

        // Add base keys sorted alphabetically
        let mut base_keys: Vec<String> = self.pressed_base_keys.iter().cloned().collect();
        base_keys.sort();
        parts.extend(base_keys);

        parts
    }

    /// Convert combo parts back to a string
    fn combo_to_string(parts: &[String]) -> String {
        parts.join("+")
    }

    /// Handle a key press event
    /// Returns the result: Trigger immediately, Pending (wait for timer), or NoMatch
    pub fn on_key_press(&mut self, key: &str) -> ChordResult {
        // Track the key
        if Self::is_modifier(key) {
            self.pressed_modifiers.insert(key.to_string());
            // Modifiers alone don't trigger combos
            return ChordResult::Pending;
        } else {
            self.pressed_base_keys.insert(key.to_string());
        }

        self.last_key_time = Instant::now();

        // Build current combo
        let current_parts = self.build_current_combo();

        if current_parts.is_empty() {
            return ChordResult::NoMatch;
        }

        // Check if this exact combo is a leaf (no extensions)
        if self.trie.is_leaf(&current_parts) {
            // Check if it's a valid binding
            if self.trie.has_binding(&current_parts) {
                self.pending_combo = None;
                return ChordResult::Trigger(Self::combo_to_string(&current_parts));
            } else {
                // Check if there's any matching prefix
                if let Some(best) = self.trie.find_best_match(&current_parts) {
                    self.pending_combo = None;
                    return ChordResult::Trigger(Self::combo_to_string(&best));
                }
                return ChordResult::NoMatch;
            }
        }

        // Extensions exist - need to wait
        self.pending_combo = Some(current_parts);
        ChordResult::Pending
    }

    /// Handle a key release event
    pub fn on_key_release(&mut self, key: &str) {
        if Self::is_modifier(key) {
            self.pressed_modifiers.remove(key);
        } else {
            self.pressed_base_keys.remove(key);
        }
    }

    /// Check if the timer has expired and we should trigger the pending combo
    /// Returns Some(combo_string) if we should trigger, None otherwise
    pub fn check_timer(&mut self) -> Option<String> {
        if let Some(ref pending) = self.pending_combo {
            let elapsed = self.last_key_time.elapsed();
            if elapsed >= Duration::from_millis(self.chord_window_ms as u64) {
                // Timer expired - trigger the best match for the pending combo
                if let Some(best) = self.trie.find_best_match(pending) {
                    let combo = Self::combo_to_string(&best);
                    self.pending_combo = None;
                    return Some(combo);
                }
                // No valid binding found
                self.pending_combo = None;
            }
        }
        None
    }

    /// Clear all state (e.g., when profile changes)
    pub fn clear(&mut self) {
        self.pressed_base_keys.clear();
        self.pressed_modifiers.clear();
        self.pending_combo = None;
    }

    /// Check if we have a pending combo waiting for timer
    #[cfg(test)]
    pub fn has_pending(&self) -> bool {
        self.pending_combo.is_some()
    }
}

/// Thread-safe wrapper for ChordDetector
pub struct ChordDetectorHandle {
    inner: Arc<Mutex<ChordDetector>>,
}

impl Clone for ChordDetectorHandle {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl ChordDetectorHandle {
    pub fn new(chord_window_ms: u32) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ChordDetector::new(chord_window_ms))),
        }
    }

    pub fn set_bindings(&self, bindings: &[String]) {
        self.inner.lock().unwrap().set_bindings(bindings);
    }

    pub fn set_chord_window(&self, ms: u32) {
        self.inner.lock().unwrap().set_chord_window(ms);
    }

    pub fn on_key_press(&self, key: &str) -> ChordResult {
        self.inner.lock().unwrap().on_key_press(key)
    }

    pub fn on_key_release(&self, key: &str) {
        self.inner.lock().unwrap().on_key_release(key);
    }

    pub fn check_timer(&self) -> Option<String> {
        self.inner.lock().unwrap().check_timer()
    }

    pub fn clear(&self) {
        self.inner.lock().unwrap().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trie_single_keys() {
        let bindings = vec!["KeyA".to_string(), "KeyB".to_string()];
        let trie = ComboTrie::build_from_bindings(&bindings);

        assert!(trie.has_binding(&vec!["KeyA".to_string()]));
        assert!(trie.has_binding(&vec!["KeyB".to_string()]));
        assert!(!trie.has_binding(&vec!["KeyC".to_string()]));

        // Single keys with no extensions are leaves
        assert!(trie.is_leaf(&vec!["KeyA".to_string()]));
        assert!(trie.is_leaf(&vec!["KeyB".to_string()]));
    }

    #[test]
    fn test_trie_with_extensions() {
        // Note: Keys are sorted alphabetically when normalized
        // "KeyA+KeyB+KeyC" -> ["KeyA", "KeyB", "KeyC"]
        let bindings = vec![
            "KeyA".to_string(),
            "KeyA+KeyB".to_string(),
            "KeyA+KeyB+KeyC".to_string(),
        ];
        let trie = ComboTrie::build_from_bindings(&bindings);

        // KeyA has extensions (KeyA+KeyB), so not a leaf
        assert!(!trie.is_leaf(&vec!["KeyA".to_string()]));
        assert!(trie.has_extensions(&vec!["KeyA".to_string()]));

        // KeyA+KeyB has extension (KeyA+KeyB+KeyC), so not a leaf
        assert!(!trie.is_leaf(&vec!["KeyA".to_string(), "KeyB".to_string()]));

        // KeyA+KeyB+KeyC has no extensions, so it's a leaf
        assert!(trie.is_leaf(&vec![
            "KeyA".to_string(),
            "KeyB".to_string(),
            "KeyC".to_string()
        ]));
    }

    #[test]
    fn test_normalize_combo() {
        // Base keys should be sorted
        let parts = ComboTrie::normalize_combo("KeyZ+KeyA");
        assert_eq!(parts, vec!["KeyA", "KeyZ"]);

        // Modifiers come first
        let parts = ComboTrie::normalize_combo("KeyA+Ctrl");
        assert_eq!(parts, vec!["Ctrl", "KeyA"]);

        // Modifiers in canonical order
        let parts = ComboTrie::normalize_combo("Alt+KeyA+Shift+Ctrl");
        assert_eq!(parts, vec!["Ctrl", "Shift", "Alt", "KeyA"]);
    }

    #[test]
    fn test_chord_detector_immediate_trigger() {
        let bindings = vec!["KeyB".to_string()]; // B has no extensions
        let mut detector = ChordDetector::new(30);
        detector.set_bindings(&bindings);

        // Pressing B should trigger immediately (it's a leaf)
        match detector.on_key_press("KeyB") {
            ChordResult::Trigger(combo) => assert_eq!(combo, "KeyB"),
            _ => panic!("Expected immediate trigger"),
        }
    }

    #[test]
    fn test_chord_detector_pending() {
        let bindings = vec!["KeyA".to_string(), "KeyA+KeyZ".to_string()];
        let mut detector = ChordDetector::new(30);
        detector.set_bindings(&bindings);

        // Pressing A should be pending (extensions exist)
        match detector.on_key_press("KeyA") {
            ChordResult::Pending => {}
            _ => panic!("Expected pending"),
        }

        assert!(detector.has_pending());
    }

    #[test]
    fn test_chord_detector_multi_key_leaf() {
        let bindings = vec!["KeyA".to_string(), "KeyA+KeyZ".to_string()];
        let mut detector = ChordDetector::new(30);
        detector.set_bindings(&bindings);

        // Press A (pending)
        detector.on_key_press("KeyA");

        // Press Z - now A+Z is a leaf, should trigger
        match detector.on_key_press("KeyZ") {
            ChordResult::Trigger(combo) => assert_eq!(combo, "KeyA+KeyZ"),
            _ => panic!("Expected trigger for A+Z"),
        }
    }
}
