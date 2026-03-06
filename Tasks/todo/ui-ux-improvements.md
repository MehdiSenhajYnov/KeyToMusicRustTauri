# 🎨 Améliorations UI/UX pour KeyToMusic

**Objectif :** Transformer l'interface actuelle en une application professionnelle, belle, claire et intuitive, avec de la personnalité tout en restant sobre, sans perdre aucune fonctionnalité.

---

## 1. HIÉRARCHIE VISUELLE & RESPIRATION

**Problème actuel :** Information très dense, éléments serrés, manque de respiration visuelle

**Améliorations :**
- [ ] **Augmenter les espacements** entre sections (passer de `gap-3` à `gap-4/5` dans les zones principales)
- [ ] **Ajouter une hiérarchie typographique claire** :
  - Titres de sections : `text-base` au lieu de `text-xs`
  - Sous-titres : `text-sm font-medium`
  - Body : `text-sm` au lieu de `text-xs`
- [ ] **Grouper visuellement** les contrôles reliés dans des cards avec background légèrement différent
- [ ] **Réduire le nombre d'éléments visibles simultanément** en utilisant des accordéons/collapsibles pour les sections avancées

---

## 2. SIDEBAR - RÉORGANISATION

**Problème :** 4 sections (Profiles + Controls + Now Playing + Discovery) se battent pour l'espace vertical

**Améliorations :**
- [ ] **Tabs en haut de la sidebar** : "Library" / "Discovery" / "Now Playing"
  - Tab Library : Profiles + Controls (toujours visibles)
  - Tab Discovery : DiscoveryPanel en pleine hauteur
  - Tab Now Playing : Liste des pistes en lecture
- [ ] **Élargir légèrement la sidebar** : `w-64` (256px) au lieu de `w-56` (224px)
- [ ] **Better empty states** dans Now Playing (actuellement juste une liste vide)

**Fichiers concernés :**
- `src/components/Layout/Sidebar.tsx`
- `src/components/Profiles/ProfileSelector.tsx`
- `src/components/Controls/NowPlaying.tsx`
- `src/components/Discovery/DiscoveryPanel.tsx`

---

## 3. KEYGRID - AMÉLIORATION DE LA LISIBILITÉ

**Problèmes :** Cards de taille variable, filtrage trop subtil, multi-sélection obscure

**Améliorations :**
- [ ] **Grille fixe** au lieu de flex : `grid grid-cols-[repeat(auto-fill,minmax(140px,1fr))]`
- [ ] **Cards uniformes** avec aspect-ratio fixe
- [ ] **État filtré plus visible** : `opacity-50` + léger `blur-[1px]` au lieu de `opacity-30`
- [ ] **Indicateur de multi-sélection** :
  - Counter badge en haut à droite : "3 selected"
  - Checkbox overlay sur les cards sélectionnées
  - Hint text : "Ctrl+Click to select multiple"
- [ ] **Hover state amélioré** : Légère élévation (`shadow-md`) + border accent
- [ ] **Track indicator redesign** : Au lieu de "2T", montrer des mini-badges colorés pour chaque track

**Fichiers concernés :**
- `src/components/Keys/KeyGrid.tsx`

---

## 4. SOUNDDETAILS - SIMPLIFICATION

**Problème :** 586 lignes, très dense, tous les contrôles ont le même poids visuel

**Améliorations :**
- [ ] **Tabs par sound** si plusieurs sounds sur la clé (au lieu de liste verticale)
- [ ] **Sections collapsibles** :
  - "Playback Settings" (momentum, volume, loop)
  - "Assignment" (key, track)
  - "Advanced" (waveform editing)
- [ ] **Momentum editor unifié** : Au lieu de 4 contrôles (play, input, slider, waveform), un seul composant intégré :
  - Waveform en grand avec overlay de tous les contrôles
  - Play button en overlay sur la waveform
  - Slider intégré sous la waveform
- [ ] **Actions principales en haut** : Preview, Remove comme floating action buttons
- [ ] **Waveform plus grande** : Hauteur min de 120px au lieu de variable

