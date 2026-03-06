# Onboarding — Empty States Progressifs

> **Catégorie:** Feature / UX
> **Priorité:** Haute
> **Statut:** ✅ Completed (2026-02-02)
> **Date ajoutée:** 2026-02-02

## Description

Quand un nouvel utilisateur installe l'app, la zone principale est vide et rien ne lui indique quoi faire. L'objectif est d'afficher un **gros call-to-action centré** dans la zone principale qui guide l'utilisateur étape par étape :

1. **Pas de profil** → "Create Profile"
2. **Profil créé, pas de track** → "Create Track"
3. **Track(s) créé(s), aucun son** → "Add Sound" (avec mention drag & drop)

Quand l'utilisateur a au moins un son assigné (binding), l'UI normale s'affiche.

## Motivation

- L'onboarding actuel est quasi inexistant : textes `italic text-xs text-muted` perdus dans l'UI
- Un nouvel utilisateur ne sait pas par où commencer
- L'expérience doit être intuitive dès la première ouverture : le bouton central doit sembler naturel et intégré au design existant

## État actuel des empty states

### 1. Pas de profil (`MainContent.tsx:239-250`)
```tsx
<main className="flex-1 flex items-center justify-center bg-bg-primary">
  <div className="text-center">
    <p className="text-text-muted text-lg">No profile selected</p>
    <p className="text-text-muted text-sm mt-1">
      Create or select a profile to get started
    </p>
  </div>
</main>
```
**Problème:** Texte passif, aucun bouton d'action, l'utilisateur doit deviner qu'il faut aller dans la sidebar.

### 2. Pas de track (`TrackView.tsx:196-200`)
```tsx
<p className="text-text-muted text-xs italic">
  No tracks yet. Add a track to assign sounds.
</p>
```
Et aussi dans `MainContent.tsx:294-298` :
```tsx
<p className="text-text-muted text-xs italic text-center py-4">
  Create a track first, then add sounds and assign keys
</p>
```
**Problème:** Petit texte italic noyé dans la page, pas de bouton visible.

### 3. Pas de bindings (`KeyGrid.tsx:54-57`)
```tsx
<p className="text-text-muted text-xs italic">
  No keys assigned. Use "Add Sound" to create key bindings.
</p>
```
**Problème:** Texte discret, le bouton "+ Add Sound" existe en haut mais pas mis en avant.

## Design

### Composant `EmptyStateAction`

Créer un composant réutilisable `src/components/common/EmptyStateAction.tsx` :

- **Conteneur** : `flex items-center justify-center` centré dans l'espace disponible
- **Icône** : SVG 48x48 en `text-accent-primary/60`, au-dessus du bouton
- **Bouton principal** : Large (`px-6 py-3`), `bg-accent-primary text-white rounded-lg`, hover avec `bg-accent-primary/80`, `text-base font-medium`
- **Sous-titre** : Texte `text-text-muted text-sm` sous le bouton, expliquant brièvement l'action
- **Animation d'entrée** : `animate-fadeIn` subtil (opacity 0→1 + léger translateY, ~300ms)
- **Responsive** : Le bloc reste centré quelle que soit la taille (flexbox), max-width pour ne pas s'étirer excessivement

### Props du composant

```tsx
interface EmptyStateActionProps {
  icon: React.ReactNode;       // SVG icon
  buttonText: string;          // "Create Profile", "Create Track", "Add Sound"
  description: string;         // Sous-titre explicatif
  onAction: () => void;        // Callback du bouton
  secondaryHint?: string;      // Ex: "or drag & drop audio files" (optionnel)
}
```

### Les 3 écrans

