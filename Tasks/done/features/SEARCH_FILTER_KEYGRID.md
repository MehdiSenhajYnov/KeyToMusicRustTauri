# Recherche et Filtre de Sons dans le KeyGrid

> **Statut:** Completed (2026-02-02)
> **Type:** Feature — Barre de recherche/filtre compacte au-dessus du KeyGrid
> **Objectif:** Permettre de filtrer instantanement les bindings par texte libre, track, loop mode ou statut, avec une UI minimale inspiree du pattern Spotlight/Command Bar

---

## Vue d'ensemble

### Le probleme

Quand un profil a 30+ bindings, retrouver un son specifique oblige a scanner visuellement tout le KeyGrid. Pas de moyen de chercher par nom, par track, ou de filtrer les sons en cours de lecture. Plus le profil grandit, plus c'est penible.

### La solution : Spotlight-style Filter Bar

Inspiree de macOS Spotlight, Raycast et des best practices UX 2025-2026 :

- **Une seule barre de texte** qui fait tout : recherche textuelle + filtres inline via prefixes (`t:`, `l:`, `s:`)
- **Chips de filtres actifs** (pills) apparaissant sous la barre quand des filtres sont actifs
- **Zero espace gaspille** : la barre remplace le titre "Key Assignments" (qui se deplace dans la barre) et n'ajoute qu'une seule ligne de hauteur
- **Activation par raccourci** : `Ctrl+F` ouvre/focus la barre, `Escape` la ferme et reset
- **Filtrage instantane** : pas de debounce (le filtrage est purement frontend, < 1ms pour 200 bindings)
- **Compteur de resultats** : "12/45" affiche combien de bindings matchent

### Principes UX appliques

| Principe | Application |
|----------|-------------|
| **Compact & non-intrusif** | La barre fait 32px de haut, s'integre dans le header existant du KeyGrid |
| **Progressive disclosure** | Filtres avances (track, loop, statut) caches derriere des prefixes, pas de dropdowns visibles |
| **Feedback immediat** | Compteur de resultats en temps reel, bindings non-matchees grises (pas supprimees) |
| **Keyboard-first** | `Ctrl+F` focus, `Escape` ferme, tout navigable au clavier |
| **Chips pour filtres actifs** | Pills removables sous la barre quand un filtre est actif (pattern standard 2025) |
| **Coherence** | Memes classes CSS que les inputs existants (`bg-bg-tertiary`, `border-border-color`, etc.) |

### References UX