**Fichiers concernés :**
- `src/components/Sounds/SoundDetails.tsx`
- `src/components/Sounds/MultiKeyDetails.tsx`

---

## 5. MODALES - REFONTE COMPLÈTE

### AddSoundModal

**Améliorations :**
- [ ] **Wizard multi-étapes** au lieu de tout afficher :
  1. Source selection (Local / YouTube) - grand cards cliquables
  2. File selection / Search
  3. Configuration (momentum, key assignment)
  4. Review & Add
- [ ] **YouTube preview plus visible** : Card design avec thumbnail si disponible
- [ ] **Momentum suggéré mis en avant** : Animation + badge "AI Suggested" ✨

**Fichiers concernés :**
- `src/components/Sounds/AddSoundModal.tsx`
- `src/components/Sounds/SearchResultPreview.tsx`

### SettingsModal

**Améliorations :**
- [ ] **Navigation latérale par catégorie** :
  - Audio
  - Controls
  - Keyboard Shortcuts
  - Data Management
  - About
- [ ] **Shortcuts section redesign** : Table layout au lieu de liste, groupé par catégorie
- [ ] **Key capture plus clair** : Mode "Recording" avec dot rouge pulsant
- [ ] **Preview des changements** : Avant/après pour les settings critiques

**Fichiers concernés :**
- `src/components/Settings/SettingsModal.tsx`
- `src/components/Settings/DislikedVideosPanel.tsx`

### ConfirmDialog

**Améliorations :**
- [ ] **Variants visuels** :
  - Danger (rouge) pour actions destructives
  - Warning (amber) pour actions réversibles
  - Info (bleu) pour confirmations générales
- [ ] **Keyboard shortcuts** : Enter = Confirm, Escape = Cancel
- [ ] **Customizable** : Titres et labels de boutons personnalisables
- [ ] **Timer visible** : Countdown visuel si auto-timeout

**Fichiers concernés :**
- `src/components/ConfirmDialog.tsx`
- `src/stores/confirmStore.ts`

---

## 6. WAVEFORM - INTERACTIVITÉ ENRICHIE

**Améliorations :**
- [ ] **Click to jump** : Click n'importe où sur la waveform pour set momentum
- [ ] **Keyboard shortcuts** :
  - `←/→` : Nudge ±0.1s
  - `Shift+←/→` : Nudge ±1s
  - `Space` : Preview from momentum point
- [ ] **Zoom controls** : `+/-` ou pinch-to-zoom
- [ ] **Minimap** pour fichiers longs (>5min)
- [ ] **Amplitude markers** : Lignes horizontales pour les seuils de détection
- [ ] **Hover preview** : Tooltip montrant le timestamp en hover

**Fichiers concernés :**
- `src/components/common/WaveformDisplay.tsx`

---

## 7. FEEDBACK UTILISATEUR - AMÉLIORATION

### Toasts

**Améliorations :**
- [ ] **Stacking limit** : Max 5, plus anciens fade out
- [ ] **Durées variables** :
  - Success : 3s
  - Info : 4s
  - Warning : 5s
  - Error : 6s (+ keep open)
- [ ] **Action buttons** : "Undo", "View Details", "Retry"
- [ ] **Progress toasts** : Pour les opérations longues (batch downloads, export)
- [ ] **Swipe to dismiss** : Gesture sur desktop aussi

**Fichiers concernés :**
- `src/stores/toastStore.ts`
- `src/components/Toast/ToastContainer.tsx`

### Loading states

**Améliorations :**
- [ ] **Shimmer effect** sur les skeletons
- [ ] **Estimated time** pour les opérations longues
- [ ] **Cancellable** : Tous les loadings doivent avoir un Cancel button

**Fichiers concernés :**
- `src/components/Layout/MainContent.tsx`
- `src/components/Discovery/DiscoveryPanel.tsx`
- `src/components/Export/ExportProgress.tsx`

### Animations

