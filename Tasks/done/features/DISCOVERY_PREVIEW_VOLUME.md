# Discovery — Volume de Preview

> **Catégorie:** Feature
> **Priorité:** Moyenne
> **Statut:** ✅ Completed
> **Date complétée:** 2026-02-02
> **Date ajoutée:** 2026-02-02

## Description

Ajouter un contrôle de volume dédié pour les previews Discovery, indépendant du volume master. Actuellement le preview joue à volume fixe (`sound_volume = 1.0`) modulé uniquement par le master volume — l'utilisateur est obligé de baisser le volume global de l'appli (ou de l'ordinateur) juste pour écouter un preview sans se faire exploser les oreilles.

## Motivation

Quand on parcourt les suggestions Discovery, on veut pouvoir écouter les previews à un volume confortable sans toucher au volume master qui affecte tous les sons en cours. C'est un problème d'UX fréquent : le volume des previews YouTube est souvent incohérent d'un son à l'autre.

## Analyse du code actuel

### Volume hardcodé à 1.0

**`src/components/Discovery/DiscoveryPanel.tsx:321-327`** — `handlePreview()` :
```typescript
await commands.playSound(
  "__preview__",
  s.videoId,
  s.cachedPath,
  s.suggestedMomentum,
  1.0  // ← volume hardcodé
);
```

**`src/components/Discovery/DiscoveryPanel.tsx:359`** — `handleSeekPreview()` :
```typescript
await commands.playSound("__preview__", s.videoId, s.cachedPath, position, 1.0);
```

### Calcul du volume final (backend)

**`src-tauri/src/audio/track.rs:97`** :
```rust
let final_volume = sound_volume * self.volume * master_volume;
// Pour __preview__ : 1.0 × 1.0 × master = master
```

### Commande existante `set_sound_volume`

**`src/utils/tauriCommands.ts:89-94`** — déjà disponible côté frontend :
```typescript
export async function setSoundVolume(trackId: string, soundId: string, volume: number): Promise<void>
```

**`src-tauri/src/commands.rs:243-250`** — backend :
```rust
pub fn set_sound_volume(state: State<'_, AppState>, track_id: String, sound_id: String, volume: f32) -> Result<(), String>
```

Cette commande permet de changer le volume d'un son en cours de lecture sur un track donné, donc on peut l'utiliser pour ajuster le volume du preview en temps réel.

## Tâches

### 1. State — Preview volume dans `discoveryStore`

- [x] Ajouter un champ `previewVolume: number` (défaut `0.5`) dans `discoveryStore.ts`
- [x] Ajouter l'action `setPreviewVolume(volume: number)` qui :
  - Met à jour le state
  - Appelle `commands.setSoundVolume("__preview__", currentSoundId, volume)` si un preview est en cours
- [x] Persister la valeur dans `localStorage` (comme pour d'autres prefs UI) pour que le réglage survive entre les sessions

### 2. UI — Slider de volume dans le DiscoveryPanel

- [x] Ajouter un petit slider de volume (type `<input type="range">`) dans la zone preview de `SuggestionCard` ou en haut du panel Discovery
  - **Option recommandée :** À côté du bouton play (Row 2, `DiscoveryPanel.tsx:797`), entre le bouton ▶ et la waveform
  - Style compact : slider fin, largeur ~50-60px, icône volume à gauche (🔊/🔉/🔇 selon niveau)
  - Couleur cohérente avec le thème (accent-primary)
- [x] Le slider doit être visible même quand aucun preview ne joue (pour pré-régler)
- [x] Afficher le pourcentage en tooltip au survol

### 3. Intégration — Passer le volume aux appels `playSound`

- [x] `handlePreview()` (`DiscoveryPanel.tsx:321`) — remplacer `1.0` par la valeur du store :
  ```typescript
  const previewVolume = useDiscoveryStore.getState().previewVolume;
  await commands.playSound("__preview__", s.videoId, s.cachedPath, s.suggestedMomentum, previewVolume);
  ```
- [x] `handleSeekPreview()` (`DiscoveryPanel.tsx:359`) — idem
- [x] Quand le slider change pendant la lecture, appeler `setSoundVolume("__preview__", videoId, newVolume)` pour mise à jour temps réel (via le handler du store)

## Fichiers à modifier

| Fichier | Modification |
|---------|-------------|
| `src/stores/discoveryStore.ts` | Ajouter `previewVolume`, `setPreviewVolume()`, persistence localStorage |
| `src/components/Discovery/DiscoveryPanel.tsx` | Slider UI + utiliser `previewVolume` dans `handlePreview`/`handleSeekPreview` |

## Notes

- Pas besoin de modifier le backend — les commandes `playSound(soundVolume)` et `setSoundVolume()` existent déjà
- Le volume ramping du backend (`track.rs` — step 0.1, ~160ms) évitera les clics lors des changements de volume en temps réel
- Volume par défaut à `0.5` (50%) pour éviter les surprises, contrairement au `1.0` actuel
- Le `__preview__` track n'est pas un vrai track de profil, donc pas de conflit avec les volumes de tracks existants
