# Multi-Selection des Cards dans le KeyGrid

> **Statut:** En attente
> **Priorité:** Feature UX

## Objectif

Permettre la sélection multiple de cards (key bindings) dans le KeyGrid, avec un panel d'édition bulk quand plusieurs cards sont sélectionnées. Le comportement reproduit la sélection de fichiers standard de Windows Explorer.

---

## Comportement de sélection

### Click simple (sans modifier)
Sélectionne uniquement cette card, désélectionne toutes les autres. Click sur la card déjà sélectionnée (quand c'est la seule) la désélectionne.

### Ctrl+Click
Toggle la card dans la sélection (ajoute si pas dedans, retire si déjà dedans). Les autres cards sélectionnées restent sélectionnées.

### Shift+Click
Sélection par range. Sélectionne toutes les cards entre la dernière card cliquée (anchor) et la card cliquée, incluses. Remplace la sélection actuelle par cette range. L'ordre est celui du rendu dans le grid (l'index dans `currentProfile.keyBindings`).

### Ctrl+A
Sélectionne toutes les cards. Ne doit fonctionner **que** quand le KeyGrid (ou une card) a le focus, **pas** quand un `<input>` ou `<textarea>` est focused. Appeler `e.preventDefault()` pour bloquer le select-all natif du navigateur.

---

## Changements d'état dans MainContent

**Fichier :** `src/components/Layout/MainContent.tsx`

L'état `selectedKey: string | null` est remplacé par :

```typescript
const [selectedKeys, setSelectedKeys] = useState<Set<string>>(new Set());
const [anchorKey, setAnchorKey] = useState<string | null>(null);
```

**Callback de sélection :**

```typescript
const handleKeySelect = (keyCode: string, event: React.MouseEvent) => {
  const bindings = currentProfile.keyBindings;

  if (event.ctrlKey || event.metaKey) {
    // Ctrl+Click : toggle dans la sélection
    setSelectedKeys(prev => {
      const next = new Set(prev);
      if (next.has(keyCode)) next.delete(keyCode);
      else next.add(keyCode);
      return next;
    });
    setAnchorKey(keyCode);

  } else if (event.shiftKey && anchorKey) {
    // Shift+Click : sélection par range
    const anchorIdx = bindings.findIndex(kb => kb.keyCode === anchorKey);
    const targetIdx = bindings.findIndex(kb => kb.keyCode === keyCode);
    if (anchorIdx !== -1 && targetIdx !== -1) {
      const start = Math.min(anchorIdx, targetIdx);
      const end = Math.max(anchorIdx, targetIdx);
      const rangeKeys = bindings.slice(start, end + 1).map(kb => kb.keyCode);
      setSelectedKeys(new Set(rangeKeys));
    }
    // anchorKey ne change pas sur Shift+Click

  } else {
    // Click simple : sélection unique (toggle si déjà seul sélectionné)
    if (selectedKeys.size === 1 && selectedKeys.has(keyCode)) {
      setSelectedKeys(new Set());
      setAnchorKey(null);
    } else {
      setSelectedKeys(new Set([keyCode]));
      setAnchorKey(keyCode);
    }
  }
};

const handleSelectAll = () => {
  if (!currentProfile) return;
  const allKeys = currentProfile.keyBindings.map(kb => kb.keyCode);
  setSelectedKeys(new Set(allKeys));
};
```

**Rendu conditionnel du panel :**

- `selectedKeys.size === 0` : pas de panel, pas de resize handle.
- `selectedKeys.size === 1` : afficher `SoundDetails` existant (passer la seule clé via `[...selectedKeys][0]`).
- `selectedKeys.size > 1` : afficher le nouveau composant `MultiKeyDetails`.

Le resize handle et le conteneur avec `panelHeight` restent identiques, seul le contenu change.

**Adaptation des callbacks existants :**

- `SoundDetails.onClose` : appelle `setSelectedKeys(new Set())`.
- `SoundDetails.onKeyChanged` : remplace l'ancienne clé par la nouvelle dans `selectedKeys`.

**Reset de la sélection :**

- Vider `selectedKeys` quand le profil change (ajouter un `useEffect` sur `currentProfile?.id`).

---

## Changements dans KeyGrid

**Fichier :** `src/components/Keys/KeyGrid.tsx`

### Nouvelles props

```typescript
interface KeyGridProps {
  selectedKeys: Set<string>;
  onKeySelect: (keyCode: string, event: React.MouseEvent) => void;
  onSelectAll: () => void;
}
```

### Sélection visuelle

```typescript
const isSelected = selectedKeys.has(kb.keyCode);
```

Le style de la card sélectionnée reste `border-accent-primary bg-accent-primary/10` (pas de changement visuel).

### Click handler

```tsx
onClick={(e) => onKeySelect(kb.keyCode, e)}
```

### Ctrl+A

Rendre le conteneur du grid focusable avec `tabIndex={0}` et un `onKeyDown` :

```tsx
<div
  className="flex flex-wrap gap-2"
  tabIndex={0}
  onKeyDown={(e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === "a") {
      // Ne pas intercepter si un input est focused
      const tag = (document.activeElement as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
      e.preventDefault();
      onSelectAll();
    }
  }}
>
```

Ajouter un style `outline-none focus:outline-none` sur ce div pour ne pas montrer le focus ring du navigateur.