**Améliorations :**
- [ ] **Confetti** pour les milestones (première sound ajoutée, 10 sounds, etc.)
- [ ] **Success checkmark animation** après add sound
- [ ] **Smoother transitions** : Spring animations au lieu de linear
- [ ] **Micro-interactions** :
  - Buttons : Scale + shadow on press
  - Sliders : Haptic-style bump at increments
  - Key press : Ripple effect from center

**Fichiers concernés :**
- `src/index.css`
- `tailwind.config.js`

---

## 8. DISCOVERY PANEL - MISE EN AVANT

**Problème :** Feature puissante cachée en bas de la sidebar

**Améliorations :**
- [ ] **Déplacer dans un onglet dédié** (voir point 2)
- [ ] **Carousel redesign** :
  - Cards plus grandes avec thumbnails YouTube
  - Preview button plus visible (floating play button)
  - Waveform preview plus lisible
- [ ] **Auto-assignment preview** : Montrer visuellement où sera assigné le sound avant d'ajouter
- [ ] **"Discover Mode"** : Plein écran overlay pour browse mode immersif

**Fichiers concernés :**
- `src/components/Discovery/DiscoveryPanel.tsx`
- `src/stores/discoveryStore.ts`

---

## 9. HEADER - AJOUT D'INFORMATIONS

**Améliorations :**
- [ ] **Breadcrumb** : Current profile name visible
- [ ] **Quick actions** :
  - Global shortcuts status (Key Detection ON/OFF indicator)
  - Quick access to Add Sound (+ button)
  - Notification bell pour les background operations
- [ ] **Master volume plus visible** : Plus grand, avec waveform en background pendant la lecture

**Fichiers concernés :**
- `src/components/Layout/Header.tsx`

---

## 10. TRACKS - REDESIGN

**Améliorations :**
- [ ] **Cards verticales** au lieu d'horizontales (plus d'espace pour les noms)
- [ ] **Color-coding** : Chaque track a une couleur unique (utilisée dans KeyGrid aussi)
- [ ] **Visual feedback** : Pulsing border quand un son joue sur ce track
- [ ] **Track solo/mute** : Boutons dédiés
- [ ] **Meter audio** : VU meter simple pour chaque track

**Fichiers concernés :**
- `src/components/Tracks/TrackView.tsx`
- `src/stores/audioStore.ts`

---

## 11. SEARCHFILTERBAR - DISCOVERABILITY

**Améliorations :**
- [ ] **Onboarding tooltip** : Au premier usage, expliquer `t:`, `l:`, `s:`
- [ ] **Autocomplete dropdown** : Suggestions en temps réel
- [ ] **Recent searches** : Historique dans un dropdown
- [ ] **Visual query builder** : Alternative aux prefix pour les débutants (dropdowns Track / Loop / Status)
- [ ] **Clear indicator** : Plus visible que juste compteur, "X to clear"

**Fichiers concernés :**
- `src/components/common/SearchFilterBar.tsx`

---

## 12. PROFILES - MEILLEURE GESTION

**Améliorations :**
- [ ] **Profile cards** au lieu de liste :
  - Thumbnail (première waveform ou custom image)
  - Sound count, track count
  - Last modified date
- [ ] **Actions visibles** : Pas de hover-only, toujours visible mais subtle
- [ ] **Quick switch** : Ctrl+1-9 pour switch rapidement
- [ ] **Import/Export** : Boutons dédiés dans la section profiles, pas cachés dans Settings

**Fichiers concernés :**
- `src/components/Profiles/ProfileSelector.tsx`
- `src/stores/profileStore.ts`

---

## 13. ERREURS - TRAITEMENT PROACTIF

**Améliorations :**
- [ ] **Error prevention** :
  - Validation avant save/export
  - Warning si actions destructives
  - Confirmation intelligente (demander seulement si vraiment nécessaire)
- [ ] **FileNotFoundModal redesign** :
  - Montrer thumbnail/waveform du son manquant
  - "Fix all" option pour batch operations
  - Récent files quick select
