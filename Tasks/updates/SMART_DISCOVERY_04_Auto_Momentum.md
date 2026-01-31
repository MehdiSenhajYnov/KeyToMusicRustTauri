# Section 4 — Auto-Momentum (marqueur suggéré sur la waveform)

> **Statut:** ⏳ PLANIFIE
> **Dépendance:** Section 3 (Waveform RMS)
> **Parent:** [SMART_DISCOVERY.md](./SMART_DISCOVERY.md)

> **Objectif:** Ajouter un marqueur de suggestion automatique sur la waveform de la section 3. L'algorithme détecte le point de "pré-pic" (fin de l'intro calme, début du buildup) et place un marqueur visuel que l'utilisateur peut accepter en un clic ou ignorer. C'est un hint, pas une décision automatique.

## Principe

Plutôt qu'un auto-remplissage aveugle du champ momentum (problème de calibration des seuils), l'auto-momentum est un **marqueur visuel sur la waveform**. L'utilisateur voit la courbe d'énergie ET le point suggéré. S'il est bien placé → un clic pour accepter. S'il est mal placé → l'utilisateur voit visuellement où est le bon point et clique dessus.

L'avantage : même si l'algorithme se trompe dans 30% des cas, l'utilisateur a la waveform pour corriger instantanément. L'auto-detect n'a pas besoin d'être parfait, juste utile.

## Rendu visuel

```
Énergie
  │
  │         ╱╲    ╱╲  ╱──╲
  │        ╱  ╲  ╱  ╲╱    ╲
  │  ─────╱    ╲╱          ╲─────
  │ ══════                        ← intro calme
  └──────────────────────────────── temps
     ▲  ▲
     │  │ marqueur auto (pointillé, discret)
     │ marqueur momentum user (solide, jaune)
```

- Le **marqueur auto-detect** est une ligne verticale en **pointillés** (ou couleur plus discrète, ex: blanc 30% opacity)
- Le **marqueur momentum user** reste la ligne solide jaune/orange (section 3)
- Les deux sont distincts visuellement
- Un petit bouton ou tooltip sur le marqueur auto : "Utiliser cette position" → place le momentum user dessus

## Algorithme de détection du pré-pic

```
1. Réutiliser les données RMS déjà calculées par la section 3 (pas de recalcul)
2. Calculer le gradient (dérivée) : grad[i] = points[i+1] - points[i]
3. Chercher le point d'inflexion :
   - Parcourir depuis le début
   - Ignorer les premières 3 secondes (si intro très courte, pas besoin de momentum)
   - Trouver le premier segment où :
     a) Le gradient est positif ET significatif (énergie qui monte)
     b) Les segments précédents avaient une énergie basse et stable (zone calme)
   - Ce point = début du buildup = momentum suggéré
4. Si aucun point d'inflexion trouvé → pas de marqueur affiché
```

> **Calibration des seuils :** C'est moins critique ici que dans un système purement automatique, car l'utilisateur VOIT la waveform et peut corriger. Des seuils approximatifs suffisent. Si le marqueur est décalé de 2-3 secondes, l'utilisateur le voit et ajuste en un clic.

## Backend (Rust)