---

## Nouveau composant : MultiKeyDetails

**Fichier à créer :** `src/components/Sounds/MultiKeyDetails.tsx`

### Props

```typescript
interface MultiKeyDetailsProps {
  selectedKeys: Set<string>;
  onClose: () => void;
}
```

### Contenu du panel

#### 1. Header

```
"{N} keys selected"                              [Delete {N} keys]  [Close]
```

- Texte : `"{selectedKeys.size} keys selected"`.
- Bouton Close (appelle `onClose`).
- Bouton Delete en rouge.

#### 2. Track dropdown

- Lire le `trackId` de chaque binding sélectionné.
- Si tous identiques : afficher cette valeur dans le `<select>`.
- Si différents : afficher une `<option disabled>` avec le texte `"Mixed"` comme valeur affichée. Cette option n'est pas sélectionnable, elle sert d'indicateur.
- Quand l'utilisateur choisit un track : appeler `updateKeyBinding(keyCode, { trackId })` pour **chaque** key sélectionnée, puis `saveCurrentProfile()`.

```tsx
const trackIds = [...selectedKeys]
  .map(k => bindings.find(kb => kb.keyCode === k)?.trackId)
  .filter(Boolean);
const allSameTrack = trackIds.every(id => id === trackIds[0]);
const commonTrackId = allSameTrack ? trackIds[0] : null;
```

#### 3. Loop mode dropdown

Même logique que Track :
- Valeur commune ou `"Mixed"`.
- Changer applique `updateKeyBinding(keyCode, { loopMode, currentIndex: 0 })` pour chaque key sélectionnée.

#### 4. Delete button

- Texte : `"Delete {N} keys"` en rouge.
- Confirmation via `useConfirmStore.confirm("Delete {N} key bindings?")`.
- Si confirmé : `removeKeyBinding(keyCode)` pour chaque key, `saveCurrentProfile()`, `onClose()`.

### Pas de contenu sons

Le panel **n'affiche PAS** la liste des sons individuels. Pas de volume, pas de momentum, pas de preview, pas de "Add Sound to Key", pas de "Change Key", pas de "Move Sound". Uniquement Track, Loop, Delete.

---

## Intégration Undo/Redo

Les opérations bulk doivent produire **une seule entrée** dans l'historique, pas N entrées séparées.

**Pattern à suivre :**

```typescript
import { useHistoryStore, captureProfileState } from "../../stores/historyStore";
import { useProfileStore } from "../../stores/profileStore";

// Avant l'opération bulk
const profile = useProfileStore.getState().currentProfile!;
const before = captureProfileState(profile);

// Effectuer toutes les modifications
for (const keyCode of selectedKeys) {
  updateKeyBinding(keyCode, { trackId: newTrackId });
}

// Après l'opération bulk
const after = captureProfileState(useProfileStore.getState().currentProfile!);
useHistoryStore.getState().pushState(
  `Change track for ${selectedKeys.size} keys`,
  before,
  after
);

saveCurrentProfile();
```

Regarder comment `profileStore.ts` utilise `pushState` pour les actions individuelles existantes et reproduire le même pattern. L'important est que `captureProfileState` est appelé **avant** et **après** le batch de modifications, et qu'un seul `pushState` est fait.

Messages d'historique :
- `"Change track for {N} keys"`
- `"Change loop mode for {N} keys"`
- `"Delete {N} keys"`

---

## Fichiers impactés

| Fichier | Action |
|---|---|
| `src/components/Layout/MainContent.tsx` | Modifier : `selectedKeys: Set<string>`, `anchorKey`, logique de sélection, rendu conditionnel |
| `src/components/Keys/KeyGrid.tsx` | Modifier : props, `tabIndex`, Ctrl+A, multi-sélection visuelle |
| `src/components/Sounds/SoundDetails.tsx` | Adapter : props inchangées en interne, mais `onClose` vide le Set, `onKeyChanged` met à jour le Set |
| `src/components/Sounds/MultiKeyDetails.tsx` | **Créer** : panel bulk edit (Track, Loop, Delete) |

---

## Edge cases

- **Key supprimée** : si une key dans `selectedKeys` est supprimée (via delete bulk ou undo), elle doit être retirée de `selectedKeys`. Filtrer `selectedKeys` contre les keyCodes existants dans `keyBindings` lors du rendu.
- **Changement de profil** : vider `selectedKeys` (useEffect sur `currentProfile?.id`).
- **Undo/Redo** : après un undo/redo, des keyCodes dans `selectedKeys` peuvent ne plus exister. Filtrer automatiquement.
- **Panel resize** : le resize handle fonctionne identiquement que ce soit `SoundDetails` ou `MultiKeyDetails` affiché en dessous.
- **Pas d'impact backend** : la sélection est purement frontend, aucun changement côté Rust.

---

## Ce qu'il ne faut PAS faire

- Ne pas afficher la liste des sons individuels en multi-sélection.
- Ne pas ajouter de "Change Key" ou "Move Sound" en multi-sélection.
- Ne pas changer le comportement du click simple existant (1 click sans modifier = sélection unique).
- Ne pas toucher aux stores Zustand (profileStore, settingsStore) sauf si strictement nécessaire pour le bulk undo.
- Ne pas ajouter de nouveaux fichiers au-delà de `MultiKeyDetails.tsx`.