- [ ] **Retry logic** : Auto-retry avec exponential backoff pour YouTube operations
- [ ] **Error toast persistant** : Reste jusqu'à action user pour les erreurs critiques

**Fichiers concernés :**
- `src/components/Errors/FileNotFoundModal.tsx`
- `src/stores/errorStore.ts`
- `src/utils/errorMessages.ts`

---

## 14. ACCESSIBILITY - NIVEAU PROFESSIONNEL

**Améliorations :**
- [ ] **Focus trap** dans toutes les modales
- [ ] **ARIA landmarks** : Toutes les regions doivent avoir role
- [ ] **Screen reader announcements** : aria-live pour toasts, progress updates
- [ ] **Keyboard navigation complète** :
  - Tab order logique
  - Skip links
  - Toutes les actions accessibles au clavier
- [ ] **High contrast mode** : Mode alternatif avec borders plus marqués
- [ ] **Tooltips explicites** : Tous les icon-only buttons doivent avoir tooltip
- [ ] **Focus indicators** : Ring visible sur tous les éléments focusables

**Fichiers concernés :**
- Tous les composants UI
- `src/index.css` (styles focus globaux)

---

## 15. PERSONNALITÉ & POLISH

**Améliorations subtiles pour ajouter du caractère :**
- [ ] **Transitions spring** : Utiliser spring physics au lieu de ease-in-out
- [ ] **Sound effects optionnels** : Click sounds, success ding (toggle dans settings)
- [ ] **Thèmes de couleur** : Quelques variants (Indigo par défaut, Rose, Teal, Amber)
- [ ] **Easter eggs** : Konami code pour unlock hidden feature ou animation
- [ ] **Loading messages fun** : Au lieu de "Loading...", utiliser des messages variés
- [ ] **Empty state illustrations** : Mini illustrations custom au lieu de juste icônes
- [ ] **Momentum sparkles** : Animation de particules quand suggestion détectée
- [ ] **Playback visualizer** : Subtle wave animation dans le header pendant lecture

**Fichiers concernés :**
- `src/index.css`
- `tailwind.config.js`
- `src/stores/settingsStore.ts` (pour theme switcher)

---

## 16. RESPONSIVE & ADAPTABILITÉ

**Améliorations :**
- [ ] **Minimum window size** : Réduire de 800x600 à 1024x768 pour meilleure lisibilité
- [ ] **Zoom interface** : Ctrl+=/- pour zoom UI (persist in settings)
- [ ] **Resizable panels** :
  - Sidebar redimensionnable (actuellement fixe)
  - SoundDetails handle plus visible (6px au lieu de 1.5px)
  - Visual feedback pendant resize
- [ ] **Compact mode** : Toggle pour utilisateurs avancés (density control)

**Fichiers concernés :**
- `src/components/Layout/MainContent.tsx`
- `src/components/Layout/Sidebar.tsx`
- `src-tauri/tauri.conf.json` (window minSize)

---

## 17. ONBOARDING & HELP

**Nouveau :**
- [ ] **First-run wizard** :
  1. Bienvenue
  2. Créer premier profile
  3. Ajouter premier son
  4. Assigner première clé
  5. Tester playback
- [ ] **Contextual hints** : Tooltips auto-show la première fois
- [ ] **"What's new" modal** : Après update, montrer les nouvelles features
- [ ] **Interactive tutorial** : Mode "Tutorial" qui guide à travers les features
- [ ] **Tips system** : Random tips dans les empty states ou au startup

**Nouveaux fichiers à créer :**
- `src/components/Onboarding/FirstRunWizard.tsx`
- `src/components/Onboarding/WhatsNewModal.tsx`
- `src/stores/onboardingStore.ts`

---

## 18. PERFORMANCE PERÇUE

**Améliorations :**
- [ ] **Optimistic updates** : Montrer le changement immédiatement, rollback si erreur
- [ ] **Skeleton screens** : Partout où il y a loading
- [ ] **Progressive enhancement** : Montrer les données dès qu'elles arrivent (pas attendre tout)
- [ ] **Background operations** : Toutes les ops lourdes doivent avoir une notification non-blocking
- [ ] **Instant feedback** : Toute interaction doit avoir feedback <100ms

