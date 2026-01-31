# Section 3 — Waveform RMS (visualisation d'énergie audio)

> **Statut:** ⏳ PLANIFIE
> **Dépendance:** Aucune
> **Parent:** [SMART_DISCOVERY.md](./SMART_DISCOVERY.md)

> **Objectif:** Afficher une courbe d'énergie visuelle dans l'éditeur de momentum pour que l'utilisateur puisse choisir le bon point de départ en 2 secondes au lieu de scrubber à l'aveugle. Plus fiable que l'auto-détection, et utile pour tous les types de sons.

## Principe

Actuellement, régler le momentum c'est : écouter le son, noter mentalement le bon moment, ajuster le slider à l'aveugle, réécouter pour vérifier, répéter. Avec une waveform RMS, l'utilisateur **voit** la structure du son — l'intro calme, le buildup, le climax — et clique directement sur le bon point.

La waveform n'est pas un affichage sample-par-sample (trop lourd et peu lisible) mais une **courbe d'énergie RMS** lissée qui montre clairement les zones calmes vs intenses du morceau.

## Rendu visuel

```
Énergie
  │
  │         ╱╲    ╱╲  ╱──╲
  │        ╱  ╲  ╱  ╲╱    ╲
  │  ─────╱    ╲╱          ╲─────
  │ ══════                        ← intro calme
  └──────────────────────────────── temps
        ▲                          ← marqueur momentum (cliquable/draggable)
        │ 0:45
```

- **Axe X** : temps (0 → durée totale)
- **Axe Y** : énergie RMS normalisée (0 → 1)
- **Courbe** : remplie (area chart) avec couleur semi-transparente (indigo/violet pour rester dans le thème)
- **Marqueur momentum** : ligne verticale draggable sur la waveform, position = valeur momentum actuelle
- **Curseur de lecture** : si un preview est en cours, une ligne de progression avance sur la waveform en temps réel

## Backend (Rust)

- [ ] **3.1** Créer le module `src-tauri/src/audio/analysis.rs`
  - Fonction principale : `pub fn compute_waveform(file_path: &str, num_points: u32) -> Result<WaveformData, String>`
  - Utilise symphonia pour décoder les samples (même pipeline que `SymphoniaSource`)
  - Calcule le RMS par segments (nombre de segments = `num_points`, typiquement 200-300 pour un affichage fluide)
  - Normalise les valeurs entre 0.0 et 1.0 (diviser par le max RMS)
  - Lisse avec une moyenne mobile (fenêtre de 3) pour éviter le bruit visuel
  - Retourne le tableau de valeurs RMS + la durée totale

- [ ] **3.2** Définir le type `WaveformData` dans `types.rs`
  ```rust
  pub struct WaveformData {
      pub points: Vec<f32>,     // Valeurs RMS normalisées (0.0-1.0), longueur = num_points
      pub duration: f64,        // Durée totale en secondes
      pub sample_rate: u32,     // Pour référence
  }
  ```

- [ ] **3.3** Détail de l'implémentation `compute_waveform`
  ```rust
  pub fn compute_waveform(file_path: &str, num_points: u32) -> Result<WaveformData, String> {
      // 1. Ouvrir le fichier avec symphonia (probe + format reader)
      // 2. Obtenir la durée totale (n_frames / sample_rate)
      // 3. Calculer segment_size = total_samples / num_points
      // 4. Pour chaque segment :
      //    - Décoder les samples
      //    - Calculer RMS = sqrt(sum(sample²) / count)
      //    - Stocker dans Vec<f32>
      // 5. Normaliser : diviser tous les points par max(points)
      //    - Si max == 0 (silence complet) : retourner des zéros
      // 6. Lisser : moyenne mobile fenêtre 3
      //    smoothed[i] = (points[i-1] + points[i] + points[i+1]) / 3
      // 7. Retourner WaveformData { points, duration, sample_rate }
  }
  ```

- [ ] **3.4** Optimisation : décodage partiel pour les fichiers longs
  - Pour les fichiers > 10 min, le décodage complet peut être lent
  - Stratégie : utiliser symphonia seek pour échantillonner des segments répartis dans le fichier
  - Alternative plus simple : décoder tout mais en mode "skip" (lire seulement N samples par segment, pas tout)
  - Objectif : < 500ms pour un fichier de 5 minutes, < 1.5s pour 10+ minutes

- [ ] **3.5** Créer la commande Tauri `get_waveform(path: String, num_points: u32) -> Result<WaveformData, String>`
  - Async pour ne pas bloquer le thread principal
  - `num_points` par défaut : 250 (bon compromis résolution/performance)

- [ ] **3.6** Cache des waveforms pour éviter de recalculer
  - Stocker en mémoire dans `AppState` : `HashMap<String, WaveformData>` (clé = file_path)
  - Pas de persistence disque (recalcul rapide, pas besoin de cache permanent)
  - Limite : 50 waveforms en mémoire (LRU eviction si dépassé)
  - Vider le cache au changement de profil

- [ ] **3.7** Enregistrer la commande dans `main.rs`

## Frontend

- [ ] **3.8** Ajouter le wrapper dans `tauriCommands.ts`
  ```typescript
  export async function getWaveform(path: string, numPoints?: number): Promise<WaveformData>
  ```

- [ ] **3.9** Définir le type `WaveformData` dans `types/index.ts`
  ```typescript
  interface WaveformData {
    points: number[];    // 0.0-1.0 normalized RMS values
    duration: number;    // total seconds
    sampleRate: number;
  }
  ```