- [Apple HIG — Search Fields](https://developer.apple.com/design/human-interface-guidelines/search-fields) : champ unique, resultats en temps reel, raccourci clavier
- [Spotlight macOS Tahoe](https://www.tech2geek.net/how-to-use-new-spotlight-filters-in-macos-26-for-faster-smarter-search/) : filtres inline via prefixes (ex: `/pdf`, `/icloud`)
- [Filter UI Patterns 2025](https://bricxlabs.com/blogs/universal-search-and-filters-ui) : chips/pills, feedback immediat, compteur
- [Search UX Best Practices 2026](https://www.designrush.com/best-designs/websites/trends/search-ux-best-practices) : keyboard navigation, dark theme, active filter visibility

---

## Design UI

### Layout : Avant / Apres

**Avant** (actuel, `MainContent.tsx:202-214`) :
```
┌──────────────────────────────────────────────┐
│ Key Assignments                  [+ Add Sound]│
├──────────────────────────────────────────────┤
│ [A] Son 1   [B] Son 2   [C] Son 3   ...     │
```

**Apres** :
```
┌──────────────────────────────────────────────┐
│ Key Assignments  [🔍 Search keys... ] [+ Add] │
│                  ╰─ t:OST ✕ ─╯ 12/45         │
├──────────────────────────────────────────────┤
│ [A] Son 1   [B] Son 2   ░░ Son 3 ░░  ...    │
│                          (grise=non-match)    │
```

### Composant : SearchFilterBar

```
┌─────────────────────────────────────────────────┐
│ ╔═══════════════════════════════════════════╗    │
│ ║ 🔍  Search keys, sounds, tracks...   ✕  ║    │
│ ╚═══════════════════════════════════════════╝    │
│  [t:OST ✕] [l:sequential ✕]         12/45      │
└─────────────────────────────────────────────────┘
```

- **Icone loupe** a gauche (SVG inline, `text-text-muted`, 14px)
- **Input** : `flex-1`, placeholder dynamique, `text-sm`
- **Bouton clear** (✕) : visible uniquement si texte non-vide
- **Chips** : sous l'input, `bg-accent-primary/20 text-accent-primary text-xs rounded-full px-2 py-0.5` avec ✕
- **Compteur** : `text-text-muted text-xs` a droite des chips, format "N/total"

### Syntaxe des filtres inline

| Prefixe | Filtre | Exemple |
|---------|--------|---------|
| *(aucun)* | Texte libre (nom son, nom binding, touche) | `battle` |
| `t:` | Par nom de track | `t:OST`, `t:amb` |
| `l:` | Par loop mode | `l:sequential`, `l:off` |
| `s:` | Par statut | `s:playing`, `s:stopped` |

**Parsing** : a chaque changement de l'input, extraire les tokens :
1. Split par espaces
2. Pour chaque token, verifier s'il commence par `t:`, `l:`, ou `s:`
3. Si oui, l'extraire comme filtre structure
4. Les tokens restants = recherche textuelle (match sur nom du son, nom du binding, keyCode display)

Quand un prefixe est detecte, il se transforme en chip et disparait de l'input texte.

### Comportement de filtrage dans le KeyGrid

- Les bindings non-matchees ne disparaissent **pas** — elles sont **grisees** (`opacity-30 pointer-events-none`)
- Cela preserve le layout spatial (l'utilisateur garde ses reperes visuels)
- Les bindings matchees restent normales et interactives
- Si aucun binding ne matche : message "No matching keys" centre dans le grid

### Logique de match

Un binding `kb` matche si **tous** les filtres actifs sont satisfaits (AND) :

```
matchesText = searchText vide OU :
  - keyCodeToDisplay(kb.keyCode) contient le texte (case-insensitive)
  - OU kb.name contient le texte
  - OU un son dans kb.soundIds a un name qui contient le texte

matchesTrack = pas de filtre track OU :
  - le track de kb a un name qui contient le filtre (case-insensitive)

matchesLoop = pas de filtre loop OU :
  - kb.loopMode === le filtre

matchesStatus = pas de filtre statut OU :
  - "playing" → au moins un soundId dans playingSoundIds
  - "stopped" → aucun soundId dans playingSoundIds

finalMatch = matchesText AND matchesTrack AND matchesLoop AND matchesStatus
```

---

## Implementation

### Section 1 — Composant SearchFilterBar

**Fichier a creer:** `src/components/common/SearchFilterBar.tsx`

**Props :**
```typescript
interface SearchFilterBarProps {
  totalCount: number;
  matchCount: number;
  onFilterChange: (filter: KeyGridFilter) => void;
  tracks: Track[];
}
```

**State interne :**
```typescript
const [inputValue, setInputValue] = useState("");
const [chips, setChips] = useState<FilterChip[]>([]);
// FilterChip = { type: "track" | "loop" | "status", value: string, label: string }
```

**Details :**
- Rendre un `<div>` contenant l'input + icone + clear + chips row + compteur
- Sur chaque `onChange` de l'input : parser les tokens, detecter les prefixes, creer des chips si besoin
- Sur suppression d'un chip : retirer du tableau et re-calculer le filtre
- Sur `Escape` keydown dans l'input : vider tout et `blur()`
- Le composant expose le filtre combine via `onFilterChange` a chaque changement
- **Pas de debounce** : le calcul est synchrone et trivial

**Gestion du raccourci `Ctrl+F` :**
- Le raccourci sera gere dans `MainContent.tsx` (pas dans le composant) car il faut prevenir le Ctrl+F natif du navigateur
- `MainContent` appelle `inputRef.current?.focus()` via une ref exposee par le composant (`forwardRef`)

### Section 2 — Type KeyGridFilter

**Fichier a modifier:** `src/types/index.ts`

**Ajouter :**
```typescript
export interface KeyGridFilter {
  searchText: string;
  trackName: string | null;    // partial match, case-insensitive
  loopMode: LoopMode | null;
  status: "playing" | "stopped" | null;
}
```

### Section 3 — Integration dans MainContent

**Fichier a modifier:** `src/components/Layout/MainContent.tsx`

**Details :**

1. **State** : ajouter `filter: KeyGridFilter | null` (null = pas de filtre actif)

2. **Raccourci Ctrl+F** : dans un `useEffect`, ecouter `keydown` global :
   - `Ctrl+F` / `Cmd+F` → `e.preventDefault()`, focus l'input de la SearchFilterBar
   - Necessite une ref vers le composant SearchFilterBar

3. **Remplacer le header** (lignes 202-214) : integrer SearchFilterBar dans la meme ligne que "Key Assignments" et "+ Add Sound" :
   ```tsx
   <div className="flex items-center justify-between gap-2">
     <h2 className="text-text-primary text-sm font-semibold whitespace-nowrap">
       Key Assignments
     </h2>
     <SearchFilterBar
       ref={searchBarRef}
       totalCount={currentProfile.keyBindings.length}
       matchCount={matchingKeys.size}
       onFilterChange={setFilter}
       tracks={currentProfile.tracks}
     />
     {currentProfile.tracks.length > 0 && (
       <button onClick={() => setShowAddSound(true)} ...>
         + Add Sound
       </button>
     )}
   </div>
   ```

4. **Calcul des bindings matchees** : `useMemo` calculant un `Set<string>` de keyCodes matchant le filtre. Si filtre null → toutes les keys matchent.

5. **Passer `matchingKeys` au KeyGrid** via une nouvelle prop.

### Section 4 — Modification du KeyGrid

**Fichier a modifier:** `src/components/Keys/KeyGrid.tsx`

**Details :**

1. **Nouvelle prop** : `matchingKeys?: Set<string>` (optionnelle pour retrocompat)

2. **Modifier l'interface** :
   ```typescript
   interface KeyGridProps {
     selectedKeys: Set<string>;
     onKeySelect: (keyCode: string, event: React.MouseEvent) => void;
     onSelectAll: () => void;
     matchingKeys?: Set<string>;  // NEW
   }
   ```

3. **Dans le rendu de chaque binding** (ligne 86-126) : si `matchingKeys` est defini et que `kb.keyCode` n'est pas dedans, ajouter `opacity-30 pointer-events-none` aux classes du bouton.

4. **Modifier le message vide** (ligne 53-56) : si des bindings existent mais aucune ne matche, afficher "No matching keys" au lieu du message "No keys assigned".

5. **Ctrl+A (Select All)** (ligne 62-67) : ne selectionner que les bindings visibles (matchees), pas toutes.

### Section 5 — Integration du raccourci Ctrl+F

**Fichier a modifier:** `src/components/Layout/MainContent.tsx`

**Details :**

Ajouter dans le `useEffect` existant ou un nouveau :
```typescript
useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && e.key === "f") {
      e.preventDefault();
      searchBarRef.current?.focus();
    }
  };
  window.addEventListener("keydown", handleKeyDown);
  return () => window.removeEventListener("keydown", handleKeyDown);
}, []);
```

**Attention** : ce handler doit etre enregistre au niveau `window` pour intercepter le Ctrl+F natif du navigateur (recherche dans la page). L'appel `e.preventDefault()` est critique.

**Interaction avec `useTextInputFocus`** : quand l'input de recherche est focus, le hook `useTextInputFocus` desactivera automatiquement la detection des touches (comportement existant pour tous les inputs texte). C'est le comportement voulu — on ne veut pas que taper dans la recherche declenche des sons.

---

## Fichiers a creer

| Fichier | Description |
|---------|-------------|
| `src/components/common/SearchFilterBar.tsx` | Composant barre de recherche/filtre avec chips, compteur, parsing de prefixes |

## Fichiers a modifier

| Fichier | Modification |
|---------|-------------|
| `src/types/index.ts` | Ajouter interface `KeyGridFilter` |
| `src/components/Layout/MainContent.tsx` | State filtre, raccourci Ctrl+F, integration SearchFilterBar dans le header, calcul `matchingKeys` via useMemo |
| `src/components/Keys/KeyGrid.tsx` | Nouvelle prop `matchingKeys`, griser les non-matchees, adapter Select All et message vide |

---

## Notes techniques

- **Performance** : Le filtrage est un simple `.filter()` sur `keyBindings` (max ~200 items). Pas besoin de debounce, Web Worker, ou index. Un `useMemo` avec les bons deps suffit.
- **Coherence CSS** : Utiliser exactement les memes classes que les inputs existants dans `AddSoundModal.tsx:797-806` et `SoundDetails.tsx:371-377`.
- **Undo/Redo** : Le filtre est de l'UI state local, il n'affecte pas le profil. Pas d'interaction avec `historyStore`.
- **Persistence** : Le filtre se reset au changement de profil (meme `useEffect` que `selectedKeys`, ligne 114-117 de MainContent).
- **Accessibilite** : `aria-label="Search key bindings"` sur l'input, `role="status"` sur le compteur, chips avec `aria-label="Remove filter: ..."`.
