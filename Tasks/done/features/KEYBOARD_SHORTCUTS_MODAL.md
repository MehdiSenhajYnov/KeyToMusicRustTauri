# Modal d'aide raccourcis clavier

> **Statut:** Completed (2026-02-02)
> **Type:** Feature — Modal d'aide listant tous les raccourcis et interactions
> **Objectif:** Ajouter un bouton "?" dans le header ouvrant un modal qui liste tous les raccourcis clavier, shortcuts globaux configurables et interactions souris, avec contenu dynamique refletant la config actuelle

---

## Vue d'ensemble

L'app a 15+ raccourcis clavier et interactions souris repartis dans differents contextes (global, modals, grid, discovery, waveform). Aucun moyen de les decouvrir sauf en lisant le code. Les shortcuts globaux (Stop All, Auto-Momentum, Key Detection) sont configurables mais leur valeur actuelle n'est visible que dans Settings.

Le modal d'aide resout ca : un seul endroit pour tout voir, avec les raccourcis configurables affiches avec leur valeur actuelle depuis `settingsStore`.

---

## Design

### Bouton dans le Header

Ajouter un bouton "?" entre le slider volume et le bouton Settings dans `Header.tsx` :

```
┌─────────────────────────────────────────────────────────┐
│ KTM  KeyToMusic          Vol ━━●━━━ 75%   [?]  [⚙]    │
└─────────────────────────────────────────────────────────┘
```

- Icone : `?` dans un cercle (SVG inline ou texte avec border-radius)
- Style : meme pattern que le bouton Settings (`p-2 text-text-secondary hover:text-text-primary hover:bg-bg-hover rounded transition-colors`)
- `title="Keyboard Shortcuts"`

### Modal

Meme pattern que `SettingsModal` : backdrop `fixed inset-0 bg-black/60 z-50`, container `bg-bg-secondary border rounded-lg`, header fixe, contenu scrollable.

Taille : `w-[520px] max-h-[80vh]` (legerement plus large que Settings pour les 2 colonnes).

### Contenu : sections avec tableaux

```
┌──────────────────────────────────────────────────┐
│ Keyboard Shortcuts                            x  │
├──────────────────────────────────────────────────┤
│                                                  │
│ GLOBAL SHORTCUTS                                 │
│ ─────────────────────────────                    │
│ Stop All       Ctrl + Shift + S     ← config │
│ Toggle Auto-Mom.  (not set)            ← config │
│ Toggle Key Det.   (not set)            ← config │
│ Momentum Mod.     Shift + key          ← config │
│                                                  │
│ GENERAL                                          │
│ ─────────────────────────────                    │
│ Undo              Ctrl + Z                       │
│ Redo              Ctrl + Y                       │
│ Open Shortcuts    ?                              │
│                                                  │
│ KEY GRID                                         │
│ ─────────────────────────────                    │
│ Select all        Ctrl + A                       │
│ Multi-select      Ctrl + Click                   │
│ Range select      Shift + Click                  │
│ Deselect          Click selected key             │
│                                                  │
│ DISCOVERY                                        │
│ ─────────────────────────────                    │
│ Previous          ←                              │
│ Next              →                              │
│                                                  │
│ MODALS                                           │
│ ─────────────────────────────                    │
│ Close / Cancel    Escape                         │
│ Submit URL        Enter                          │
│                                                  │
│ MOUSE INTERACTIONS                               │
│ ─────────────────────────────                    │
│ Adjust sliders    Mouse wheel                    │
│ Set momentum      Drag on waveform               │
│ Resize panel      Drag divider bar               │
│ Add files         Drag & drop audio files        │
│                                                  │
│              Tip: ? to toggle this modal          │
└──────────────────────────────────────────────────┘
```

### Presentation des raccourcis

Chaque raccourci affiche en **2 colonnes** :
- **Gauche** : description (`text-text-secondary text-sm`)
- **Droite** : touches dans des `<kbd>` stylises

Style `<kbd>` :
```
bg-bg-tertiary text-text-primary text-xs font-mono px-1.5 py-0.5 rounded border border-border-color
```

Pour les combinaisons : `Ctrl` + `Shift` + `S` — chaque touche dans son propre `<kbd>`, separes par un `+` en `text-text-muted`.

Pour les raccourcis non configures : afficher `(not set)` en `text-text-muted italic`.