- [ ] **3.10** Créer le composant `WaveformDisplay.tsx` dans `src/components/common/`
  - **Rendu** : élément `<canvas>` ou `<svg>` (canvas recommandé pour la performance)
  - **Dimensions** : largeur = 100% du parent, hauteur fixe (~60px dans AddSoundModal, ~40px dans SoundDetails)
  - **Dessin de la courbe** :
    - Background : couleur sombre (bg-secondary)
    - Courbe remplie (area) : indigo semi-transparent (`rgba(99, 102, 241, 0.3)`)
    - Ligne de contour : indigo plus vif (`rgba(99, 102, 241, 0.7)`)
    - Les points RMS sont interpolés linéairement entre eux pour un rendu lisse
  - **Marqueur momentum** :
    - Ligne verticale jaune/orange sur la position du momentum actuel
    - Petit triangle en bas comme handle
    - Draggable horizontalement (mousedown + mousemove + mouseup)
    - Pendant le drag : afficher le timestamp en tooltip au-dessus du curseur
    - Au release : mettre à jour la valeur momentum (appeler le callback `onMomentumChange(seconds)`)
  - **Clic direct** : cliquer n'importe où sur la waveform = déplacer le marqueur à cette position
  - **Curseur de lecture** (optionnel) : si un preview est en cours, ligne verte qui avance
  - **État loading** : skeleton/shimmer pendant le calcul de la waveform
  - **État erreur** : si le calcul échoue, afficher le slider classique comme fallback

  **Props :**
  ```typescript
  interface WaveformDisplayProps {
    waveformData: WaveformData | null;
    momentum: number;                    // position actuelle du momentum (secondes)
    onMomentumChange: (seconds: number) => void;
    isLoading: boolean;
    playbackPosition?: number;           // position de lecture en cours (secondes, optionnel)
    height?: number;                     // hauteur en px (défaut: 60)
  }
  ```

- [ ] **3.11** Intégrer dans `AddSoundModal.tsx` — éditeur de momentum
  - Quand un son est ajouté (local ou YouTube) et que la durée est connue :
    - Lancer `getWaveform(filePath)` en background
    - Afficher le WaveformDisplay à la place du (ou en plus du) slider de momentum actuel
    - Le slider numérique reste visible en dessous pour l'ajustement fin
  - Quand la waveform est chargée :
    - L'utilisateur peut cliquer/drag sur la waveform pour positionner le momentum
    - La valeur numérique se met à jour en sync
  - **Fallback** : si le calcul de waveform échoue → afficher uniquement le slider classique (pas de régression)

- [ ] **3.12** Intégrer dans `SoundDetails.tsx` — éditeur de momentum existant
  - Même composant WaveformDisplay, mais plus compact (height: 40px)
  - Charger la waveform quand le son est sélectionné dans le KeyGrid
  - Le slider + input numérique restent pour l'ajustement fin
  - Le bouton play/stop preview existant continue de fonctionner
  - Si un preview est en cours → afficher le curseur de lecture sur la waveform

- [ ] **3.13** Synchronisation waveform ↔ slider ↔ input
  - Les trois contrôles (waveform click/drag, slider, input numérique) sont synchronisés bidirectionnellement
  - Modifier l'un met à jour les deux autres
  - Debounce de 200ms sur l'input numérique pour éviter les recalculs inutiles
  - Le drag sur la waveform met à jour en temps réel (pas de debounce)

## Performance

- **Calcul** : ~200-500ms pour un fichier de 5 minutes (symphonia est rapide)
- **Rendu** : canvas 2D avec 250 points = instantané, pas de problème de performance
- **Cache mémoire** : évite les recalculs quand on re-sélectionne un son déjà vu
- **Lazy loading** : la waveform n'est calculée que quand l'éditeur de momentum est visible

## Cas limites

- **Fichier très long** (>30 min, ex: mix ambient) → `num_points` reste à 250, chaque point couvre plus de temps, la waveform est moins détaillée mais lisible
- **Fichier très court** (<5 secondes) → waveform courte mais fonctionnelle, le slider classique reste plus pratique
- **Fichier silencieux** → waveform plate (tous les points à 0), pas de problème
- **Format non supporté par symphonia** → fallback vers le slider classique, pas de crash
- **Fichier en cours de download** (YouTube) → waveform affichée après que le download est terminé et que le fichier est accessible

## Fichiers impactés

| Fichier | Action |
|---------|--------|
| `src-tauri/src/audio/analysis.rs` | **Nouveau** — Calcul RMS waveform |
| `src-tauri/src/audio/mod.rs` | Exporter `analysis` |
| `src-tauri/src/types.rs` | Ajouter `WaveformData` |
| `src-tauri/src/commands.rs` | Ajouter commande `get_waveform` |
| `src-tauri/src/main.rs` | Enregistrer la commande |
| `src/components/common/WaveformDisplay.tsx` | **Nouveau** — Composant canvas |
| `src/utils/tauriCommands.ts` | Ajouter wrapper |
| `src/types/index.ts` | Ajouter type frontend |
| `src/components/Sounds/AddSoundModal.tsx` | Intégrer waveform dans l'éditeur momentum |
| `src/components/Sounds/SoundDetails.tsx` | Intégrer waveform dans l'éditeur momentum |
