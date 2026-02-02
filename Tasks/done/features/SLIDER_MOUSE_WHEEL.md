# Contrôle des Sliders via Molette de Souris

> **Catégorie:** Amélioration UX
> **Priorité:** Moyenne
> **Statut:** ✅ Completed
> **Date ajoutée:** 2026-02-02

## Description

Ajouter le support de la molette de souris sur tous les sliders (`<input type="range">`) de l'application. Chaque tick de molette ajuste la valeur de ±½ step (ou un incrément adapté au slider).

## Motivation

Actuellement aucun slider ne réagit à la molette. L'utilisateur doit cliquer-glisser pour ajuster les valeurs, ce qui est moins pratique — surtout pour les ajustements fins de volume pendant la lecture. La molette offre un contrôle rapide et précis sans quitter le workflow.

## Sliders concernés (12 au total)

| # | Slider | Fichier | Ligne | Min | Max | Step actuel | Incrément molette proposé |
|---|--------|---------|-------|-----|-----|-------------|---------------------------|
| 1 | Master Volume | `Header.tsx` | ~37 | 0 | 100 | 1 | **±1** (1%) |
| 2 | Track Volume | `TrackView.tsx` | ~154 | 0 | 100 | 1 | **±1** (1%) |
| 3 | Sound Volume (details) | `SoundDetails.tsx` | ~410 | 0 | 100 | 1 | **±1** (1%) |
| 4 | Sound Volume (add modal) | `AddSoundModal.tsx` | ~1111 | 0 | 100 | 1 | **±1** (1%) |
| 5 | Sound Momentum (details) | `SoundDetails.tsx` | ~460 | 0 | duration | 0.1 | **±0.5s** |
| 6 | Sound Momentum (add modal) | `AddSoundModal.tsx` | ~986 | 0 | duration | 0.1 | **±0.5s** |
| 7 | Discovery Preview Volume | `DiscoveryPanel.tsx` | ~638 | 0 | 1 | 0.01 | **±0.01** (1%) |
| 8 | Search Preview Seek | `SearchResultPreview.tsx` | ~158 | 0 | duration | 0.1 | **±0.5s** |
| 9 | Now Playing Seek | `NowPlaying.tsx` | ~63 | 0 | duration | 0.5 | **±0.5s** |
| 10 | Key Cooldown | `SettingsModal.tsx` | ~336 | 0 | 2000 | 50 | **±50ms** |
| 11 | Chord Window | `SettingsModal.tsx` | ~361 | 20 | 100 | 5 | **±5ms** |
| 12 | Crossfade Duration | `SettingsModal.tsx` | ~441 | 100 | 2000 | 50 | **±50ms** |

## Détails techniques

### Approche recommandée : Hook custom `useWheelSlider`

Créer un hook réutilisable plutôt que copier-coller le handler `onWheel` 12 fois.

**Fichier à créer :** `src/hooks/useWheelSlider.ts`

```typescript
// Concept :
function useWheelSlider(options: {
  value: number;
  min: number;
  max: number;
  step: number;        // incrément par tick de molette
  onChange: (value: number) => void;
}): {
  onWheel: (e: React.WheelEvent) => void;
}
```

**Comportement :**
- `deltaY < 0` (scroll up) → `+step`
- `deltaY > 0` (scroll down) → `−step`
- Clamp entre `min` et `max`
- `e.preventDefault()` pour empêcher le scroll de la page
- Arrondir au step le plus proche pour éviter les erreurs de floating point

**Note importante :** `preventDefault()` sur `wheel` nécessite que l'event listener soit **non-passive**. Les events React sont passifs par défaut. Il faudra soit :
- Utiliser un `ref` + `addEventListener('wheel', handler, { passive: false })` dans un `useEffect`
- Ou retourner une `ref` depuis le hook qui s'attache au DOM directement

L'approche `ref` + `useEffect` est la plus propre :

```typescript
function useWheelSlider(options) {
  const ref = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const handler = (e: WheelEvent) => {
      e.preventDefault();
      const direction = e.deltaY < 0 ? 1 : -1;
      const newValue = Math.min(options.max, Math.max(options.min, options.value + direction * options.step));
      options.onChange(newValue);
    };
    el.addEventListener('wheel', handler, { passive: false });
    return () => el.removeEventListener('wheel', handler);
  }, [options.value, options.min, options.max, options.step, options.onChange]);

  return ref;
}
```