### Raccourci pour ouvrir le modal

- `?` (touche seule, pas Ctrl+?) — toggle le modal ouvert/ferme
- Aussi `F1` comme alternative classique
- Ne doit pas se declencher si un input texte est focus (meme guard que undo/redo)

---

## Implementation

### Section 1 — Composant KeyboardShortcutsModal

**Fichier a creer :** `src/components/common/KeyboardShortcutsModal.tsx`

**Props :**
```typescript
interface KeyboardShortcutsModalProps {
  onClose: () => void;
}
```

**Contenu dynamique :**
Le composant lit directement depuis `useSettingsStore` pour les raccourcis configurables :
- `config.StopAllShortcut` → affiche via `formatShortcut()` de `utils/keyMapping.ts`
- `config.autoMomentumShortcut` → idem, ou "(not set)" si tableau vide
- `config.keyDetectionShortcut` → idem
- `config.momentumModifier` → affiche "Shift + key", "Ctrl + key", "Alt + key", ou "Disabled"

**Detection plateforme :**
Pour Undo/Redo, afficher la bonne combinaison selon la plateforme :
```typescript
const isMac = navigator.platform.toUpperCase().indexOf("MAC") >= 0;
// Undo: isMac ? "Cmd + Z" : "Ctrl + Z"
// Redo: isMac ? "Cmd + Shift + Z" : "Ctrl + Y"
// Select All: isMac ? "Cmd + A" : "Ctrl + A"
```
Ce pattern existe deja dans `useUndoRedo.ts:26`.

**Structure du composant :**
```tsx
// Backdrop + container (meme pattern que SettingsModal.tsx:188-189)
<div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
  <div className="bg-bg-secondary border border-border-color rounded-lg w-[520px] max-h-[80vh] flex flex-col">
    {/* Header fixe */}
    {/* Contenu scrollable avec sections */}
  </div>
</div>
```

**Sections :**
Utiliser le meme `SectionHeader` que `SettingsModal.tsx:27-33` — extraire en composant partage ou dupliquer (petite fonction utilitaire de 6 lignes, duplication acceptable).

**Helper pour les lignes de raccourcis :**
```tsx
function ShortcutRow({ label, keys }: { label: string; keys: string[] | string }) {
  // label a gauche, <kbd> a droite
}
```

**Gestion Escape :**
```typescript
useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") onClose();
  };
  window.addEventListener("keydown", handleKeyDown);
  return () => window.removeEventListener("keydown", handleKeyDown);
}, [onClose]);
```

### Section 2 — Bouton "?" dans le Header

**Fichier a modifier :** `src/components/Layout/Header.tsx`

**Details :**

1. Ajouter une prop `onHelpClick: () => void` a l'interface `HeaderProps` (ligne 5-7)

2. Ajouter le bouton entre le volume et le settings (avant ligne 58) :
   ```tsx
   <button
     onClick={onHelpClick}
     className="p-2 text-text-secondary hover:text-text-primary hover:bg-bg-hover rounded transition-colors"
     title="Keyboard Shortcuts"
   >
     <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
       <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
         d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01" />
       <circle cx="12" cy="12" r="10" strokeWidth={2} />
     </svg>
   </button>
   ```

### Section 3 — State et raccourci dans App.tsx

**Fichier a modifier :** `src/App.tsx`

**Details :**

1. **State** : ajouter `showHelp` (meme pattern que `showSettings`, ligne 46) :
   ```typescript
   const [showHelp, setShowHelp] = useState(false);
   ```

2. **Lazy import** (meme pattern que SettingsModal, ligne 42) :
   ```typescript
   const KeyboardShortcutsModal = lazy(() =>
     import("./components/common/KeyboardShortcutsModal")
       .then(m => ({ default: m.KeyboardShortcutsModal }))
   );
   ```

3. **Passer la prop au Header** (ligne 167) :
   ```tsx
   <Header
     onSettingsClick={() => setShowSettings(true)}
     onHelpClick={() => setShowHelp(true)}
   />
   ```

4. **Rendu conditionnel du modal** (apres le SettingsModal, ligne 175) :
   ```tsx
   {showHelp && <KeyboardShortcutsModal onClose={() => setShowHelp(false)} />}
   ```

