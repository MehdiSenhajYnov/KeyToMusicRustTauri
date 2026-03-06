# Phase 8.2 - Combined Key Shortcuts & UI Refactor

## Recap des discussions (2026-01-25)

Ce document capture toutes les décisions et détails techniques discutés pour éviter toute perte d'information.

---

## 1. Problèmes identifiés

### 1.1 Alignement icônes ProfileSelector
- **Problème** : Les icônes duplication (⎘) et suppression (x) n'étaient pas alignées
- **Solution** : Remplacé par des SVG uniformes avec dimensions fixes (w-5 h-5, icônes w-3.5 h-3.5)
- **Status** : ✅ Corrigé

### 1.2 Numpad + Shift ne fonctionne pas pour le momentum
- **Problème** : Shift+Numpad4 ne déclenche pas le momentum
- **Cause** : Comportement hardware/OS standard. Quand NumLock est ON et Shift est pressé, le système envoie la touche alternative (ArrowLeft, End, etc.) au lieu de "Shift+Numpad4"
- **Status** : ✅ RÉSOLU
- **Solution** : Momentum modifier configurable (Phase 8.5) - dropdown Shift/Ctrl/Alt/Désactivé dans Settings > Key Detection

### 1.3 Combined Key Shortcuts pas visible sur le frontend
- **Problème** : Le backend envoie les codes combinés (Ctrl+KeyA) mais l'UI ne permet pas de les assigner ni de les afficher
- **Status** : 🔄 À implémenter (UI refactor nécessaire)

---

## 2. Nouvelle architecture proposée

### 2.1 Système de capture de touches unifié

Actuellement, deux systèmes différents :
- **Global shortcuts (Settings)** : Capture au clic avec support modifiers ✓
- **Assignment de sons (AddSoundModal)** : Input texte "aze", pas de modifiers ✗

**Proposition** : Unifier avec le même pattern de capture partout.

### 2.2 Refonte UI AddSoundModal - Key Assignment

**Avant (actuel)** :
```
Keys: [aze________]  ← tape tout d'un coup
```

**Après (proposé)** :
```
Keys:
┌────────────────────────────────────────┐
│ 1. [Click to capture]  →  [Ctrl+A] [×] │
│ 2. [Click to capture]  →  [Z]      [×] │
│ 3. [+ Add key]                         │
└────────────────────────────────────────┘

Sounds (5):                  Assigned to:
├─ epic_battle.mp3           → Ctrl+A (key 1)
├─ calm_ambient.mp3          → Z (key 2)
├─ victory_theme.mp3         → Ctrl+A (cycle)
├─ tension.mp3               → Z (cycle)
└─ finale.mp3                → Ctrl+A (cycle)
```

**Comportement** :
- Par défaut : 1 slot de key vide
- Chaque slot = bouton de capture (comme dans Settings)
- Cliquer → mode capture → appuyer sur touche(s) → enregistré
- Support complet des modifiers : Ctrl+A, Shift+F1, Alt+Z
- Bouton "+" apparaît si `nombre de keys < nombre de sons`
- Bouton "×" pour supprimer un slot
- Preview du cycling en temps réel en dessous
- Si 1 seule key pour N sons → tous assignés à cette key

### 2.3 Format de stockage des key codes

**Format string avec modifiers** (déjà implémenté backend) :
```
"Ctrl+KeyA"
"Shift+F1"
"Ctrl+Shift+KeyZ"
"KeyA"  (sans modifier)
```

**Ordre des modifiers** (consistant backend/frontend) :
```
Ctrl > Shift > Alt > Key
```

Exemples :
- `Ctrl+Shift+KeyA` ✓
- `Shift+Ctrl+KeyA` ✗ (mauvais ordre)

---

## 3. Multi-key chords (A+Z simultanés) - Phase 8.4

### 3.1 Concept
Permettre des combinaisons de touches non-modifier pressées ensemble, comme un accord de piano :
- `KeyA+KeyZ` (A et Z pressés en même temps)
- `Ctrl+KeyA+KeyZ` (Ctrl + A + Z)

### 3.2 Avantage : Explosion combinatoire
| Type | Combinaisons (~50 touches) |
|------|----------------------------|
| 1 touche | 50 |
| 2 touches | 1,225 |
| 3 touches | 19,600 |
| + Modifiers (×8) | ×8 pour chaque |