- [ ] **4.1** Ajouter la fonction `detect_momentum_point` dans `src-tauri/src/audio/analysis.rs`
  - Fonction : `pub fn detect_momentum_point(waveform: &WaveformData) -> Option<f64>`
  - Prend les données waveform déjà calculées (pas de re-décodage audio)
  - Applique l'algorithme de gradient sur les points RMS
  - Retourne `Some(timestamp_seconds)` si un point d'inflexion est trouvé, `None` sinon
  - Paramètres internes (pas exposés à l'utilisateur) :
    ```rust
    const MIN_OFFSET_RATIO: f32 = 0.05;      // Ignorer les premiers 5% du morceau
    const GRADIENT_THRESHOLD: f32 = 0.03;     // Seuil de montée (relatif, normalisé)
    const QUIET_WINDOW: usize = 5;            // Segments calmes avant le pic
    const QUIET_THRESHOLD: f32 = 0.15;        // En dessous = "calme" (normalisé 0-1)
    ```
  - Ces seuils sont plus tolérants qu'avant car l'imprécision est compensée par la visualisation

- [ ] **4.2** Intégrer dans la commande `get_waveform` existante
  - Modifier `WaveformData` pour inclure le point suggéré :
    ```rust
    pub struct WaveformData {
        pub points: Vec<f32>,
        pub duration: f64,
        pub sample_rate: u32,
        pub suggested_momentum: Option<f64>,  // AJOUT: point de pré-pic détecté (secondes)
    }
    ```
  - Après le calcul de la waveform, appeler `detect_momentum_point(&waveform)` et stocker le résultat
  - Pas de commande Tauri supplémentaire : le suggested_momentum est retourné avec la waveform

- [ ] **4.3** Commande batch pour les imports multiples (AddSoundModal)
  - Créer `get_waveforms_batch(entries: Vec<WaveformRequest>) -> Result<HashMap<String, WaveformData>, String>`
    ```rust
    pub struct WaveformRequest {
        pub path: String,
        pub num_points: u32,
    }
    ```
  - Utilise un thread pool (2-4 threads) pour paralléliser
  - Chaque résultat inclut le `suggested_momentum`
  - Utile quand l'utilisateur importe 10+ sons d'un coup (playlist ou multi-file)

- [ ] **4.4** Enregistrer la commande batch dans `main.rs`

## Frontend

- [ ] **4.5** Mettre à jour le type `WaveformData` dans `types/index.ts`
  ```typescript
  interface WaveformData {
    points: number[];
    duration: number;
    sampleRate: number;
    suggestedMomentum: number | null;  // AJOUT
  }
  ```

- [ ] **4.6** Mettre à jour `WaveformDisplay.tsx` pour afficher le marqueur auto
  - **Nouveau prop :**
    ```typescript
    suggestedMomentum?: number | null;   // position suggérée (secondes)
    onAcceptSuggestion?: () => void;     // callback quand l'utilisateur accepte
    ```
  - **Marqueur visuel :**
    - Ligne verticale en pointillés, couleur discrète (blanc 30% ou indigo 40%)
    - Positionnée à `suggestedMomentum` sur l'axe X
    - Petit label au-dessus : timestamp formaté (ex: "0:45")
    - Au hover : tooltip "Cliquer pour utiliser cette position"
    - Au clic sur le marqueur : appeler `onAcceptSuggestion()` → met à jour le momentum à cette valeur
  - **Pas affiché si :**
    - `suggestedMomentum` est `null` (pas de point détecté)
    - `suggestedMomentum` est proche du momentum actuel (< 1 seconde d'écart) → déjà au bon endroit
    - Le momentum actuel a été modifié manuellement par l'utilisateur (ne pas insister)

- [ ] **4.7** Intégrer dans `AddSoundModal.tsx`
  - Quand la waveform est chargée et que `suggestedMomentum` est non-null :
    - Si le momentum actuel est à 0 (valeur par défaut, jamais modifié) :
      - Afficher le marqueur auto sur la waveform
      - NE PAS pré-remplir automatiquement le champ (l'utilisateur clique s'il veut)
    - Si le momentum a déjà été modifié manuellement :
      - Afficher le marqueur mais en encore plus discret (il est informatif, pas intrusif)

- [ ] **4.8** Intégrer dans `SoundDetails.tsx`
  - Même logique que AddSoundModal
  - Le bouton "baguette magique" pour re-détecter peut être remplacé par :
    - Si le marqueur auto est visible → cliquer dessus suffit
    - Sinon → un bouton discret "Suggérer un point" qui recalcule et affiche le marqueur

- [ ] **4.9** Ajouter le wrapper batch dans `tauriCommands.ts`
  ```typescript
  export async function getWaveformsBatch(entries: { path: string; numPoints?: number }[]): Promise<Record<string, WaveformData>>
  ```

## Comportement par type de son

| Type de son | Waveform | Marqueur auto |
|-------------|----------|---------------|
| OST avec intro calme → buildup | Courbe croissante visible | Marqueur au début du buildup |
| OST qui commence fort | Courbe haute dès le début | Pas de marqueur (null) |
| Ambient constant (pluie, vent) | Courbe plate | Pas de marqueur (null) |
| Son très court (<10s) | Waveform visible mais petite | Pas de marqueur (trop court) |
| Plusieurs buildups | Courbe avec plusieurs montées | Marqueur sur le premier |

## Avantage de cette approche vs auto-remplissage

- **Fiabilité** : l'utilisateur voit et décide, pas de mauvaise surprise
- **Rapidité** : même si le marqueur est mal placé, l'utilisateur voit le bon endroit sur la waveform et clique
- **Calibration non critique** : les seuils peuvent être approximatifs, la visualisation compense
- **Pas de régression** : si l'algo échoue → pas de marqueur → l'utilisateur utilise la waveform normalement

## Fichiers impactés

| Fichier | Action |
|---------|--------|
| `src-tauri/src/audio/analysis.rs` | Ajouter `detect_momentum_point` |
| `src-tauri/src/types.rs` | Ajouter `suggested_momentum` à `WaveformData`, ajouter `WaveformRequest` |
| `src-tauri/src/commands.rs` | Ajouter commande `get_waveforms_batch` |
| `src-tauri/src/main.rs` | Enregistrer la commande batch |
| `src/components/common/WaveformDisplay.tsx` | Ajouter marqueur auto-detect |
| `src/utils/tauriCommands.ts` | Ajouter wrapper batch |
| `src/types/index.ts` | Mettre à jour `WaveformData` |
| `src/components/Sounds/AddSoundModal.tsx` | Intégrer marqueur auto |
| `src/components/Sounds/SoundDetails.tsx` | Intégrer marqueur auto |