5. **Raccourci `?` et `F1`** : ajouter un listener global dans un `useEffect` :
   ```typescript
   useEffect(() => {
     const handleKeyDown = (e: KeyboardEvent) => {
       // Ne pas intercepter si un input texte est focus
       const tag = (document.activeElement as HTMLElement)?.tagName;
       if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

       if (e.key === "?" || e.key === "F1") {
         e.preventDefault();
         setShowHelp((prev) => !prev);  // toggle
       }
     };
     window.addEventListener("keydown", handleKeyDown);
     return () => window.removeEventListener("keydown", handleKeyDown);
   }, []);
   ```

---

## Catalogue complet des raccourcis a afficher

### Global Shortcuts (dynamiques depuis config)

| Action | Source | Defaut |
|--------|--------|--------|
| Stop All | `config.StopAllShortcut` → `formatShortcut()` | Ctrl + Shift + S |
| Toggle Auto-Momentum | `config.autoMomentumShortcut` → `formatShortcut()` ou "(not set)" | (not set) |
| Toggle Key Detection | `config.keyDetectionShortcut` → `formatShortcut()` ou "(not set)" | (not set) |
| Momentum Modifier | `config.momentumModifier` → "Shift/Ctrl/Alt + key" ou "Disabled" | Shift + key |

### General (hardcodes)

| Action | Raccourci | Defini dans |
|--------|-----------|-------------|
| Undo | Ctrl+Z / Cmd+Z | `useUndoRedo.ts:31-36` |
| Redo | Ctrl+Y / Cmd+Shift+Z | `useUndoRedo.ts:37-42` |
| Keyboard Shortcuts | ? ou F1 | `App.tsx` (nouveau) |

### Key Grid (hardcodes)

| Action | Raccourci | Defini dans |
|--------|-----------|-------------|
| Select all | Ctrl+A / Cmd+A | `KeyGrid.tsx:62-67` |
| Multi-select | Ctrl+Click | `MainContent.tsx:128-136` |
| Range select | Shift+Click | `MainContent.tsx:137-147` |
| Deselect | Click on selected key | `MainContent.tsx:148-157` |

### Discovery Panel (hardcodes)

| Action | Raccourci | Defini dans |
|--------|-----------|-------------|
| Previous suggestion | Arrow Left | `DiscoveryPanel.tsx:457-460` |
| Next suggestion | Arrow Right | `DiscoveryPanel.tsx:461-464` |

### Modals (hardcodes)

| Action | Raccourci | Defini dans |
|--------|-----------|-------------|
| Close / Cancel capture | Escape | Tous les modals |
| Submit YouTube URL | Enter | `AddSoundModal.tsx:832-834` |

### Interactions souris (hardcodes)

| Action | Interaction | Defini dans |
|--------|-------------|-------------|
| Adjust sliders | Mouse wheel on any slider | `useWheelSlider.ts:11-37` |
| Set momentum | Drag on waveform | `WaveformDisplay.tsx:67-93` |
| Resize details panel | Drag divider bar | `MainContent.tsx:47-86` |
| Add audio files | Drag & drop onto main area | `MainContent.tsx:89-111` |

---

## Fichiers a creer

| Fichier | Description |
|---------|-------------|
| `src/components/common/KeyboardShortcutsModal.tsx` | Modal avec toutes les sections de raccourcis, contenu dynamique depuis settingsStore |

## Fichiers a modifier

| Fichier | Modification |
|---------|-------------|
| `src/components/Layout/Header.tsx` | Ajouter prop `onHelpClick`, bouton "?" avec icone SVG |
| `src/App.tsx` | State `showHelp`, lazy import du modal, prop `onHelpClick` au Header, raccourci `?`/`F1` global, rendu conditionnel |

---

## Notes

- **Pas de nouvelle dependance** : tout est du React/Tailwind standard
- **Code splitting** : le modal est lazy-loaded (meme pattern que SettingsModal) — zero impact sur le startup
- **Coherence** : meme backdrop, meme container, meme `SectionHeader`, meme gestion Escape que les modals existants
- **Dynamique** : les raccourcis configurables se mettent a jour automatiquement via `useSettingsStore` — changer Stop All dans Settings se reflete instantanement dans le modal d'aide
- **Plateforme** : les raccourcis affichent Cmd sur macOS, Ctrl sur Windows/Linux (meme detection que `useUndoRedo.ts:26`)
