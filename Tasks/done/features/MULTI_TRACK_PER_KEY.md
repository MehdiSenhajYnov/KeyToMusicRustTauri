# Multi-Track par Touche — Lancer plusieurs sons simultanément

> **Statut:** ✅ Terminé
> **Type:** Feature — Permettre qu'une même touche déclenche des sons sur des tracks différents simultanément
> **Objectif:** Appuyer sur une touche lance un son par track associé (ex: OST triste + pluie), au lieu d'un seul son sur un seul track comme actuellement.

---

## Vue d'ensemble

### Problème actuel

Aujourd'hui, un `KeyBinding` est identifié uniquement par son `keyCode`. Chaque touche ne peut avoir **qu'un seul binding**, qui pointe vers **un seul track** :

```typescript
// src/types/index.ts:30-37
export interface KeyBinding {
  keyCode: KeyCode;
  trackId: TrackId;       // ← UN seul track
  soundIds: SoundId[];    // Sons qui cyclent sur CE track
  loopMode: LoopMode;
  currentIndex: number;
  name?: string;
}
```

Conséquences :
- Impossible de lancer "OST triste" (track OST) + "pluie" (track Ambiance) avec une seule touche
- **Bug actuel :** dans `AddSoundModal`, si on ajoute un son à une touche existante en choisissant un track différent, le track est ignoré — le son est ajouté au binding existant sans changer son `trackId` (`src/components/Sounds/AddSoundModal.tsx:724-731`)

### Solution proposée

Changer la clé d'unicité des bindings de `keyCode` seul vers `keyCode + trackId`. Une touche peut ainsi avoir **plusieurs bindings**, un par track. Quand la touche est pressée, chaque binding joue son son sur son track respectif.

Exemple : touche `KeyA` avec 2 bindings :
- `{ keyCode: "KeyA", trackId: "ost-1", soundIds: ["sad_theme"], loopMode: "single" }`
- `{ keyCode: "KeyA", trackId: "ambiance-1", soundIds: ["rain", "wind"], loopMode: "random" }`

Résultat : appuyer sur A lance `sad_theme` sur la track OST **et** un son aléatoire (rain/wind) sur la track Ambiance.

---

## Implementation

### Section 1 — Nouveau modèle de données

**Fichiers concernés :**
- `src/types/index.ts` — Type `KeyBinding` (inchangé structurellement, mais la sémantique change)
- `src-tauri/src/types.rs` — Struct `KeyBinding` (idem)

**Détails :**

Le type `KeyBinding` ne change pas structurellement — il a déjà `keyCode` + `trackId`. Ce qui change c'est la **contrainte d'unicité** : on passe de "un binding par `keyCode`" à "un binding par `(keyCode, trackId)`".

Il faut ajouter un helper pour identifier un binding de manière unique :

```typescript
// Nouvelle convention : identifier un binding par keyCode + trackId
type BindingKey = `${KeyCode}::${TrackId}`;
function bindingKey(kb: KeyBinding): BindingKey {
  return `${kb.keyCode}::${kb.trackId}`;
}
```

### Section 2 — Profile Store (coeur du changement)

**Fichier concerné :** `src/stores/profileStore.ts`

**2.1 — `addKeyBinding` (lignes 424-446)**

Actuellement :
```typescript
const existing = state.currentProfile.keyBindings.filter(
  (kb) => kb.keyCode !== binding.keyCode  // ← Supprime TOUT binding sur cette touche
);
```

Nouveau comportement :
```typescript
const existing = state.currentProfile.keyBindings.filter(
  (kb) => !(kb.keyCode === binding.keyCode && kb.trackId === binding.trackId)
  // ← Ne supprime que le binding sur la même touche ET le même track
);
```

**2.2 — `updateKeyBinding` (lignes 448-478)**

Actuellement cherche par `keyCode` seul :
```typescript
state.currentProfile.keyBindings.map((kb) =>
  kb.keyCode === keyCode ? { ...kb, ...updates } : kb
)
```

Doit chercher par `keyCode + trackId` :
```typescript
// Signature change: updateKeyBinding(keyCode, trackId, updates)
state.currentProfile.keyBindings.map((kb) =>
  kb.keyCode === keyCode && kb.trackId === trackId ? { ...kb, ...updates } : kb
)
```

**2.3 — `removeKeyBinding` (lignes 480-515)**

Actuellement supprime par `keyCode` seul. Deux options :
- `removeKeyBinding(keyCode, trackId)` — supprime un binding spécifique
- `removeAllKeyBindings(keyCode)` — supprime tous les bindings d'une touche (utile pour "supprimer la touche entière")

L'orphan cleanup (sons non-référencés) reste identique.

**2.4 — `removeTrack` (lignes 355-396)**

Déjà correct : supprime tous les bindings dont `trackId` correspond au track supprimé. Pas de changement nécessaire.

**2.5 — `removeSound` (lignes 274-301)**

Déjà correct : retire le `soundId` de tous les bindings qui le contiennent, puis supprime les bindings vides. Pas de changement nécessaire.