#### Écran 1 — Pas de profil
- **Icône** : Dossier / profil utilisateur (folder-plus ou user-plus)
- **Bouton** : "Create Profile"
- **Description** : "A profile stores your sounds, tracks, and key bindings"
- **Action** : Déclenche la création de profil inline (même logique que `ProfileSelector.handleCreate`, mais ici avec un input centré dans le main content, ou ouvre directement l'input dans la sidebar)

#### Écran 2 — Pas de track
- **Icône** : Piste audio / layers (music-note ou layers)
- **Bouton** : "Create Track"
- **Description** : "Tracks organize your sounds (OST, Ambiance, SFX...)"
- **Action** : Déclenche `TrackView.setIsAdding(true)` — problème : c'est un state local. **Solution** : soit remonter le state, soit le bouton scrolle vers TrackView et focus l'input, soit utiliser un callback passé en prop.

#### Écran 3 — Pas de son / pas de binding
- **Condition** : `currentProfile.keyBindings.length === 0` (on vérifie les bindings, pas juste les sons, car un son sans binding n'est pas utilisable)
- **Icône** : Note de musique + plus (music-note-plus)
- **Bouton** : "Add Sound"
- **Description** : "Assign sounds to keyboard keys"
- **Hint secondaire** : "or drag & drop audio files here"
- **Action** : `setShowAddSound(true)` (ouvre AddSoundModal)

### Hiérarchie de priorité

L'affichage est **exclusif** (un seul écran à la fois), dans cet ordre :
1. `!currentProfile` → Écran "Create Profile"
2. `currentProfile.tracks.length === 0` → Écran "Create Track"
3. `currentProfile.keyBindings.length === 0` → Écran "Add Sound"
4. Sinon → UI normale (TrackView + KeyGrid + SoundDetails)

**Important** : Les écrans 2 et 3 remplacent le contenu de la zone scrollable (`MainContent.tsx:263`), pas la totalité de `<main>`. Le header de TrackView ("Tracks" + "+ Add Track") reste visible à l'écran 3 car l'utilisateur a déjà des tracks. Seule la KeyGrid vide est remplacée par le CTA centré.

**Correction** : En fait, pour les écrans 2 et 3, on veut que le CTA soit **le contenu principal dominant**. Donc :
- **Écran 2 (no tracks)** : Le CTA remplace tout le contenu scrollable. TrackView ne s'affiche pas (il n'y a rien à montrer).
- **Écran 3 (no bindings)** : TrackView s'affiche normalement en haut, mais au lieu de la KeyGrid vide + texte italic, on affiche le CTA centré dans l'espace restant.

### Cas particuliers à gérer

1. **Retour à un état vide** : Si l'utilisateur supprime tous ses tracks ou tous ses sons, le CTA réapparaît. Pas de "onboarding complété" persisté — c'est purement basé sur l'état.
2. **Profil avec sons mais sans bindings** : Peu probable en usage normal (les sons sont ajoutés via AddSoundModal qui crée aussi le binding), mais si ça arrive (ex: import corrompu), on affiche quand même le CTA "Add Sound".
3. **Drag & drop** : L'overlay drag-drop existant (`MainContent.tsx:255-261`) doit rester fonctionnel même quand le CTA "Add Sound" est affiché. Le hint "or drag & drop" du CTA le rend naturel.
4. **Sidebar "Create Profile"** : Le bouton "+ New Profile" dans la sidebar reste fonctionnel. Le CTA du main content est un raccourci supplémentaire, pas un remplacement. Pour l'action du CTA "Create Profile", le plus clean serait de focus/activer l'input de création dans la sidebar (via un callback ou un ref exposé par ProfileSelector).

## Fichiers à créer

| Fichier | Description |
|---------|-------------|
| `src/components/common/EmptyStateAction.tsx` | Composant réutilisable pour les CTAs d'onboarding |

## Fichiers à modifier

| Fichier | Modification |
|---------|-------------|
| `src/components/Layout/MainContent.tsx` | Remplacer les empty states existants par `EmptyStateAction`. Logique conditionnelle pour les 3 écrans. Passer un callback pour "Create Track" |
| `src/components/Keys/KeyGrid.tsx:54-57` | Supprimer le texte empty state inline (géré par MainContent maintenant) |
| `src/components/Tracks/TrackView.tsx:196-200` | Supprimer le texte empty state inline (géré par MainContent maintenant) |
| `src/components/Layout/MainContent.tsx:294-298` | Supprimer le second texte empty state "Create a track first..." |
| `src/index.css` (ou tailwind config) | Ajouter `@keyframes fadeIn` si pas déjà présent |

## Tâches

### Phase 1 — Composant EmptyStateAction
- [x] Créer `src/components/common/EmptyStateAction.tsx` avec les props définies ci-dessus
- [x] Implémenter le layout centré (icône + bouton + description + hint optionnel)
- [x] Ajouter l'animation d'entrée fadeIn (CSS keyframes ou Tailwind `animate-`)
- [x] S'assurer du rendu responsive (flexbox, max-width ~400px pour le bloc)

### Phase 2 — Intégration dans MainContent
- [x] **Écran 1 (no profile)** : Remplacer le texte passif (`MainContent.tsx:239-250`) par `EmptyStateAction` avec icon folder-plus, bouton "Create Profile", description
- [x] Implémenter l'action "Create Profile" : afficher un input inline centré dans le main content
- [x] **Écran 2 (no tracks)** : Remplacer le contenu scrollable quand `tracks.length === 0` par `EmptyStateAction` avec icon layers, bouton "Create Track"
- [x] Implémenter l'action "Create Track" : créer un track avec nom par défaut ("Track 1") directement
- [x] **Écran 3 (no bindings)** : Quand `keyBindings.length === 0`, afficher `EmptyStateAction` à la place de la KeyGrid vide, avec icon music-note, bouton "Add Sound", hint "or drag & drop"
- [x] Garder TrackView visible au-dessus du CTA à l'écran 3

### Phase 3 — Nettoyage
- [x] Supprimer le texte italic dans `KeyGrid.tsx:54-57` (le cas `keyBindings.length === 0` sera géré en amont)
- [x] Supprimer le texte italic dans `TrackView.tsx:196-200`
- [x] Supprimer le texte dans `MainContent.tsx:294-298`
- [x] Vérifier que le drag-drop overlay fonctionne toujours avec les CTAs affichés
- [ ] Tester le flow complet : install fresh → create profile → create track → add sound → UI normale
- [ ] Tester le retour à l'état vide : supprimer tous les sons → CTA réapparaît, supprimer tous les tracks → CTA change, supprimer le profil → CTA change

## Notes

- Les icônes peuvent être des SVG inline (comme dans le reste de l'app, voir `ProfileSelector.tsx:120-122`) — pas besoin de librairie d'icônes
- Le bouton du CTA doit utiliser les mêmes classes que les boutons primaires existants (`bg-accent-primary text-white rounded hover:bg-accent-primary/80`) mais en plus grand
- L'animation doit être subtile — pas de bounce ou de slide exagéré, juste un fadeIn doux
- Penser à `prefers-reduced-motion` pour désactiver l'animation si nécessaire