### Intégration dans chaque composant

Pour chaque slider, l'intégration se résume à :

```tsx
const volWheelRef = useWheelSlider({
  value: Math.round(config.masterVolume * 100),
  min: 0, max: 100, step: 1,
  onChange: (v) => handleVolumeChange(v / 100),
});

<input ref={volWheelRef} type="range" ... />
```

### Cas particuliers

- **Volume sliders** (Master, Track, Sound) : les handlers existants sont déjà debounced (100ms) — le hook doit appeler le même handler `handleVolumeChange` pour conserver le debounce.
- **Momentum sliders** (SoundDetails, AddSoundModal) : le handler existant gère le debounced seek si en preview — réutiliser `handleMomentumChange` tel quel.
- **Settings sliders** (Cooldown, Chord, Crossfade) : pas de debounce nécessaire, les changements sont déjà instantanés via le store.

## Tâches

- [x] Créer `src/hooks/useWheelSlider.ts` avec l'approche `ref` + `addEventListener` non-passive
- [x] Intégrer sur Master Volume (`Header.tsx:~37`)
- [x] Intégrer sur Track Volume (`TrackView.tsx:~154`)
- [x] Intégrer sur Sound Volume dans SoundDetails (`SoundDetails.tsx:~410`)
- [x] Intégrer sur Sound Volume dans AddSoundModal (`AddSoundModal.tsx:~1111`)
- [x] Intégrer sur Sound Momentum dans SoundDetails (`SoundDetails.tsx:~460`)
- [x] Intégrer sur Sound Momentum dans AddSoundModal (`AddSoundModal.tsx:~986`)
- [x] Intégrer sur Discovery Preview Volume (`DiscoveryPanel.tsx:~638`)
- [x] Intégrer sur Search Preview Seek (`SearchResultPreview.tsx:~158`)
- [x] Intégrer sur Now Playing Seek (`NowPlaying.tsx:~63`)
- [x] Intégrer sur Key Cooldown (`SettingsModal.tsx:~336`)
- [x] Intégrer sur Chord Window (`SettingsModal.tsx:~361`)
- [x] Intégrer sur Crossfade Duration (`SettingsModal.tsx:~441`)
- [x] Tester que le scroll de page ne se déclenche pas quand le curseur est sur un slider

## Fichiers à créer

| Fichier | Description |
|---------|-------------|
| `src/hooks/useWheelSlider.ts` | Hook custom pour le contrôle molette des sliders |

## Fichiers à modifier

| Fichier | Modification |
|---------|-------------|
| `src/components/Layout/Header.tsx` | Ajouter `ref` molette sur Master Volume |
| `src/components/Tracks/TrackView.tsx` | Ajouter `ref` molette sur Track Volume |
| `src/components/Sounds/SoundDetails.tsx` | Ajouter `ref` molette sur Sound Volume + Momentum |
| `src/components/Sounds/AddSoundModal.tsx` | Ajouter `ref` molette sur Volume + Momentum |
| `src/components/Discovery/DiscoveryPanel.tsx` | Ajouter `ref` molette sur Preview Volume |
| `src/components/Sounds/SearchResultPreview.tsx` | Ajouter `ref` molette sur Seek bar |
| `src/components/Controls/NowPlaying.tsx` | Ajouter `ref` molette sur Seek bar |
| `src/components/Settings/SettingsModal.tsx` | Ajouter `ref` molette sur Cooldown + Chord + Crossfade |

## Notes

- L'incrément de ±1 pour les volumes (sur 100) correspond bien à la demande de "±½" puisque le step natif est 1 — on ne peut pas descendre en dessous du step. Pour les sliders avec step 0.1 (momentum), ±0.5 donne 5× le step ce qui est un bon compromis rapidité/précision.
- Si l'utilisateur souhaite un comportement différent (ex: ±2 pour les volumes, ±1s pour le momentum), l'incrément est facilement ajustable par slider puisque c'est un paramètre du hook.