### Section 3 — Key Detection (déclenchement multi-track)

**Fichier concerné :** `src/hooks/useKeyDetection.ts`

**3.1 — `handleKeyPress` (lignes 84-159)**

Actuellement : trouve UN binding, joue UN son.

Nouveau comportement :
```typescript
// Au lieu de:
let binding = currentProfile.keyBindings.find(kb => kb.keyCode === payload.keyCode);

// Trouver TOUS les bindings pour cette touche:
const bindings = currentProfile.keyBindings.filter(kb => kb.keyCode === payload.keyCode);
if (bindings.length === 0) return;

// Pour chaque binding, sélectionner et jouer un son
for (const binding of bindings) {
  const { sound, nextIndex } = selectSound(
    binding.soundIds, soundMap, binding.loopMode, binding.currentIndex
  );
  if (!sound) continue;

  if (binding.loopMode !== "single") {
    updateKeyBinding(binding.keyCode, binding.trackId, { currentIndex: nextIndex });
  }

  const startPosition = (config.autoMomentum || useModifierForMomentum) ? sound.momentum : 0;
  await commands.playSound(binding.trackId, sound.id, filePath, startPosition, sound.volume);
}
```

**3.2 — Momentum modifier (lignes 94-110)**

La logique momentum avec base key doit aussi trouver tous les bindings et appliquer le momentum à chacun.

### Section 4 — AddSoundModal (fix du bug + nouveau flow)

**Fichier concerné :** `src/components/Sounds/AddSoundModal.tsx`

**4.1 — Submit logic (lignes 667-749)**

Actuellement (le bug) :
```typescript
const existingBinding = currentProfile?.keyBindings.find(
  (kb) => kb.keyCode === keyCode  // ← Trouve LE binding, ignore le track sélectionné
);
if (existingBinding) {
  updateKeyBinding(keyCode, {
    soundIds: [...existingBinding.soundIds, ...newSoundIds],
    // ← Ajoute au binding existant, track IGNORÉ
  });
}
```

Nouveau comportement :
```typescript
const existingBinding = currentProfile?.keyBindings.find(
  (kb) => kb.keyCode === keyCode && kb.trackId === trackId
  // ← Cherche un binding sur cette touche ET ce track
);
if (existingBinding) {
  // Même touche, même track → ajouter les sons au binding existant
  updateKeyBinding(keyCode, trackId, {
    soundIds: [...existingBinding.soundIds, ...newSoundIds],
  });
} else {
  // Même touche, track différent (ou nouvelle touche) → nouveau binding
  addKeyBinding({
    keyCode,
    trackId,
    soundIds: newSoundIds,
    loopMode,
    currentIndex: 0,
  });
}
```

**4.2 — UX indication**

Quand l'utilisateur choisit une touche déjà assignée, afficher sur quel(s) track(s) elle est déjà utilisée. Si le track sélectionné est différent, indiquer clairement que ça créera un binding additionnel ("Cette touche déclenchera aussi un son sur [Track Ambiance]").

### Section 5 — KeyGrid (affichage multi-track)

**Fichier concerné :** `src/components/Keys/KeyGrid.tsx`

**5.1 — Groupement par touche (lignes 71-130)**

Actuellement : une cellule par binding (= par touche). Avec multi-track, plusieurs bindings peuvent partager le même `keyCode`.

Options d'affichage :
1. **Grouper par keyCode** — une cellule par touche, afficher les tracks comme indicateurs (pastilles colorées, noms)
2. **Cellule par binding** — chaque binding a sa propre cellule (plus de cellules dans la grille)

**Approche recommandée : Option 1** (grouper par keyCode) — plus cohérent avec le concept "une touche = une action". La cellule montre le nom de la touche + indicateurs de chaque track assignée.

```typescript
// Grouper les bindings par keyCode
const bindingsByKey = new Map<string, KeyBinding[]>();
for (const kb of keyBindings) {
  const group = bindingsByKey.get(kb.keyCode) || [];
  group.push(kb);
  bindingsByKey.set(kb.keyCode, group);
}

// Rendre une cellule par keyCode
for (const [keyCode, bindings] of bindingsByKey) {
  // Afficher : nom de touche, track indicators, nombre total de sons
  // Playing state : vert si AU MOINS un binding joue
}
```

**5.2 — Indicateurs de track**

Chaque cellule de touche pourrait montrer :
- Le nombre de tracks actifs (ex: "2 tracks")
- Des pastilles de couleur par track
- Le nom des tracks en petit

**5.3 — Sélection**

`selectedKeys` reste basé sur `keyCode` (Set<string>). Sélectionner une touche sélectionne tous ses bindings.

### Section 6 — SoundDetails (panneau de détail)

**Fichier concerné :** `src/components/Sounds/SoundDetails.tsx`

Quand une touche est sélectionnée et a plusieurs bindings (multi-track), le panneau de détail doit afficher **chaque binding séparément**, groupé par track :