**Fichiers concernés :**
- Tous les stores Zustand
- `src/hooks/useAudioEvents.ts`
- `src/utils/tauriCommands.ts`

---

## 🎯 PRIORITÉS D'IMPLÉMENTATION

### Phase 1 - Quick Wins (Impact élevé, effort faible)
**Estimation : 1-2 semaines**

1. ✅ Augmenter les espacements généraux
2. ✅ Améliorer KeyGrid (grille fixe, hover states)
3. ✅ Toasts avec durées variables et action buttons
4. ✅ Confirm dialog avec variants et keyboard
5. ✅ Waveform click-to-jump

**Fichiers principaux :**
- `src/components/Keys/KeyGrid.tsx`
- `src/stores/toastStore.ts`
- `src/components/Toast/ToastContainer.tsx`
- `src/components/ConfirmDialog.tsx`
- `src/components/common/WaveformDisplay.tsx`
- `src/index.css`

---

### Phase 2 - Fondations (Impact élevé, effort moyen)
**Estimation : 2-3 semaines**

1. ✅ Sidebar avec tabs
2. ✅ SoundDetails simplification (tabs + sections collapsibles)
3. ✅ Tracks redesign avec color-coding
4. ✅ Profiles cards design
5. ✅ Accessibility basics (ARIA, focus trap)

**Fichiers principaux :**
- `src/components/Layout/Sidebar.tsx`
- `src/components/Sounds/SoundDetails.tsx`
- `src/components/Tracks/TrackView.tsx`
- `src/components/Profiles/ProfileSelector.tsx`
- Tous les composants (ARIA)

---

### Phase 3 - Polish (Impact moyen, effort moyen)
**Estimation : 2-3 semaines**

1. ✅ AddSoundModal wizard
2. ✅ SettingsModal navigation latérale
3. ✅ Discovery panel full redesign
4. ✅ Animations & micro-interactions
5. ✅ Onboarding wizard

**Fichiers principaux :**
- `src/components/Sounds/AddSoundModal.tsx`
- `src/components/Settings/SettingsModal.tsx`
- `src/components/Discovery/DiscoveryPanel.tsx`
- `src/index.css` (animations)
- Nouveaux composants Onboarding

---

### Phase 4 - Avancé (Impact moyen, effort élevé)
**Estimation : 3-4 semaines**

1. ✅ Waveform zoom & minimap
2. ✅ Themes de couleur
3. ✅ Compact mode
4. ✅ Interactive tutorial
5. ✅ Sound effects system

**Fichiers principaux :**
- `src/components/common/WaveformDisplay.tsx`
- `tailwind.config.js`
- `src/stores/settingsStore.ts`
- Système de sons UI

---

## 📊 MÉTRIQUES DE SUCCÈS

**Objectifs mesurables :**
- [ ] Réduction du temps de découverte des features (onboarding tracking)
- [ ] Augmentation de la satisfaction utilisateur (feedback form)
- [ ] Réduction des erreurs utilisateur (analytics)
- [ ] Score d'accessibilité 90+ (Lighthouse/axe)
- [ ] Temps de réponse UI < 100ms (toutes interactions)

---

## 🚀 NOTES D'IMPLÉMENTATION

**Design System :**
- Créer un fichier `src/styles/tokens.ts` avec tous les design tokens
- Documenter tous les composants avec Storybook ou similaire
- Créer un guide de style pour maintenir la cohérence

**Testing :**
- Tests visuels (Chromatic ou Percy)
- Tests d'accessibilité (jest-axe)
- Tests E2E pour les flows critiques (Playwright)

**Documentation :**
- Mettre à jour CLAUDE.md avec les nouveaux patterns
- Créer un DESIGN.md pour la documentation UI
- Vidéos tutoriels pour les features complexes

---

**Total estimé : 8-12 semaines pour tout implémenter**
**Préserve 100% des fonctionnalités existantes**
