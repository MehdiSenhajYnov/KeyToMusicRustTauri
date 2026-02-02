# Suggestion de Momentum — Détection & UX Moderne

> **Statut:** En attente d'implémentation
> **Type:** Bug Fix + UX Overhaul — La détection automatique du momentum ne fonctionne pas bien et la suggestion est quasi invisible dans l'UI
> **Priorité:** Haute

---

## Problème

La fonctionnalité qui détecte automatiquement un bon point de momentum (début d'écoute) et le suggère à l'utilisateur souffre de deux problèmes critiques :

1. **Détection peu fiable** — L'algorithme `detect_momentum_point()` est trop simpliste et retourne souvent `None` ou un point non pertinent
2. **Suggestion invisible** — Même quand un momentum est détecté, l'indicateur dans l'UI est quasi invisible (ligne pointillée blanche à 30% opacité, bouton "Use" en 9px à 50% opacité)

---

## Analyse UI/UX — Tendances 2026

### Contexte : Design d'Applications Audio en 2026

D'après les recherches sur les [tendances UI/UX 2026](https://motiongility.com/future-of-ui-ux-design/), plusieurs principes clés s'appliquent aux suggestions intelligentes dans les applications audio :

#### 1. **AI-First Design avec Contrôle Utilisateur**

> "AI is no longer an add-on—it's the core of the user experience. Designers focus on shaping AI behavior, transparency, and trust." — [Millipixels AI in UX](https://millipixels.com/blog/ai-in-ux-design)

**Application à KeyToMusic :**
- Le momentum suggéré doit être **explicite et visible** dès qu'il est détecté
- L'utilisateur doit pouvoir **accepter/refuser/modifier** la suggestion facilement
- La **transparence** : montrer pourquoi cette suggestion (ex: "Détecté après intro calme")

#### 2. **Progressive Disclosure**

> "Progressive disclosure defers advanced features to a secondary screen, preventing decision paralysis." — [Nielsen Norman Group](https://www.nngroup.com/articles/progressive-disclosure/)

**Application à KeyToMusic :**
- **Niveau 1** (toujours visible) : Indicateur visuel clair dans le waveform + badge "Suggéré: X.Xs"
- **Niveau 2** (au survol/focus) : Bouton d'action + explication contextuelle
- **Niveau 3** (optionnel) : Settings pour ajuster la sensibilité de détection

#### 3. **Microinteractions Contextuelles**

> "Research shows micro-interaction interfaces increase engagement by 45%, but only when they're functional, not decorative." — [UX Trends 2026](https://medium.com/@mohitphogat/the-ux-trends-2026-designers-need-to-know-not-just-guess-3269d023b0b7)

**Application à KeyToMusic :**
- Animation subtile lors de la détection (pulse sur le marqueur)
- Feedback visuel immédiat lors de l'acceptation (transition douce du marqueur suggéré → marqueur actuel)
- Toast de confirmation : "Momentum appliqué à 12.5s"

#### 4. **Accessibility-First**

> "Accessibility-first UI/UX design is now a core design principle." — [Big Human Design Trends](https://www.bighuman.com/blog/top-ui-ux-design-trends)

**Application à KeyToMusic :**
- Contraste élevé (WCAG AAA) pour les marqueurs
- Taille de texte lisible (minimum 11px)
- Actions accessibles au clavier (Tab + Enter pour accepter la suggestion)

#### 5. **Context-Aware Interfaces**

> "Products now understand who the user is, where they are, what device they're using, and what they're trying to achieve." — [DEV UX Trends](https://dev.to/pixel_mosaic/top-uiux-design-trends-for-2026-ai-first-context-aware-interfaces-spatial-experiences-166j)

**Application à KeyToMusic :**
- Auto-appliquer le momentum en mode Discovery (contexte : l'utilisateur explore rapidement)
- Suggérer mais ne pas auto-appliquer en édition manuelle (contexte : l'utilisateur ajuste finement)
- Adapter la visibilité selon le contexte (waveform 28px en discovery vs 40px en édition)

---

## Causes Identifiées (par priorité)

### P0 - CRITIQUE — Algorithme de Détection Peu Fiable

#### 1. Détection basée sur un gradient point-à-point
**Fichier:** `src-tauri/src/audio/analysis.rs:498-503`

```rust
let gradient = points[i + 1] - points[i];
if gradient > gradient_threshold {
    let timestamp = (i as f64 / points.len() as f64) * duration;
    return Some(timestamp);
}
```

**Impact:** Le gradient entre deux points adjacents est extrêmement sensible au bruit. Avec 200 points pour un morceau de 3 minutes, chaque point représente ~0.9 secondes. Un micro-pic de bruit (artefact de compression, claquement) déclenche un faux positif. Inversement, une montée progressive (fondu) ne sera jamais détectée car le gradient point-à-point reste < 0.05.

**Fix moderne (inspiré des DAWs 2026) :**
- **Gradient sur fenêtre glissante** : Comparer la moyenne de N points futurs vs. N points passés (N = 5-7)
- **Multi-seuils adaptatifs** : Calculer des percentiles dynamiques (P25, P50, P75) pour adapter aux caractéristiques du morceau
- **Validation de stabilité** : Vérifier que la section post-détection reste "active" pendant au moins 1-2 secondes
- **Détection de type d'intro** : Différencier intro silence/fade-in/noise/spoken → ajuster la stratégie

#### 2. Seuils statiques inadaptés
**Fichier:** `src-tauri/src/audio/analysis.rs:485-486`

```rust
let quiet_threshold = 0.15;
let gradient_threshold = 0.05;
```

**Impact:** Ces seuils fixes ne s'adaptent pas au contenu. Sur un morceau très dynamique (normalisé), le "silence" peut être à 0.2+. Sur un morceau calme, même le "fort" peut être < 0.15 après normalisation.

**Solution moderne :**
```rust
// Calculer les percentiles de l'amplitude
let sorted_points = points.iter().cloned().sorted();
let p25 = sorted_points[points.len() / 4];        // Quiet threshold
let p75 = sorted_points[points.len() * 3 / 4];    // Active threshold
let median = sorted_points[points.len() / 2];

// Seuils dynamiques basés sur la distribution
let quiet_threshold = p25 * 1.2;                  // 20% au-dessus du P25
let active_threshold = median.max(0.3);           // Au moins 30% de l'amplitude max
let gradient_threshold = (p75 - p25) * 0.15;      // 15% de la plage dynamique
```

#### 3. Retour au premier match sans validation de qualité
**Fichier:** `src-tauri/src/audio/analysis.rs:500-503`

Le premier point qui satisfait les conditions est retourné immédiatement. Aucune vérification que c'est un bon momentum.

**Solution multi-passes :**
```rust
// Pass 1: Collecter tous les candidats potentiels
let mut candidates = Vec::new();
for i in (skip + window_size)..points.len().saturating_sub(lookahead) {
    if meets_momentum_criteria(points, i, thresholds) {
        let quality_score = compute_quality_score(points, i, lookahead);
        candidates.push((i, quality_score));
    }
}

// Pass 2: Sélectionner le meilleur candidat (score le plus élevé)
candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
if let Some((best_idx, score)) = candidates.first() {
    if *score > MIN_QUALITY_SCORE {
        return Some((*best_idx as f64 / points.len() as f64) * duration);
    }
}
```

**Quality Score = f(amplitude_rise, sustained_energy, position_penalty)**

#### 4. Trop peu de points en mode Discovery
**Fichier:** `src-tauri/src/commands.rs:1207`

```rust
analysis::compute_waveform_sampled(&wf_path, 50)
```

Seulement 50 points en discovery (vs. 200 en normal). Avec 50 points, la résolution est trop faible (~6s/point pour un morceau de 5 minutes).

**Fix :** Passer à **150-200 points** même en discovery. Le coût CPU est négligeable avec `compute_waveform_sampled` (seek-based, ~40x plus rapide).

---

### P1 - HAUTE — UX Invisible : Anti-Pattern 2026

#### 5. Indicateur visuel trop discret (violations WCAG)
**Fichier:** `src/components/common/WaveformDisplay.tsx:179-203`

```tsx
// Ligne pointillée
ctx.strokeStyle = "rgba(255, 255, 255, 0.3)";  // 30% opacité ❌ WCAG fail
ctx.lineWidth = 1;                              // 1px ❌ Trop fin

// Label
ctx.fillStyle = "rgba(255, 255, 255, 0.5)";     // 50% opacité ❌ Faible contraste
ctx.font = "9px sans-serif";                     // 9px ❌ Illisible
```

**Contraste actuel :** ~1.8:1 (WCAG Level A requis : 3:1, AAA : 7:1)

**Solution moderne (style 2026) :**

```tsx
// Couleur d'accent distinctive (cyan = suggestions AI dans les DAWs modernes)
const suggestedColor = "rgba(34, 211, 238, 0.85)";  // cyan-400 à 85% opacité
ctx.strokeStyle = suggestedColor;
ctx.lineWidth = 2;                                   // 2px pour visibilité
ctx.setLineDash([4, 2]);                             // Dash pattern plus marqué

// Label avec background pour contraste maximum
ctx.font = "bold 11px Inter, sans-serif";            // 11px bold
const label = `Suggéré: ${suggestedMomentum.toFixed(1)}s`;
const labelW = ctx.measureText(label).width;
const labelX = Math.min(sx + 4, w - labelW - 4);

// Background semi-opaque pour le label
ctx.fillStyle = "rgba(0, 0, 0, 0.8)";
ctx.fillRect(labelX - 2, 2, labelW + 4, 14);

// Texte du label
ctx.fillStyle = suggestedColor;
ctx.fillText(label, labelX, 12);

// Effet pulse subtil (microinteraction)
if (isNewSuggestion) {
  ctx.shadowColor = suggestedColor;
  ctx.shadowBlur = 8;
}
```

**Contraste nouveau :** ~11:1 (WCAG AAA ✅)

#### 6. Bouton "Use" invisible — Anti-Pattern Progressive Disclosure
**Fichier:** `src/components/common/WaveformDisplay.tsx:286-301`

```tsx
<button className="absolute top-0.5 text-[9px] text-white/50 hover:text-white/80 bg-black/30">
  Use
</button>
```

**Problèmes multiples :**
- Taille 9px = illisible sur écran 1080p+ (standard 2026 : 11px minimum)
- Opacité 50% = invisible sur waveform sombre
- Positionnement `top-0.5` dans waveform 40px = caché
- Action critique cachée dans un bouton 3 lettres

**Solution moderne : Badge d'action avec Progressive Disclosure**

**Option A : Badge externe visible (Recommandé)**

Dans `SoundDetails.tsx` ligne ~458, remplacer la ligne "Mom:" par :

```tsx
<div className="flex items-center gap-2 text-xs">
  <span className="text-text-secondary whitespace-nowrap">Momentum:</span>

  {/* Badge de suggestion (seulement si suggestion disponible et différente) */}
  {waveform?.suggestedMomentum != null &&
   Math.abs(waveform.suggestedMomentum - sound.momentum) > 0.3 && (
    <button
      onClick={() => {
        handleMomentumChange(sound.id, Math.round(waveform.suggestedMomentum * 10) / 10);
        addToast({
          type: "success",
          message: `Momentum appliqué : ${waveform.suggestedMomentum.toFixed(1)}s`
        });
      }}
      className="flex items-center gap-1.5 px-2 py-0.5 rounded-full
                 bg-cyan-500/20 hover:bg-cyan-500/30
                 border border-cyan-500/40 hover:border-cyan-500/60
                 text-cyan-400 hover:text-cyan-300
                 transition-all duration-200 ease-out
                 text-[10px] font-medium
                 group"
      title={`Momentum suggéré: ${waveform.suggestedMomentum.toFixed(1)}s\nCliquer pour appliquer`}
    >
      {/* Icône sparkle (suggestion AI) */}
      <svg className="w-3 h-3 animate-pulse" fill="currentColor" viewBox="0 0 20 20">
        <path d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z" />
      </svg>
      <span>{waveform.suggestedMomentum.toFixed(1)}s</span>
      {/* Chevron right (action) */}
      <svg className="w-2.5 h-2.5 opacity-60 group-hover:opacity-100 transition-opacity" fill="currentColor" viewBox="0 0 20 20">
        <path fillRule="evenodd" d="M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 011.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z" clipRule="evenodd" />
      </svg>
    </button>
  )}

  {/* Input momentum actuel */}
  <input type="number" ... />
</div>
```

**Bénéfices design :**
- ✅ **Visible** : Badge distinct avec couleur accent cyan (convention 2026 pour AI)
- ✅ **Explicite** : Affiche la valeur suggérée directement
- ✅ **Actionnable** : Clic direct = application (1 action vs. 2)
- ✅ **Progressive Disclosure** : N'apparaît que quand pertinent
- ✅ **Microinteraction** : Pulse subtil + transition hover
- ✅ **Feedback** : Toast de confirmation après application

**Option B : Tooltip contextuel moderne (Alternative)**

```tsx
{/* Dans WaveformDisplay.tsx, remplacer le bouton "Use" par un tooltip */}
{suggestedMomentum != null && Math.abs(suggestedMomentum - momentum) > 0.3 && (
  <div
    className="absolute z-10 flex items-center gap-1.5 px-2.5 py-1.5
               bg-bg-secondary/95 backdrop-blur-sm
               border border-cyan-500/40 rounded-lg shadow-lg
               transition-all duration-200 ease-out"
    style={{
      left: `${Math.min((suggestedMomentum / waveformData.duration) * 100, 80)}%`,
      top: -32,  // Au-dessus du waveform
    }}
  >
    <div className="flex flex-col gap-0.5">
      <span className="text-[9px] text-text-muted uppercase tracking-wide">Suggéré</span>
      <span className="text-xs text-cyan-400 font-semibold">
        {suggestedMomentum.toFixed(1)}s
      </span>
    </div>
    <button
      onClick={(e) => {
        e.stopPropagation();
        onAcceptSuggestion();
      }}
      className="px-2 py-1 rounded bg-cyan-500 hover:bg-cyan-400
                 text-white text-[11px] font-medium
                 transition-colors duration-150"
    >
      Appliquer
    </button>
  </div>
)}
```

#### 7. Condition de masquage trop restrictive
**Fichier:** `src/components/common/WaveformDisplay.tsx:184`

```tsx
Math.abs(suggestedMomentum - momentum) > 1  // ❌ Masque les suggestions < 1s
```

**Problème :** Suggestion de 0.8s avec momentum=0 → pas affichée, alors qu'elle est utile.

**Fix :**
```tsx
Math.abs(suggestedMomentum - momentum) > 0.3  // ✅ Seuil plus réaliste (300ms)
```

**Rationale :** En audio, une différence de 300ms est audible et significative. Seuil de 1s trop élevé.

---

### P2 - MOYENNE — Feedback Proactif Absent

#### 8. Aucune indication de découverte

L'utilisateur ne sait jamais qu'un momentum a été détecté. Pattern 2026 : **"Don't make me think"** — l'information doit être évidente.

**Solutions modernes :**

**A. Indicateur visuel dans la liste de sons**

Dans `SoundDetails.tsx`, ajouter un petit badge à côté du nom du son :

```tsx
<div className="flex items-center gap-2 mb-1">
  <span className="text-text-primary font-medium truncate flex-1">
    {sound.name}
  </span>

  {/* Badge "Momentum détecté" */}
  {waveform?.suggestedMomentum != null &&
   Math.abs(waveform.suggestedMomentum - sound.momentum) > 0.3 && (
    <span className="shrink-0 px-1.5 py-0.5 rounded text-[9px] font-medium
                     bg-cyan-500/15 text-cyan-400 border border-cyan-500/30
                     flex items-center gap-1"
          title={`Momentum suggéré : ${waveform.suggestedMomentum.toFixed(1)}s`}>
      <svg className="w-2.5 h-2.5" fill="currentColor" viewBox="0 0 20 20">
        <path d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z" />
      </svg>
      Auto
    </span>
  )}
</div>
```

**B. Toast lors de la détection (première fois seulement)**

Ajouter dans `AddSoundModal.tsx` après l'ajout d'un son avec waveform :

```tsx
// Après FileWaveform fetch
if (waveform.suggestedMomentum != null && !hasShownSuggestionToast) {
  addToast({
    type: "info",
    message: `💡 Momentum suggéré détecté : ${waveform.suggestedMomentum.toFixed(1)}s`,
    duration: 4000,
  });
  localStorage.setItem('has_seen_momentum_suggestion', 'true');
}
```

**C. Animation pulse sur le marqueur (nouvelle détection)**

```tsx
// Dans WaveformDisplay.tsx, ajouter un état pour l'animation
const [isNewSuggestion, setIsNewSuggestion] = useState(false);

useEffect(() => {
  if (suggestedMomentum != null) {
    setIsNewSuggestion(true);
    const timer = setTimeout(() => setIsNewSuggestion(false), 2000);
    return () => clearTimeout(timer);
  }
}, [suggestedMomentum]);

// Dans le canvas drawing
if (isNewSuggestion) {
  const pulse = Math.sin(Date.now() / 200) * 0.2 + 0.8;  // Pulse 0.8-1.0
  ctx.globalAlpha = pulse;
}
```

---

## Plan d'Implémentation — Architecture Moderne

### Phase 1 : Algorithme de Détection Intelligent

**Objectif :** Passer d'un algorithme naïf à un système multi-passes avec seuils adaptatifs

- [ ] **1.1** Refactoriser `detect_momentum_point()` dans `analysis.rs:478-508`

  **Nouvelle architecture :**

  ```rust
  fn detect_momentum_point(points: &[f32], duration: f64) -> Option<f64> {
      // Step 1: Compute adaptive thresholds
      let thresholds = compute_adaptive_thresholds(points);

      // Step 2: Find candidate momentum points
      let candidates = find_momentum_candidates(points, &thresholds);

      // Step 3: Score and rank candidates
      let scored = score_candidates(points, candidates, &thresholds);

      // Step 4: Select best candidate above quality threshold
      select_best_momentum(scored, duration, points.len())
  }
  ```

  - [ ] Implémenter `compute_adaptive_thresholds()` — Calcul des percentiles P25/P50/P75
  - [ ] Implémenter `find_momentum_candidates()` — Gradient sur fenêtre glissante (5-7 points)
  - [ ] Implémenter `score_candidates()` — Quality score = f(amplitude_rise, sustained_energy, position)
  - [ ] Implémenter `select_best_momentum()` — Sélection avec MIN_QUALITY_SCORE
  - [ ] Ajouter tests unitaires avec cas edge (silence total, bruit constant, fade-in)

- [ ] **1.2** Augmenter la résolution en mode Discovery
  - [ ] `commands.rs:1207` — Passer de 50 à **150-200 points**
  - [ ] Mesurer impact perf (devrait être <10ms avec compute_waveform_sampled)

- [ ] **1.3** Ajouter logging pour debugging
  - [ ] Log candidates trouvés avec leurs scores
  - [ ] Log seuils adaptatifs calculés
  - [ ] Format : `[Momentum] Detected at 12.5s (score: 0.87, candidates: 3)`

**Estimation complexité :** O(n) où n = num_points (identique à l'algo actuel)

---

### Phase 2 : UX Moderne — Visible & Actionnable

**Objectif :** Transformer une suggestion invisible en feature découvrable et utilisable

#### 2.1 Waveform Display — Indicateurs Visuels WCAG AAA

- [ ] **2.1.1** Améliorer le marqueur suggéré (`WaveformDisplay.tsx:179-203`)
  - [ ] Couleur distinctive : `rgba(34, 211, 238, 0.85)` (cyan-400)
  - [ ] Épaisseur ligne : 2px (vs 1px)
  - [ ] Pattern dash : `[4, 2]` (vs `[3, 3]`)
  - [ ] Label : 11px bold avec background opaque
  - [ ] Format label : `"Suggéré: 12.5s"` (vs `"12.5s"`)

- [ ] **2.1.2** Ajouter effet pulse pour nouvelle suggestion
  - [ ] État `isNewSuggestion` (2s duration)
  - [ ] Shadow glow avec `ctx.shadowBlur = 8`
  - [ ] Animation pulse subtile (sin wave 0.8-1.0)

- [ ] **2.1.3** Ajuster condition de masquage
  - [ ] Changer seuil de 1.0s → 0.3s
  - [ ] Toujours afficher si momentum actuel = 0 et suggestion > 0

#### 2.2 Badge d'Action Externe (Progressive Disclosure)

- [ ] **2.2.1** Créer composant `MomentumSuggestionBadge` réutilisable

  ```tsx
  // src/components/common/MomentumSuggestionBadge.tsx
  interface Props {
    suggestedMomentum: number;
    currentMomentum: number;
    onApply: () => void;
    size?: 'sm' | 'md';  // sm pour discovery (28px), md pour edition (40px)
  }
  ```

  - [ ] Design : Badge pill avec icône sparkle + valeur + chevron
  - [ ] Couleurs : bg-cyan-500/20, border-cyan-500/40, text-cyan-400
  - [ ] Hover state : bg-cyan-500/30, border-cyan-500/60
  - [ ] Animation : pulse sur l'icône (CSS animate-pulse)
  - [ ] Accessibilité : Title tooltip explicatif, focus ring, keyboard (Enter)

- [ ] **2.2.2** Intégrer dans `SoundDetails.tsx` (ligne ~458)
  - [ ] Placer badge entre label "Momentum:" et input number
  - [ ] Condition : `suggestedMomentum != null && abs(diff) > 0.3`
  - [ ] onApply : `handleMomentumChange + toast confirmation`
  - [ ] Toast : `"Momentum appliqué : 12.5s"` (type success, 3s)

- [ ] **2.2.3** Intégrer dans `AddSoundModal.tsx` (bulk add)
  - [ ] Afficher badge dans la liste des fichiers à côté du nom
  - [ ] Auto-appliquer lors du bulk add si mode Auto-Momentum actif
  - [ ] Indicateur visuel : badge "Auto" si auto-appliqué

- [ ] **2.2.4** Adapter pour Discovery (`DiscoveryPanel.tsx`)
  - [ ] Version compacte (size='sm') dans la suggestion card
  - [ ] Inline avec le waveform (row 2 de la card)

#### 2.3 Indicateurs Proactifs (Discoverability)

- [ ] **2.3.1** Badge dans liste de sons (`SoundDetails.tsx`)
  - [ ] Mini badge "Auto" à côté du nom du son
  - [ ] Style : bg-cyan-500/15, border-cyan-500/30, 9px
  - [ ] Tooltip : "Momentum suggéré : X.Xs"
  - [ ] Condition : suggestion disponible ET différente de momentum actuel

- [ ] **2.3.2** Toast éducatif (première détection)
  - [ ] Toast info avec icône 💡
  - [ ] Message : "Momentum suggéré détecté : X.Xs — cliquez sur le badge pour l'appliquer"
  - [ ] Affichage : 1 seule fois (localStorage flag)
  - [ ] Trigger : Première waveform loaded avec suggestion dans AddSoundModal

- [ ] **2.3.3** Animation lors de l'application
  - [ ] Transition smooth du marqueur suggéré → marqueur actuel
  - [ ] Durée : 300ms ease-out
  - [ ] Badge disparaît avec fade-out 200ms

---

### Phase 3 : Auto-Application Contextuelle (Optionnel)

**Objectif :** Appliquer automatiquement le momentum suggéré dans les contextes appropriés

- [ ] **3.1** Mode Discovery — Auto-apply par défaut
  - [ ] Dans `DiscoveryPanel.tsx`, lors de l'enrichment initial
  - [ ] Si `waveform.suggestedMomentum != null`, set `s.suggestedMomentum = waveform.suggestedMomentum`
  - [ ] Badge "Auto" visible pour indiquer l'auto-application
  - [ ] L'utilisateur peut override en drag sur le waveform

- [ ] **3.2** Bulk Add — Auto-apply si mode Auto-Momentum actif
  - [ ] Dans `AddSoundModal.tsx`, lors de l'ajout de multiples fichiers
  - [ ] Check `config.autoMomentum` (déjà existant dans AppConfig)
  - [ ] Si true : auto-appliquer suggère à chaque fichier
  - [ ] Toast récap : "3 sons ajoutés — momentum auto-appliqué"

- [ ] **3.3** Edit Manuel — Suggérer sans auto-apply
  - [ ] Dans `SoundDetails.tsx` (édition individuelle)
  - [ ] Ne JAMAIS auto-appliquer — toujours montrer le badge
  - [ ] Respecter l'intention de l'utilisateur (contexte : ajustement fin)

- [ ] **3.4** Undo/Redo compatible
  - [ ] S'assurer que l'auto-application est captée dans l'historique
  - [ ] Test : Ajouter son avec auto-momentum → Undo → momentum revient à 0

---

### Phase 4 : Polish & Feedback

- [ ] **4.1** Ajouter setting pour sensibilité de détection (optionnel)
  - [ ] `SettingsModal.tsx` → Section "Audio"
  - [ ] Slider : "Sensibilité momentum" (Low / Medium / High)
  - [ ] Impact : Ajuste MIN_QUALITY_SCORE dans le backend

- [ ] **4.2** Analytics logging (optionnel)
  - [ ] Track combien de suggestions sont appliquées vs. ignorées
  - [ ] Format : `[Analytics] Momentum suggestion applied: true/false`
  - [ ] Objectif : Mesurer l'adoption de la feature

- [ ] **4.3** Tests utilisateurs
  - [ ] Test A/B : Badge externe vs. Tooltip inline
  - [ ] Mesure : Taux d'application des suggestions
  - [ ] Feedback qualitatif : "Avez-vous remarqué les suggestions de momentum ?"

---

## Fichiers Concernés

### Backend (Rust)

| Fichier | Modifications | Lignes | Complexité |
|---------|--------------|--------|------------|
| `src-tauri/src/audio/analysis.rs` | Refonte complète `detect_momentum_point()` + helpers | 478-508 → ~150 lignes | Haute |
| `src-tauri/src/commands.rs` | Augmenter num_points discovery : 50 → 150 | 1207 | Triviale |

### Frontend (React/TypeScript)

| Fichier | Modifications | Lignes | Complexité |
|---------|--------------|--------|------------|
| `src/components/common/WaveformDisplay.tsx` | Marqueur visible, effet pulse, condition masquage | 179-203, 286-301 | Moyenne |
| `src/components/common/MomentumSuggestionBadge.tsx` | **[NOUVEAU]** Composant badge réutilisable | 0 → ~80 lignes | Moyenne |
| `src/components/Sounds/SoundDetails.tsx` | Intégration badge + mini badge liste | 458-490 | Moyenne |
| `src/components/Sounds/AddSoundModal.tsx` | Badge + auto-apply + toast éducatif | 60-110 | Moyenne |
| `src/components/Discovery/DiscoveryPanel.tsx` | Badge compact + auto-apply initial | 860-883 | Faible |
| `src/stores/toastStore.ts` | (Aucune modif, déjà compatible) | - | - |

### Styling

| Fichier | Modifications |
|---------|--------------|
| `tailwind.config.js` | (Aucune modif, cyan-400/500 déjà disponibles) |

---

## Maquettes UX — Avant/Après

### Avant (État actuel)
```
[Waveform avec ligne pointillée blanche 30% opacité — INVISIBLE]
"Use" (9px, 50% opacité) — INVISIBLE

Mom: [12.5] s [━━━━━━━━━━━━] 180.0s [▶]
```

### Après (Nouvelle UX)
```
[Waveform avec ligne cyan 85% opacité, 2px, label "Suggéré: 34.2s"]

Momentum: [✨ 34.2s →] [12.5] s [━━━━━━━━━━━━] 180.0s [▶]
          ↑ Badge clickable cyan pulsant

[Toast] ✅ Momentum appliqué : 34.2s
```

**Badge détaillé :**
```
┌─────────────────────────┐
│ ✨ 34.2s →             │  ← Hover : bg plus clair, chevron animé
│ (sparkle) (value) (→)   │
│                         │
│ Border: cyan-500/40     │
│ BG: cyan-500/20         │
│ Text: cyan-400          │
└─────────────────────────┘
```

---

## Critères de Succès

### Quantitatifs
- [ ] **Détection** : Taux de détection réussie > 80% (vs ~30% actuel estimé)
- [ ] **Visibilité** : Contraste WCAG AAA (7:1+) pour tous les indicateurs
- [ ] **Performance** : Calcul waveform + détection < 50ms (même avec 200 points)
- [ ] **Adoption** : >50% des suggestions appliquées par les utilisateurs (métrique à tracker)

### Qualitatifs
- [ ] L'utilisateur **remarque** la suggestion dès qu'elle apparaît (badge visible)
- [ ] L'utilisateur **comprend** ce que c'est (label "Suggéré", icône sparkle)
- [ ] L'utilisateur peut **agir** en 1 clic (badge clickable)
- [ ] L'utilisateur reçoit un **feedback** immédiat (toast + transition)

---

## Références & Sources

### Tendances UI/UX 2026
- [Future Of UI UX Design: 2026 Trends & New AI Workflow](https://motiongility.com/future-of-ui-ux-design/)
- [7 New Rules of AI in UX Design for 2026](https://millipixels.com/blog/ai-in-ux-design)
- [Progressive Disclosure Matters: Applying 90s UX Wisdom to 2026 AI Agents](https://aipositive.substack.com/p/progressive-disclosure-matters)
- [Progressive Disclosure - Nielsen Norman Group](https://www.nngroup.com/articles/progressive-disclosure/)
- [The UX Trends 2026 Designers Need to Know](https://medium.com/@mohitphogat/the-ux-trends-2026-designers-need-to-know-not-just-guess-3269d023b0b7)

### Waveform & Audio UI
- [ElevenLabs UI - Waveform Component](https://ui.elevenlabs.io/docs/components/waveform)
- [Wavesurfer.js - Audio Waveform Player](https://wavesurfer.xyz/)
- [Waves UI - Interactive Temporal Visualizations](https://github.com/wavesjs/waves-ui)

### Design Patterns
- [Microinteractions: Enhancing User Experience](https://designlab.com/blog/microinteractions-enhancing-user-experience-through-small-details)
- [AI UX Patterns Guide](https://www.aiuxpatterns.com/)
- [12 UI/UX Design Trends That Will Dominate 2026](https://www.index.dev/blog/ui-ux-design-trends)

---

## Notes Techniques

### Performance
- `compute_waveform_sampled` avec 200 points : ~10-20ms (seek-based, 40x plus rapide)
- Percentile calculation : O(n log n) pour sorting (négligeable avec n=200)
- Gradient windowed : O(n × window_size) = O(n) avec window constant
- **Impact total** : <5ms supplémentaires vs. algo actuel

### Compatibilité
- Waveforms normaux : 200 points (`SoundDetails.tsx:107`, `AddSoundModal.tsx:67`)
- Discovery : 50 → 150-200 points (`commands.rs:1207`)
- Cache waveform : Déjà en place (LRU 50 entries, disk persistence)
- Types : `WaveformData.suggestedMomentum: Option<f64>` déjà existant

### Accessibilité
- Contraste WCAG AAA : 7:1+ (cyan-400 sur bg-primary)
- Taille texte minimum : 11px (vs 9px actuel)
- Focus ring : Tailwind default (2px offset ring-2 ring-offset-2)
- Keyboard : Tab pour atteindre badge, Enter pour appliquer
- Screen reader : Title/aria-label sur tous les éléments interactifs

### Design Tokens (Tailwind)
```js
// Déjà disponibles dans tailwind.config.js
colors: {
  'accent-primary': '#6366f1',      // Indigo (actions primaires)
  cyan: {                           // Built-in Tailwind
    400: '#22d4ee',                 // Suggestions AI
    500: '#06b6d4',
  }
}
```