```
── Track: OST ──
  Sons: sad_theme.m4a
  Loop mode: Single
  [Changer track] [Supprimer ce binding]

── Track: Ambiance ──
  Sons: rain.m4a, wind.m4a
  Loop mode: Random
  [Changer track] [Supprimer ce binding]
```

### Section 7 — Backend (chord detector)

**Fichier concerné :** `src-tauri/src/commands.rs` (ligne 128-132)

`set_profile_bindings` envoie les `keyCode` au chord detector pour construire le Trie. Avec multi-track, un même `keyCode` peut apparaître plusieurs fois → dédupliquer avant d'envoyer :

```rust
// Avant d'envoyer au detector, dédupliquer les keyCodes
let unique_keys: Vec<String> = bindings.iter()
    .map(|b| b.key_code.clone())
    .collect::<HashSet<_>>()
    .into_iter()
    .collect();
state.key_detector.set_profile_bindings(&unique_keys);
```

Note : actuellement `set_profile_bindings` reçoit un `Vec<String>` de keyCodes, pas de bindings complets. Si des doublons sont envoyés, vérifier que le Trie les gère (probablement oui car il insère des chemins, un doublon écrase juste le noeud).

### Section 8 — Undo/Redo

**Fichier concerné :** `src/stores/historyStore.ts`

Le système d'undo capture des snapshots complets du profil (`captureProfileState`). Il n'a pas besoin de changement structurel — restaurer un snapshot restaure automatiquement tous les bindings dans leur état précédent, qu'il y en ait un ou plusieurs par touche.

Les fonctions modifiées (`addKeyBinding`, `updateKeyBinding`, `removeKeyBinding`) doivent continuer à appeler `captureProfileState` avant la modification et `pushToHistory` après.

### Section 9 — Discovery (auto-assignment)

**Fichier concerné :** `src/utils/profileAnalysis.ts`

`analyzeProfile()` et `suggestAssignment()` assignent une touche + track aux suggestions Discovery. Avec multi-track, il faut décider si une suggestion peut être assignée à une touche déjà utilisée (sur un track différent). Probablement oui — c'est même le use-case principal (rajouter de l'ambiance sur des touches existantes).

La logique "single-sound mode" vs "multi-sound mode" devrait rester basée sur le nombre moyen de sons par binding, mais en comptant les bindings par track séparément.

---

## Fichiers à modifier

| Fichier | Modification |
|---------|-------------|
| `src/stores/profileStore.ts` | `addKeyBinding`, `updateKeyBinding`, `removeKeyBinding` — unicité par `(keyCode, trackId)` |
| `src/hooks/useKeyDetection.ts` | `handleKeyPress` — trouver et jouer tous les bindings d'une touche |
| `src/components/Sounds/AddSoundModal.tsx` | Fix du bug track ignoré + UX pour multi-track |
| `src/components/Keys/KeyGrid.tsx` | Groupement des bindings par keyCode, indicateurs multi-track |
| `src/components/Sounds/SoundDetails.tsx` | Affichage multi-binding par track dans le panneau détail |
| `src-tauri/src/commands.rs` | Dédupliquer les keyCodes avant envoi au chord detector |
| `src/utils/profileAnalysis.ts` | Adapter la suggestion Discovery pour multi-track |
| `src/components/Layout/MainContent.tsx` | Adapter la sélection (si nécessaire) |

## Fichiers inchangés

| Fichier | Pourquoi |
|---------|----------|
| `src/types/index.ts` | Le type `KeyBinding` ne change pas structurellement |
| `src-tauri/src/types.rs` | Idem côté Rust |
| `src-tauri/src/keys/detector.rs` | Le detector envoie juste le keyCode, pas de binding lookup |
| `src-tauri/src/keys/chord.rs` | Le Trie fonctionne sur les keyCodes, pas impacté |
| `src-tauri/src/storage/profile.rs` | Sérialisation JSON inchangée |
| `src/stores/historyStore.ts` | Snapshots complets, pas de changement nécessaire |

---

## Points d'attention

1. **Migration des profils existants** — Pas nécessaire. Les profils existants ont déjà un binding par touche, ce qui est un sous-cas valide du nouveau modèle (un binding par touche = multi-track avec un seul track).

2. **Performance** — `filter()` au lieu de `find()` dans `handleKeyPress`. Impact négligeable (< 50 bindings typiquement).

3. **Cooldown** — Le cooldown de 200ms s'applique-t-il globalement (une touche = un cooldown pour tous ses tracks) ou par binding ? **Recommandé : global par touche** — un appui = tous les tracks jouent, cooldown empêche le double-appui.

4. **Stop All** — Déjà correct : `engine.stop_all()` arrête tous les tracks.

5. **Loop mode indépendant** — Chaque binding sur la même touche a son propre `loopMode` et `currentIndex`. Appuyer sur la touche avance chaque binding indépendamment.

6. **Conflits visuels KeyGrid** — Avec le filtre `t:track_name`, une touche multi-track devrait matcher si AU MOINS un de ses bindings est sur le track filtré.