### 3.3 Défi technique : Détection "simultanée"
Même si l'utilisateur appuie "en même temps", les événements arrivent séquentiellement :
```
t=0ms    KeyA down
t=3ms    KeyZ down    ← "simultané" mais séquentiel
```

### 3.4 Solution : Système de combo (style jeux de combat)

Inspiré des jeux de combat (Street Fighter, Tekken), on utilise un **arbre préfixe (Trie)** pour détecter les combos de manière optimale.

**Principe :**
- Trigger immédiat si le combo actuel est une **feuille** (pas d'extensions possibles)
- Sinon attendre timer ou prochaine touche

**Exemple avec bindings : A, A+Z, A+Z+E**

```
Structure Trie:
Root
├── A (binding exists)
│   ├── Z (binding exists)
│   │   └── E (binding exists, FEUILLE)
│   └── E (binding exists, FEUILLE)
└── B (binding exists, FEUILLE)
```

**Scénario 1 : User presse A puis Z puis E**
```
t=0ms    A pressé
         → Combos possibles: A, A+Z, A+Z+E
         → Timer démarre (A existe mais extensions possibles)

t=10ms   Z pressé
         → Combo actuel: A+Z
         → Combos possibles: A+Z, A+Z+E
         → Timer continue (A+Z existe mais A+Z+E possible)

t=20ms   E pressé
         → Combo actuel: A+Z+E
         → C'est une FEUILLE (pas d'extension A+Z+E+*)
         → TRIGGER IMMÉDIAT "A+Z+E" ✓
```

**Scénario 2 : User presse A puis Z (sans E)**
```
t=0ms    A pressé → Timer démarre
t=10ms   Z pressé → Combo = A+Z, timer continue
t=40ms   Timer expire (30ms après dernier input)
         → TRIGGER "A+Z" (meilleur match) ✓
```

**Scénario 3 : User presse A seul**
```
t=0ms    A pressé → Timer démarre
t=30ms   Timer expire
         → TRIGGER "A" ✓
```

**Scénario 4 : User presse B (pas d'extensions)**
```
t=0ms    B pressé
         → B est une FEUILLE (pas de B+* dans le profil)
         → TRIGGER IMMÉDIAT "B" (0ms latence) ✓
```

### 3.5 Optimisation : Latence conditionnelle

| Situation | Latence |
|-----------|---------|
| Touche sans extension possible (feuille) | 0ms |
| Touche avec extensions, combo complet atteint | 0ms |
| Touche avec extensions, attente timer | 30-50ms (configurable) |

**Règle :** Le délai ne s'applique QUE si des extensions existent pour le combo actuel.

### 3.6 Format des combos

**Ordre canonique :** Modifiers d'abord (Ctrl > Shift > Alt), puis base keys triées alphabétiquement.

```
Ctrl+Shift+KeyA+KeyZ  ← correct
KeyZ+KeyA             ← incorrect, sera normalisé en KeyA+KeyZ
```

### 3.7 Configuration

Nouveau paramètre dans `config.json` :
```json
{
  "chordWindowMs": 30  // 30-100ms, configurable dans Settings
}
```

### 3.8 Implémentation Backend (detector.rs)

```rust
struct ChordDetector {
    trie: ComboTrie,                    // Arbre des combos existants
    current_combo: Vec<String>,         // Touches actuellement pressées
    timer_handle: Option<TimerHandle>,  // Timer en cours
    chord_window_ms: u32,               // Fenêtre configurable
}

impl ChordDetector {
    fn on_key_press(&mut self, key: String) {
        self.current_combo.push(key);
        self.current_combo.sort(); // Ordre alphabétique

        let node = self.trie.find(&self.current_combo);

        if node.is_leaf() {
            // Pas d'extensions possibles → trigger immédiat
            self.trigger_combo();
        } else {
            // Extensions possibles → reset timer
            self.reset_timer();
        }
    }

    fn on_timer_expire(&mut self) {
        // Timer expiré → trigger le meilleur match actuel
        self.trigger_combo();
    }
}
```

### 3.9 Implémentation Frontend

**KeyCaptureSlot :** Déjà supporte multi-key via `pressedKeysRef`.

**Affichage :** `keyCodeToDisplay("KeyA+KeyZ")` → "A+Z"

### 3.10 Status
✅ **Phase 8.4** - IMPLÉMENTÉ

**Fichiers créés/modifiés:**
- `src-tauri/src/keys/chord.rs` - ComboTrie et ChordDetector
- `src-tauri/src/keys/detector.rs` - Intégration du ChordDetector
- `src-tauri/src/types.rs` - Ajout de `chord_window_ms`
- `src-tauri/src/commands.rs` - Commande `set_profile_bindings`
- `src/utils/keyMapping.ts` - Fonctions `normalizeCombo`, `isMultiKeyChord`, mise à jour de `buildComboFromPressedKeys`
- `src/stores/settingsStore.ts` - Ajout de `setChordWindowMs`
- `src/components/Settings/SettingsModal.tsx` - Slider pour Chord Window
- `src/App.tsx` - Sync des bindings au backend

---

## 4. Modificateur momentum configurable - DÉCIDÉ

### 4.1 Design final
Simple dropdown dans Settings avec 4 options :
- **Shift** (défaut) - comportement actuel
- **Ctrl** - fonctionne avec Numpad
- **Alt** - fonctionne avec Numpad
- **Désactivé** - momentum uniquement via Auto-Momentum toggle

### 4.2 Décisions clés
1. **Pas de touche custom** (comme "A") - trop de complexity (latence, edge cases)
2. **Pas de per-binding** - mémoire musculaire impossible, UX cauchemar
3. **Règle simple** : le match exact a priorité
   - Si "Ctrl+A" est assigné → se déclenche normalement
   - Si seulement "A" est assigné et Ctrl=momentum → "A" se déclenche avec momentum

### 4.3 Avantages
- Résout le problème Numpad+Shift
- Simple à comprendre (un seul dropdown)
- Pas de confusion pour les utilisateurs qui veulent juste "que ça marche"

### 4.4 Implémentation
- `config.momentumModifier: "Shift" | "Ctrl" | "Alt" | "None"`
- Dropdown dans Settings > Key Detection
- Backend : `detector.rs` vérifie le modifier configuré
- Frontend : `useKeyDetection.ts` vérifie le modifier correspondant

### 4.5 Détection de conflits

**Problème identifié:** Si l'utilisateur a un son sur "A", le momentum modifier sur "Alt", et le shortcut Auto-Momentum sur "Alt+A", appuyer sur Alt+A déclenche le shortcut au lieu du son avec momentum.

**Solution:** Warnings bidirectionnels dans Settings :
1. **À la modification du momentum modifier:** Vérifie si des shortcuts existants utilisent ce modifier + une touche bindée
2. **À la configuration d'un shortcut:** Vérifie si le shortcut utilise le momentum modifier + une touche bindée

**Messages toast:**
- "Warning: Auto-Momentum shortcut(s) use Alt + bound keys. They will override momentum."
- "Warning: This shortcut uses Alt + a bound key. It will override momentum for that key."

**Icônes warning persistantes:**
- À côté des shortcuts en conflit dans Settings (avec tooltip explicatif)
- À côté du dropdown Momentum Modifier si des conflits existent
- Sur les touches KeyGrid affectées (rappel visuel après fermeture des Settings)

### 4.6 Status
✅ **IMPLÉMENTÉ** (2026-01-25)

**Fichiers modifiés:**
- `src/types/index.ts` - Type `MomentumModifier`
- `src/stores/settingsStore.ts` - Action `setMomentumModifier()`
- `src-tauri/src/types.rs` - Enum `MomentumModifier` avec Default
- `src/hooks/useKeyDetection.ts` - Fonction `hasMomentumModifier()`
- `src/components/Settings/SettingsModal.tsx` - Dropdown + sections + warnings avec tooltips
- `src/components/Keys/KeyGrid.tsx` - Warning icons sur les touches en conflit
- `src/components/common/WarningTooltip.tsx` - Composant réutilisable (nouveau)
- `src/utils/keyMapping.ts` - Fonctions utilitaires pour détection de conflits

---

## 5. Plan d'implémentation

### Phase 0 : Validation des raccourcis réservés (Pré-requis)

Avant d'implémenter la capture, on doit bloquer les raccourcis déjà utilisés.

**Raccourcis réservés (à bloquer) :**

| Catégorie | Raccourcis | Raison |
|-----------|------------|--------|
| App (Undo/Redo) | `Ctrl+Z`, `Ctrl+Y`, `Cmd+Z`, `Cmd+Shift+Z` | Système undo/redo |
| App (Global shortcuts) | `config.StopAllShortcut` | Stop All |
| App (Global shortcuts) | `config.autoMomentumShortcut` | Toggle Auto-Momentum |
| App (Global shortcuts) | `config.keyDetectionShortcut` | Toggle Key Detection |
| OS (Système) | `Ctrl+C`, `Ctrl+V`, `Ctrl+X` | Copy/Paste/Cut |
| OS (Système) | `Ctrl+A`, `Ctrl+S`, `Ctrl+W`, `Ctrl+Q`, `Ctrl+N`, `Ctrl+T` | Actions système |
| OS (Système) | `Alt+F4` | Fermer fenêtre |
| OS (Warning) | `Ctrl+1-9` | Tabs navigateur (warning, pas blocage) |
| OS (Warning) | `Alt+lettre` | Menus Windows (warning, pas blocage) |

**Fonction de validation étendue (`keyMapping.ts`) :**

```typescript
interface ShortcutConflict {
  type: 'error' | 'warning';
  message: string;
  conflictWith: string;  // "Undo", "Stop All", "Copy", etc.
}

export function checkShortcutConflicts(
  combo: string,
  config: AppConfig
): ShortcutConflict | null {
  // 1. Check app undo/redo (hardcoded)
  const undoRedo: Record<string, string> = {
    "Ctrl+KeyZ": "Undo",
    "Ctrl+KeyY": "Redo",
  };
  if (undoRedo[combo]) {
    return {
      type: 'error',
      message: `Already used for ${undoRedo[combo]}`,
      conflictWith: undoRedo[combo]
    };
  }

  // 2. Check user-configured global shortcuts
  const configShortcuts = [
    { keys: config.StopAllShortcut, name: "Stop All" },
    { keys: config.autoMomentumShortcut, name: "Auto-Momentum Toggle" },
    { keys: config.keyDetectionShortcut, name: "Key Detection Toggle" },
  ];

  for (const shortcut of configShortcuts) {
    if (shortcut.keys.length > 0) {
      const configCombo = buildKeyCombo(shortcut.keys);
      if (configCombo === combo) {
        return {
          type: 'error',
          message: `Already used for ${shortcut.name}`,
          conflictWith: shortcut.name
        };
      }
    }
  }

  // 3. Check system shortcuts
  const systemShortcuts: Record<string, string> = {
    "Ctrl+KeyC": "Copy",
    "Ctrl+KeyV": "Paste",
    "Ctrl+KeyX": "Cut",
    "Ctrl+KeyA": "Select All",
    "Ctrl+KeyS": "Save",
    "Ctrl+KeyW": "Close Window",
    "Ctrl+KeyQ": "Quit App",
    "Ctrl+KeyN": "New Window",
    "Ctrl+KeyT": "New Tab",
    "Alt+F4": "Close Window",
  };

  if (systemShortcuts[combo]) {
    return {
      type: 'error',
      message: `System shortcut for ${systemShortcuts[combo]}`,
      conflictWith: systemShortcuts[combo]
    };
  }

  // 4. Warnings (pas de blocage, juste info)
  const { baseKey, ctrl, alt } = parseKeyCombo(combo);

  if (ctrl && /^Digit[1-9]$/.test(baseKey)) {
    return {
      type: 'warning',
      message: "May conflict with browser tab switching",
      conflictWith: "Browser tabs"
    };
  }

  if (alt && /^Key[A-Z]$/.test(baseKey)) {
    return {
      type: 'warning',
      message: "May conflict with menu access on Windows",
      conflictWith: "Windows menus"
    };
  }

  return null;
}
```

**UI feedback dans KeyCaptureSlot :**

```tsx
// Après capture d'une touche
const conflict = checkShortcutConflicts(capturedCombo, config);

if (conflict?.type === 'error') {
  // Afficher message d'erreur, ne pas accepter la touche
  showError(`Cannot use ${displayCombo}: ${conflict.message}`);
  return; // Ne pas enregistrer
}

if (conflict?.type === 'warning') {
  // Afficher warning mais permettre quand même
  showWarning(`Warning: ${conflict.message}`);
  // Continuer et enregistrer
}
```

**Messages UI exemples :**
- Error: `"Ctrl+Z is already used for Undo"`
- Error: `"Ctrl+Shift+S is already used for Stop All"`
- Error: `"Ctrl+C is a system shortcut for Copy"`
- Warning: `"Ctrl+1 may conflict with browser tab switching"`

### Phase 1 : UI Refactor AddSoundModal (Priorité haute)
1. [ ] Implémenter `checkShortcutConflicts()` étendue (avec config)
2. [ ] Créer composant `KeyCaptureSlot` réutilisable
3. [ ] Remplacer input texte par liste de slots dans AddSoundModal
4. [ ] Implémenter logique de capture avec modifiers
5. [ ] Afficher preview du cycling
6. [ ] Tester avec Ctrl+A, Shift+F1, etc.
7. [ ] Afficher erreurs/warnings pour raccourcis réservés

### Phase 2 : Affichage KeyGrid
1. [ ] Mettre à jour `keyCodeToDisplay()` pour afficher "Ctrl+A" au lieu de "KeyA"
2. [ ] Gérer les noms plus longs dans la grille (truncate ou wrap)

### Phase 3 : Backend adjustments
1. [ ] Vérifier que le backend envoie correctement les combined codes (déjà fait)
2. [ ] S'assurer du fallback : si "Ctrl+A" n'existe pas mais "A" existe, déclencher "A" avec momentum si Shift était dans le combo

### Phase 4 : Momentum modifier configurable ✅ IMPLÉMENTÉ
1. [x] Ajouter champ `momentumModifier` dans types et settingsStore
2. [x] UI dropdown dans Settings (Shift/Ctrl/Alt/Désactivé)
3. [x] Backend: `types.rs` - enum MomentumModifier avec serde
4. [x] Frontend: `useKeyDetection.ts` - fonction `hasMomentumModifier()`
5. [x] Persistance via `updateConfig` (pas de commande séparée nécessaire)

> Voir section 4 pour le design complet.

### Phase 5 (Future) : Multi-key chords
1. [ ] Implémenter fenêtre de détection 30ms
2. [ ] Tri alphabétique des touches pour consistance
3. [ ] Résolution de conflits
4. [ ] UI pour capturer multi-key

---

## 6. Fichiers à modifier

### Frontend
- `src/components/Sounds/AddSoundModal.tsx` - Refonte majeure
- `src/components/Keys/KeyCaptureSlot.tsx` - Nouveau composant
- `src/components/Keys/KeyGrid.tsx` - Affichage combined keys
- `src/utils/keyMapping.ts` - Fonctions helper (déjà partiellement fait)
- `src/hooks/useKeyDetection.ts` - Déjà fait, vérifier fallback

### Backend (si nécessaire)
- `src-tauri/src/keys/detector.rs` - Déjà fait pour combined codes
- Potentiellement ajouter config momentum modifier

### Documentation
- `CLAUDE.md`
- `KeyToMusic_Technical_Specification.md`
- `TASKS.md`

---

## 7. Questions ouvertes

1. **Momentum avec combined keys** : Si user assigne `Ctrl+A`, est-ce que `Ctrl+Shift+A` déclenche le momentum ? Ou faut-il un binding explicite ?
   - Proposition : Oui, ajouter Shift à un binding existant déclenche momentum

2. **Migration profils existants** : Les profils avec `KeyA` continuent de fonctionner, Shift+A = momentum
   - Proposition : Oui, backward compatible

3. **Limite de modifiers** : Autoriser Ctrl+Shift+Alt+Key ou limiter à 2 modifiers ?
   - Proposition : Pas de limite artificielle, laisser le user décider

4. **Affichage KeyGrid** : Les noms "Ctrl+Shift+A" sont plus longs. Truncate ? Smaller font ? Wrap ?
   - À tester visuellement

---

## 8. Références

- Fichier actuel AddSoundModal : `src/components/Sounds/AddSoundModal.tsx`
- Système de capture Settings : `src/components/Settings/SettingsModal.tsx`
- Backend detector : `src-tauri/src/keys/detector.rs`
- Key mapping utils : `src/utils/keyMapping.ts`
