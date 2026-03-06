# 🎨 KeyToMusic 2.0 - Refonte Complète de l'UI

**Status:** Proposition en attente de validation

**Philosophie:** "Tout doit avoir sa place naturelle, rien ne doit se battre pour l'attention."

---

## 📋 RÉSUMÉ EXÉCUTIF

### Pourquoi une refonte complète ?

L'UI actuelle a évolué organiquement en ajoutant feature après feature. Résultat : tout est trop serré, dense, manque de respiration. Maintenant qu'on a la vision complète des fonctionnalités, c'est le moment de **repenser l'architecture de l'information de zéro**.

### Vision 2026

**Mots-clés :** Spacieux, Clair, Moderne, Fluide, Contextuel, Respiration

**Inspirations :**
- **Linear** (minimalisme, respiration, typography)
- **Figma** (panels, outils contextuels, inspector)
- **Arc Browser** (sidebar verticale, spaces, élégance)
- **Spotify redesign** (cards, immersion)

### Changements majeurs

1. **Navigation Rail** (64px) au lieu de Sidebar (224px)
2. **Inspector Panel** contextuel à droite (collapsible)
3. **4 sections principales** : Keys, Discovery, Library, Settings
4. **Now Playing Bar** global en bottom
5. **Spacing généreux** partout (échelle 8px)
6. **Track color-coding** pour clarté visuelle

---

## 📐 ARCHITECTURE GLOBALE

### Layout Principal

```
┌────────────────────────────────────────────────────────────────────────────┐
│  ◉ KeyToMusic    【 My Manga Profile 】   ♫ Master 75%    🔍  ⚙️  ?        │
├─────┬──────────────────────────────────────────────────────────┬────────────┤
│     │                                                           │            │
│  🎹 │                                                           │            │
│ ──  │                                                           │   Detail   │
│  ⭐ │              MAIN WORKSPACE                               │            │
│ ──  │                                                           │   Panel    │
│  📚 │          (Contextuel selon section)                       │            │
│ ──  │                                                           │ (Inspector)│
│  ⚙️ │                                                           │            │
│     │                                                           │            │
├─────┴──────────────────────────────────────────────────────────┴────────────┤
│  ▶ OST: Demon Slayer OP [══════▶─────────] 1:23 / 3:45        ⏹  Vol 80%  │
│  ▶ Ambiance: Rain    [════▶───────────────] 0:45 / 5:12       ⏹  Vol 60%  │
└────────────────────────────────────────────────────────────────────────────┘
```

### Zones Principales

#### 1. Header Bar (fixe, 56px)
- Logo + App name (gauche)
- Profile switcher - Dropdown moderne avec preview
- Master volume - Compact mais visible
- Search global - Spotlight-style
- Quick actions - Settings, Help icons

#### 2. Nav Rail (gauche, 64px, icônes only)
```
│  🎹  │  Keys (vue principale)
│ ──── │
│  ⭐  │  Discovery
│ ──── │
│  📚  │  Library
│ ──── │
│  ⚙️  │  Settings
```
- Icônes avec tooltips
- Active state clair (border accent gauche)
- Hover state subtil

#### 3. Main Workspace (centre, flexible)
- Change selon la section active
- Spacieux, aéré
- Padding généreux (32px)

#### 4. Detail Panel (droite, 360px, resizable, collapsible)
- Inspector contextuel
- Affiche les détails du sélectionné
- Peut se cacher (toggle icon, Cmd+D)
- Resize handle visible (6px)

#### 5. Now Playing Bar (bottom, height auto)
- Toujours visible
- Mini-player pour chaque track en lecture
- Collapsible si rien ne joue (devient 1px de séparation)
- Max 3 tracks visibles, scroll horizontal si plus

---

## 🎹 SECTION 1: KEYS (Vue Principale)

**C'est la vue par défaut, le cœur de l'app.**

```
┌────────────────────────────────────────────────────────────────────────────┐
│  🎹 Keys                                                                    │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─ TRACKS ─────────────────────────────────────────────────────┐         │
│  │                                                                │         │
│  │  ┏━━━━━━━━━━━┓  ┏━━━━━━━━━━━┓  ┏━━━━━━━━━━━┓  ┏━━━━━━━━━━┓ │         │
│  │  ┃   OST     ┃  ┃ Ambiance  ┃  ┃    SFX    ┃  ┃ Voice    ┃ │   +     │
│  │  ┃  🔊 80%   ┃  ┃  🔊 60%   ┃  ┃  🔊 90%   ┃  ┃ 🔊 70%   ┃ │  Track  │
│  │  ┃ 🔵 Playing┃  ┃           ┃  ┃           ┃  ┃          ┃ │         │
│  │  ┗━━━━━━━━━━━┛  ┗━━━━━━━━━━━┛  ┗━━━━━━━━━━━┛  ┗━━━━━━━━━━┛ │         │
│  │                                                                │         │
│  └────────────────────────────────────────────────────────────────         │
│                                                                             │
│  ┌─ KEY BINDINGS ───────────────────────────────┐                         │
│  │                                               │                         │
│  │  🔍 Search keys...          Track: All ▼      │  🎵 Add Sound           │
│  │                                               │                         │
│  └───────────────────────────────────────────────┘                         │
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────┐         │
│  │                                                                │         │
│  │   ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐     │         │
│  │   │  A  │  │  Z  │  │  E  │  │  R  │  │  T  │  │  Y  │     │         │
│  │   │ 🎵  │  │ 🎵🎵│  │     │  │ 🎵  │  │ 🎵  │  │     │     │         │
│  │   │ OST │  │Multi│  │Empty│  │ SFX │  │ AMB │  │Empty│     │         │
│  │   └─────┘  └─────┘  └─────┘  └─────┘  └─────┘  └─────┘     │         │
│  │                                                                │         │
│  │   ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐     │         │
│  │   │  Q  │  │  S  │  │  D  │  │  F  │  │  G  │  │  H  │     │         │
│  │   │ 🎵  │  │ 🎵  │  │ 🎵  │  │ 🎵  │  │     │  │     │     │         │
│  │   │Voice│  │ OST │  │ OST │  │ SFX │  │Empty│  │Empty│     │         │
│  │   └─────┘  └─────┘  └─────┘  └─────┘  └─────┘  └─────┘     │         │
│  │                                                                │         │
│  │   ... (grille continue, spacieuse)                            │         │
│  │                                                                │         │
│  └────────────────────────────────────────────────────────────────         │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘

┌─ INSPECTOR (RIGHT PANEL) ────────────────┐
│                                           │
│  Key: Z                                   │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                           │
│  🎵 Demon Slayer OP        Track: OST    │
│     ┌────────────────────────────┐       │
│     │    [Waveform Display]      │       │
│     │  ▁▂▃▅▇█▇▅▃▂▁▂▃▅▇█▇▅▃▂▁    │       │
│     │         ↑ Momentum         │       │
│     └────────────────────────────┘       │
│     🔊 Volume: ████████░░ 80%            │
│     🔁 Loop: Sequential ▼                │
│     ⚡ Momentum: 2.3s                    │
│                                           │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                           │
│  🎵 Attack on Titan OP     Track: OST    │
│     ┌────────────────────────────┐       │
│     │    [Waveform Display]      │       │
│     │  ▁▃▅▇█▅▃▁▂▄▆█▆▄▂▁▃▅▇█▅    │       │
│     └────────────────────────────┘       │
│     🔊 Volume: ██████████ 100%           │
│     🔁 Loop: Random ▼                    │
│     ⚡ Momentum: 0.0s                    │
│                                           │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                           │
│  ✏️ Reassign Key    🗑️ Remove Binding   │
│                                           │
└───────────────────────────────────────────┘
```

### Design Details - Keys Section

#### Tracks Bar
**Layout:**
- Cards horizontales, spacieuses (160px width, 100px height)
- Gap 16px entre cards
- Scroll horizontal si >6 tracks
- Bouton "+ Track" en fin de liste

**Card Design:**
- Couleur unique par track (border + subtle background gradient)
- Track name (editable inline)
- Volume slider (toujours visible, pas de hover)
- Indicateur de lecture:
  - Dot bleu pulsant (animé)
  - Ring autour du card
  - Waveform miniature animée en background
- Actions en hover:
  - Solo/Mute toggle
  - Delete (confirmation)
  - Settings (rename, color)

**Track Colors:**
- OST: `#6366F1` (Indigo)
- Ambiance: `#8B5CF6` (Purple)
- SFX: `#EC4899` (Pink)
- Voice: `#14B8A6` (Teal)
- Custom tracks: Palette étendue (8 couleurs supplémentaires)

#### Key Grid

**Layout:**
- CSS Grid: `grid-template-columns: repeat(auto-fill, minmax(80px, 1fr))`
- Gap: 12px (respiration)
- Cards: 80x80px (aspect-square)
- Padding container: 24px

**Card Design:**
```
┌─────────┐
│   KEY   │  ← Keycode (14px, semibold, mono)
│   🎵    │  ← Sound icon
│  Track  │  ← Track name (truncated, 11px)
└─────────┘
```

**États visuels:**
1. **Empty:**
   - Dashed border `#2A2A2A`
   - Background `transparent`
   - Subtle text "Empty" (opacity 0.3)
   - Hover: Solid border + "Click to assign"

2. **Assigned (1 sound):**
   - Solid border with track color (`2px`)
   - Background `#161616`
   - Track color accent (subtil gradient)
   - Sound icon: 🎵
   - Track name visible

3. **Multi-sounds:**
   - Badge "×N" en top-right corner
   - Multiple track color indicators (dots en bottom)
   - Icon: 🎵🎵

4. **Playing:**
   - Pulsing ring (track color, 3px)
   - Subtle glow `shadow-[0_0_20px_rgba(color,0.4)]`
   - Icon animé (scale pulse)

5. **Selected:**
   - Border accent épaisse (4px, `#6366F1`)
   - Background slightly elevated
   - Inspector panel shows details

6. **Filtered (search):**
   - Opacity: 0.3
   - Blur: 1px
   - Pointer-events: none

**Interactions:**
- Click: Select (show in inspector)
- Double-click: Play preview
- Ctrl+Click: Multi-select
- Right-click: Context menu (Reassign, Remove, Copy, etc.)
- Drag: Reassign to another key (drag & drop)

#### Search/Filter Bar

**Layout:**
```
┌────────────────────────────────────────────────┐
│ 🔍 Search keys...          Track: All ▼        │  🎵 Add Sound
└────────────────────────────────────────────────┘
```

**Features:**
- Search input (focus: Ctrl+F)
- Track filter dropdown
- Result counter: "24 / 48 keys"
- "Add Sound" button (primary action)
- Clear button (X icon, appears when filtering)

#### Inspector Panel (Right)

**Header:**
```
Key: Z                    [Collapse icon]
```

**Sound Cards (vertical list):**
```
┌────────────────────────────────────┐
│ 🎵 Sound Name      Track: OST ▼   │
│ ────────────────────────────────── │
│ [Waveform - 280px width]           │
│ ▁▂▃▅▇█▇▅▃▂▁                       │
│         ↑ Momentum marker          │
│ ────────────────────────────────── │
│ 🔊 Volume  ████████░░ 80%          │
│ 🔁 Loop    Sequential ▼            │
│ ⚡ Momentum 2.3s  [✨ Apply AI]     │
│ ────────────────────────────────── │
│ ▶ Preview  |  🗑️ Remove            │
└────────────────────────────────────┘
```

**Features:**
- Collapsible sections si >2 sounds
- Waveform cliquable (set momentum)
- Inline editing (all fields)
- Apply AI suggestion (badge avec animation)
- Preview button (plays in preview track)
- Remove confirmation

**Footer Actions:**
```
┌────────────────────────────────────┐
│  ✏️ Reassign Key                   │
│  🗑️ Remove All Bindings            │
└────────────────────────────────────┘
```

---

## ⭐ SECTION 2: DISCOVERY

**Vue immersive, plein écran, focus sur la découverte.**

```
┌────────────────────────────────────────────────────────────────────────────┐
│  ⭐ Discovery                                          🔄 Refresh  ⚙️ Seeds │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   Based on your library: Demon Slayer, Attack on Titan, JJK...            │
│                                                                             │
│  ┌────────────────────────────────────────────────────────────────┐       │
│  │                                                                 │       │
│  │    ┌─────────────────┐  ┌─────────────────┐  ┌───────────────┐│       │
│  │    │                 │  │                 │  │               ││       │
│  │    │   [Thumbnail]   │  │   [Thumbnail]   │  │  [Thumbnail]  ││       │
│  │ ◄──│                 │──│                 │──│               │├─►     │
│  │    │  Bleach OP 13   │  │  Naruto OP 16   │  │  One Piece OP ││       │
│  │    │                 │  │                 │  │               ││       │
│  │    │  ▁▃▅▇█▅▃▁▂▄▆█   │  │  ▂▄▆█▆▄▂▁▃▅▇█   │  │ ▁▂▃▅▇█▇▅▃▂▁  ││       │
│  │    │                 │  │                 │  │               ││       │
│  │    │  ⚡ Auto 2.3s   │  │  ⚡ Auto 1.8s   │  │ ⚡ Auto 3.1s  ││       │
│  │    │  📊 3:45        │  │  📊 4:12        │  │ 📊 2:58       ││       │
│  │    │                 │  │                 │  │               ││       │
│  │    │  ▶ Preview      │  │  ▶ Preview      │  │ ▶ Preview     ││       │
│  │    │                 │  │                 │  │               ││       │
│  │    │  ✚ Add  👎      │  │  ✚ Add  👎      │  │ ✚ Add  👎     ││       │
│  │    └─────────────────┘  └─────────────────┘  └───────────────┘│       │
│  │                                                                 │       │
│  │                        1 / 30 suggestions                       │       │
│  │                                                                 │       │
│  └─────────────────────────────────────────────────────────────────        │
│                                                                             │
│  ╔═══════════════════════════════════════════════════════════════╗        │
│  ║  Quick Add Setup:                                             ║        │
│  ║  Key: [Press...] ⌨️   Track: OST ▼   Momentum: Auto ✨       ║        │
│  ╚═══════════════════════════════════════════════════════════════╝        │
│                                                                             │
│  ┌─ DISLIKED VIDEOS (Collapsed) ──────────────────────────────┐           │
│  │  5 videos disliked  [Expand ▼]                              │           │
│  └──────────────────────────────────────────────────────────────           │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

### Design Details - Discovery

#### Top Bar
```
⭐ Discovery         Based on: [Seeds preview]         🔄 Refresh  ⚙️ Configure
```

**Seeds preview:**
- Scrolling horizontal list de thumbnails
- Max 5 visible
- Click pour voir tous les seeds
- Configure ouvre modal de seed management

#### Carousel

**Layout:**
- 3 cards visibles simultanément (desktop)
- Card size: 300px width × 450px height
- Gap: 24px
- Navigation: Arrows (keyboard ←/→ aussi)
- Pagination dots en bas

**Card Design:**
```
┌─────────────────┐
│                 │
│   [Thumbnail]   │  ← YouTube thumbnail (16:9, 300x169px)
│    YouTube      │     ou gradient placeholder si pas dispo
│                 │
├─────────────────┤
│ Video Title     │  ← Truncated à 2 lignes, tooltip on hover
├─────────────────┤
│ ▁▃▅▇█▅▃▁▂▄▆█   │  ← Waveform preview (interactive)
│       ↑         │     Momentum marker visible
├─────────────────┤
│ ⚡ Auto 2.3s    │  ← Momentum suggestion (AI badge)
│ 📊 3:45         │  ← Duration
│ 🎤 Artist Name  │  ← Metadata si dispo
├─────────────────┤
│  ▶ Preview      │  ← Play/Stop button
│ ─────────────   │     Volume slider en hover
├─────────────────┤
│ [✚ Add] [👎]   │  ← Primary & Secondary actions
└─────────────────┘
```

**States:**
- Loading: Skeleton avec shimmer
- Predownloading: Progress bar subtle en bottom
- Playing preview: Button devient ⏸, waveform animée
- Already in library: Badge "In library" + disable Add

#### Quick Add Setup Bar

**Always visible en bas:**
```
╔═══════════════════════════════════════════════════════════════╗
║  Quick Add Setup:                                             ║
║  Key: [Press...] ⌨️   Track: OST ▼   Momentum: Auto ✨       ║
║                                                                ║
║  [Apply to All] Configure once, add multiple sounds quickly   ║
╚═══════════════════════════════════════════════════════════════╝
```

**Features:**
- Key capture inline (click to activate)
- Track selector (dropdown with colors)
- Momentum mode:
  - Auto (use AI suggestion) ← Default
  - Manual (input seconds)
  - None (start at 0)
- Settings persist across suggestions
- "Add" button sur chaque card utilise ces settings

#### Disliked Videos Panel

**Collapsed par défaut:**
```
┌─ DISLIKED VIDEOS ────────────────────────────┐
│  5 videos disliked  [Expand ▼]               │
└───────────────────────────────────────────────┘
```

**Expanded:**
```
┌─ DISLIKED VIDEOS ────────────────────────────┐
│                                               │
│  🎵 Video Title 1            [Undislike]     │
│  🎵 Video Title 2            [Undislike]     │
│  🎵 Video Title 3            [Undislike]     │
│                                               │
│  [Clear All]                                  │
└───────────────────────────────────────────────┘
```

**No Inspector Panel in Discovery** - Mode immersif, full focus sur carousel

---

## 📚 SECTION 3: LIBRARY

**Gestion et organisation de tous les éléments.**

```
┌────────────────────────────────────────────────────────────────────────────┐
│  📚 Library                                                                 │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─ PROFILES ───────────────────────────────────────────────────┐         │
│  │                                                                │         │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────┐ │         │
│  │  │ 📁 Manga   │  │ 📁 Chill   │  │ 📁 Gaming  │  │  + New │ │         │
│  │  │ ──────────│  │ ──────────│  │ ──────────│  │        │ │         │
│  │  │ [Waveform] │  │ [Waveform] │  │ [Waveform] │  │        │ │         │
│  │  │ ──────────│  │ ──────────│  │ ──────────│  │        │ │         │
│  │  │ 127 sounds │  │  45 sounds │  │  89 sounds │  │        │ │         │
│  │  │ 4 tracks   │  │  3 tracks  │  │  5 tracks  │  │        │ │         │
│  │  │ Modified:  │  │ Modified:  │  │ Modified:  │  │        │ │         │
│  │  │ 2h ago     │  │ 1 day ago  │  │ 3 days ago │  │        │ │         │
│  │  │ ──────────│  │ ──────────│  │ ──────────│  │        │ │         │
│  │  │ ⭐ Active  │  │  Switch    │  │  Switch    │  │        │ │         │
│  │  │  ⋮ More   │  │  ⋮ More    │  │  ⋮ More    │  │        │ │         │
│  │  └────────────┘  └────────────┘  └────────────┘  └────────┘ │         │
│  │                                                                │         │
│  └────────────────────────────────────────────────────────────────         │
│                                                                             │
│  ┌─ ALL SOUNDS ────────────────────────────────────────────────┐          │
│  │                                                              │          │
│  │  🔍 Search sounds...     Sort: Name ▼   Filter: Track ▼     │          │
│  │                                                              │          │
│  │  127 sounds • 45 minutes total                              │          │
│  │                                                              │          │
│  └──────────────────────────────────────────────────────────────          │
│                                                                             │
│  ┌────────────────────────────────────────────────────────────┐           │
│  │                                                              │           │
│  │  ┌──────────────────────────────────────────────────────┐  │           │
│  │  │ 🎵 Demon Slayer OP                                   │  │           │
│  │  │    ▁▂▃▅▇█▇▅▃▂▁▂▃▅▇  3:45  Track: OST  Key: Z  ⚡2.3s│  │           │
│  │  │    🔊 80%   🔁 Sequential                            │  │           │
│  │  │    ▶ Preview  ✏️ Edit  🗑️ Delete                    │  │           │
│  │  └──────────────────────────────────────────────────────┘  │           │
│  │                                                              │           │
│  │  ┌──────────────────────────────────────────────────────┐  │           │
│  │  │ 🎵 Attack on Titan OP                                │  │           │
│  │  │    ▁▃▅▇█▅▃▁▂▄▆█  4:12  Track: OST  Key: A  ⚡0.0s   │  │           │
│  │  │    🔊 100%  🔁 Random                                │  │           │
│  │  │    ▶ Preview  ✏️ Edit  🗑️ Delete                    │  │           │
│  │  └──────────────────────────────────────────────────────┘  │           │
│  │                                                              │           │
│  │  ... (liste continue)                                       │           │
│  │                                                              │           │
│  └──────────────────────────────────────────────────────────────          │
│                                                                             │
│  ┌─ ACTIONS ──────────────────────────────────────────────────┐           │
│  │  💾 Import Profile   📤 Export Profile   🗑️ Cleanup Cache  │           │
│  └────────────────────────────────────────────────────────────────        │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘

┌─ INSPECTOR (RIGHT PANEL) ────────────────┐
│                                           │
│  Profile: Manga                           │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                           │
│  Created: Jan 15, 2026                   │
│  Modified: 2 hours ago                   │
│  Size: 2.3 GB                            │
│                                           │
│  ┌─ STATISTICS ─────────────────┐        │
│  │ Total Sounds: 127            │        │
│  │ Total Duration: 8h 23m       │        │
│  │ Tracks: 4                    │        │
│  │ Key Bindings: 89             │        │
│  │ Unassigned: 38               │        │
│  └──────────────────────────────┘        │
│                                           │
│  ┌─ TOP PLAYED ─────────────────┐        │
│  │ 1. Demon Slayer OP (342×)    │        │
│  │ 2. Attack on Titan OP (289×) │        │
│  │ 3. JJK Opening (203×)        │        │
│  └──────────────────────────────┘        │
│                                           │
│  ┌─ ACTIONS ────────────────────┐        │
│  │ 📤 Export Profile             │        │
│  │ 📋 Duplicate Profile          │        │
│  │ 🎨 Change Color Theme         │        │
│  │ 🗑️ Delete Profile             │        │
│  └──────────────────────────────┘        │
│                                           │
└───────────────────────────────────────────┘
```

### Design Details - Library

#### Profiles Section

**Card Design (180px × 240px):**
```
┌────────────┐
│  Icon 📁   │  ← Profile icon ou thumbnail custom
├────────────┤
│ [Waveform] │  ← Preview waveform du 1er sound
├────────────┤
│ Name       │  ← Editable inline
├────────────┤
│ 127 sounds │  ← Stats
│ 4 tracks   │
│ Modified:  │
│ 2h ago     │
├────────────┤
│ ⭐ Active  │  ← Status badge (si active)
│  Switch    │  ← Primary action
│  ⋮ More    │  ← Dropdown menu
└────────────┘
```

**More menu:**
- Rename
- Duplicate
- Export
- Change Icon
- Delete (confirmation)

**Active profile:**
- Badge "⭐ Active" en vert
- Border accent
- Slightly elevated

#### All Sounds Section

**Liste design (table-like mais stylée):**

**Header:**
```
🔍 Search...    Sort: [Name/Date/Duration/Track] ▼    Filter: [Track/Loop] ▼

127 sounds • 8h 23m total
```

**Sound Row:**
```
┌──────────────────────────────────────────────────────────┐
│ 🎵 Sound Name                                            │
│    ▁▂▃▅▇█▇▅▃▂▁  3:45  Track: OST  Key: Z  ⚡2.3s       │
│    🔊 80%  🔁 Sequential                                 │
│    ▶ Preview  ✏️ Edit  🗑️ Delete                        │
└──────────────────────────────────────────────────────────┘
```

**Features:**
- Click row to select (show in inspector)
- Inline actions (preview, edit, delete)
- Multi-select with Ctrl+Click
- Batch actions (delete, move track, export)
- Sortable by all fields
- Filterable by track, loop mode, assigned/unassigned

#### Actions Section

**Bottom bar:**
```
┌─ ACTIONS ──────────────────────────────────────────┐
│  💾 Import Profile                                  │
│  📤 Export Active Profile                          │
│  🗑️ Cleanup Cache                                  │
│  📊 View Analytics                                  │
└─────────────────────────────────────────────────────┘
```

#### Inspector Panel (Profile Details)

**Quand aucun son sélectionné, montre les stats du profile**

**Sections:**
1. **Metadata** (created, modified, size)
2. **Statistics** (counts, totals)
3. **Top Played** (most used sounds)
4. **Actions** (export, duplicate, delete)

**Quand son sélectionné, montre les détails du son** (même layout que Keys section)

---

## ⚙️ SECTION 4: SETTINGS

**Organisation claire par catégories avec navigation latérale.**

```
┌────────────────────────────────────────────────────────────────────────────┐
│  ⚙️ Settings                                                                │
├─────────────────┬──────────────────────────────────────────────────────────┤
│                 │                                                           │
│  🎵 Audio       │  ╔═══ AUDIO SETTINGS ═══════════════════════════════╗   │
│  ⌨️  Keyboard   │  ║                                                   ║   │
│  🎨 Appearance  │  ║  Output Device:                                   ║   │
│  📊 Performance │  ║  ┌────────────────────────────────────────────┐  ║   │
│  🔌 Advanced    │  ║  │ Realtek High Definition Audio          ▼  │  ║   │
│  ℹ️  About      │  ║  └────────────────────────────────────────────┘  ║   │
│                 │  ║  [🔄 Refresh Devices]                            ║   │
│                 │  ║                                                   ║   │
│                 │  ║  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ ║   │
│                 │  ║                                                   ║   │
│                 │  ║  Crossfade Duration: 500ms                       ║   │
│                 │  ║  ▰▰▰▰▰▰▰▰▰▰▱▱▱▱▱▱▱▱▱▱                            ║   │
│                 │  ║  100ms ←─────────────────────→ 2000ms            ║   │
│                 │  ║                                                   ║   │
│                 │  ║  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ ║   │
│                 │  ║                                                   ║   │
│                 │  ║  Master Volume: 75%                              ║   │
│                 │  ║  ▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▱▱▱▱▱                            ║   │
│                 │  ║                                                   ║   │
│                 │  ║  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ ║   │
│                 │  ║                                                   ║   │
│                 │  ║  ☐ Enable Audio Enhancements                     ║   │
│                 │  ║  ☐ Normalize Volume Across Tracks                ║   │
│                 │  ║                                                   ║   │
│                 │  ╚═══════════════════════════════════════════════════╝   │
│                 │                                                           │
│                 │  [Reset to Defaults]                                     │
│                 │                                                           │
└─────────────────┴──────────────────────────────────────────────────────────┘
```

### Design Details - Settings

#### Navigation Sidebar (180px)

**Categories:**
```
🎵 Audio           ← Active state (border left + bg highlight)
⌨️  Keyboard
🎨 Appearance
📊 Performance
🔌 Advanced
ℹ️  About
```

**Active state:**
- Left border accent (4px)
- Background slightly elevated
- Text color primary

#### Content Panels

**Structure commune:**
```
╔═══ CATEGORY NAME ═══════════════════════════╗
║                                              ║
║  Setting Label:                             ║
║  [Control]                                   ║
║  Help text explaining the setting           ║
║                                              ║
║  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ ║  ← Separator
║                                              ║
║  Next Setting...                            ║
║                                              ║
╚══════════════════════════════════════════════╝

[Reset to Defaults]  ← Action button
```

#### 🎵 Audio Settings

**Sections:**
1. **Output Device**
   - Dropdown list des devices
   - Refresh button
   - Auto-detect default device

2. **Playback**
   - Crossfade Duration slider (100-2000ms)
   - Master Volume slider
   - Sample Rate (dropdown)
   - Buffer Size (dropdown)

3. **Enhancements**
   - Audio enhancements toggle
   - Volume normalization toggle

#### ⌨️ Keyboard Settings

**Sections:**
1. **Global Shortcuts**
   ```
   Stop All Sounds:     [Ctrl + Shift + S]  [Change]
   Toggle Key Detection: [Ctrl + K]          [Change]
   Toggle Auto-Momentum: [Ctrl + M]          [Change]
   ```
   - Key capture on click
   - Conflict warnings inline
   - Reset to defaults

2. **Key Detection**
   - Enable/Disable toggle
   - Cooldown slider (0-5000ms)
   - Chord Window slider (20-100ms)
   - Test button (shows key presses live)

3. **Momentum Modifier**
   - Dropdown: Shift / Ctrl / Alt / None
   - Conflict warnings avec track colors
   - Explain text

#### 🎨 Appearance Settings

**Sections:**
1. **Theme**
   - Color scheme selector:
     - Indigo (default)
     - Rose
     - Teal
     - Amber
     - Custom (color picker)
   - Preview cards showing theme

2. **Density**
   - Comfortable (default)
   - Compact
   - Spacious
   - Preview comparison

3. **Font Size**
   - Slider: Small / Medium / Large
   - Live preview

4. **Animations**
   - Enable/Disable toggle
   - Reduce motion (accessibility)

#### 📊 Performance Settings

**Sections:**
1. **Waveform Cache**
   - Cache size (MB)
   - Clear cache button
   - Auto-cleanup toggle

2. **Preloading**
   - Preload sounds on profile load
   - Preload discovery suggestions
   - Max concurrent downloads

3. **Memory Usage**
   - Current usage display
   - Max cache entries
   - Garbage collection settings

#### 🔌 Advanced Settings

**Sections:**
1. **Data Management**
   - Import Profile
   - Export Profile
   - Import Legacy (Unity format)
   - Cleanup unused files

2. **Developer**
   - Open Data Folder
   - Open Logs Folder
   - Enable Debug Mode
   - Export Debug Info

3. **Experimental**
   - Beta features toggles
   - Warning messages

#### ℹ️ About

**Content:**
```
╔═══ ABOUT KEYTOMUSIC ═══════════════════════╗
║                                              ║
║  KeyToMusic                                 ║
║  Version 2.0.0                              ║
║  Build 2026.02.02                           ║
║                                              ║
║  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ ║
║                                              ║
║  A desktop soundboard for manga reading     ║
║  with global keyboard detection.            ║
║                                              ║
║  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ ║
║                                              ║
║  🌐 Website                                 ║
║  📖 Documentation                           ║
║  🐛 Report Bug                              ║
║  💡 Request Feature                         ║
║  ⭐ Star on GitHub                          ║
║                                              ║
║  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ ║
║                                              ║
║  License: MIT                               ║
║  © 2026 KeyToMusic Team                     ║
║                                              ║
╚══════════════════════════════════════════════╝

[Check for Updates]
```

**No Inspector Panel in Settings** - Content area is sufficient

---

## 🎯 NOW PLAYING BAR (Global, Bottom)

**Toujours visible, collapsible, responsive.**

```
┌────────────────────────────────────────────────────────────────────────────┐
│  ▶ OST       Demon Slayer OP    [═════════▶──────────] 1:23 / 3:45  ⏹ 80% │
│  ▶ Ambiance  Rainy Night        [════▶────────────────] 0:45 / 8:12  ⏹ 60% │
└────────────────────────────────────────────────────────────────────────────┘
```

### Design Details

**Layout (per track):**
```
▶ [Track]  [Sound Name]  [Progress Bar]  [Time]  ⏹  [Volume]
```

**Components:**
1. **Play/Pause icon** (⏵/⏸)
   - Track color accent
   - Click to pause (pas de pause actuellement, mais pour futur)

2. **Track name** (60px width, fixed)
   - Track color background subtle
   - Truncated

3. **Sound name** (flexible, min 120px)
   - Truncated avec tooltip on hover
   - Click to focus dans KeyGrid

4. **Progress bar** (flexible, min 200px)
   - Track color fill
   - Clickable pour seek
   - Smooth animation

5. **Time display** (80px, monospace)
   - `current / total`
   - Format: `m:ss` ou `h:mm:ss` si >1h

6. **Stop button** (⏹)
   - Hover state obvious
   - Confirmation si multiple tracks

7. **Volume slider** (60px)
   - Compact, only shows on hover
   - Track-specific

**States:**
- **Nothing playing:** Bar collapses to 1px separator
- **1-3 tracks:** Stacked vertically (auto-height)
- **4+ tracks:** Scroll horizontal (max 3 visible)

**Interactions:**
- Click progress bar → Seek
- Hover → Show tooltips
- Drag volume slider → Adjust track volume

**Collapsible:**
- Icon en left: `▼` (collapse) / `▲` (expand)
- Collapsed: Just 1px line
- Keyboard: `Ctrl+P` toggle

---

## 🎨 DESIGN SYSTEM 2.0

### Couleurs

#### Base (Dark Mode)

**Backgrounds:**
```css
--bg-app: #0A0A0A           /* Main app background (plus foncé) */
--bg-surface: #161616       /* Cards, panels */
--bg-elevated: #1E1E1E      /* Elevated elements (hover, modal) */
--bg-input: #242424         /* Input fields */
```

**Borders:**
```css
--border-subtle: #2A2A2A    /* Default borders */
--border-medium: #3A3A3A    /* Hover, focus */
--border-strong: #4A4A4A    /* Active, selected */
```

**Text:**
```css
--text-primary: #FFFFFF     /* Main content */
--text-secondary: #B0B0B0   /* Secondary labels */
--text-tertiary: #707070    /* Hints, placeholders */
--text-disabled: #505050    /* Disabled state */
```

#### Accent Colors

**Primary:**
```css
--accent-primary: #6366F1       /* Indigo - Main brand */
--accent-primary-hover: #7C3AED /* Hover state */
--accent-primary-subtle: #6366F120 /* Background subtle */
```

**Semantic:**
```css
--color-success: #10B981    /* Emerald - Success, playing */
--color-warning: #F59E0B    /* Amber - Warnings */
--color-error: #EF4444      /* Red - Errors, destructive */
--color-info: #3B82F6       /* Blue - Info */
```

#### Track Colors

**Predefined:**
```css
--track-ost: #6366F1        /* Indigo */
--track-ambiance: #8B5CF6   /* Purple */
--track-sfx: #EC4899        /* Pink */
--track-voice: #14B8A6      /* Teal */
--track-custom-1: #F59E0B   /* Amber */
--track-custom-2: #EF4444   /* Red */
--track-custom-3: #10B981   /* Emerald */
--track-custom-4: #3B82F6   /* Blue */
```

**Usage:**
- Border: Full opacity
- Background: 10% opacity (`--track-ost + 10`)
- Glow: 40% opacity

#### Special

**Momentum:**
```css
--momentum-ai: #06B6D4      /* Cyan - AI suggestions */
--momentum-manual: #F59E0B  /* Amber - Manual set */
```

---

### Typography

#### Font Stack

**Sans-serif:**
```css
font-family: 'Inter Variable', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
```

**Monospace:**
```css
font-family: 'JetBrains Mono', 'Fira Code', 'Courier New', monospace;
```

#### Type Scale

**Headers:**
```css
--text-display: 24px / 32px / 600   /* Section titles */
--text-heading: 18px / 24px / 500   /* Subsections */
--text-subheading: 16px / 22px / 500 /* Card headers */
```

**Body:**
```css
--text-body: 14px / 20px / 400      /* Main content */
--text-caption: 12px / 16px / 400   /* Labels, hints */
--text-overline: 11px / 16px / 600  /* Uppercase labels */
```

**Mono:**
```css
--text-mono: 13px / 18px / 400      /* Key codes, timestamps */
```

#### Font Weights
```css
--weight-regular: 400
--weight-medium: 500
--weight-semibold: 600
--weight-bold: 700
```

---

### Spacing System

**Base: 4px**

```css
--space-1: 4px      /* xs - Minimal gaps */
--space-2: 8px      /* sm - Inline elements */
--space-3: 12px     /* md - Between components */
--space-4: 16px     /* lg - Section padding */
--space-5: 20px     /* xl - Cards */
--space-6: 24px     /* 2xl - Major sections */
--space-8: 32px     /* 3xl - Page padding */
--space-12: 48px    /* 4xl - Large breakpoints */
```

**Usage:**
- Component padding: `--space-4` (16px)
- Grid gaps: `--space-3` (12px)
- Section spacing: `--space-6` (24px)
- Page margins: `--space-8` (32px)

---

### Border Radius

```css
--radius-sm: 6px    /* Small elements (badges, chips) */
--radius-md: 8px    /* Buttons, inputs */
--radius-lg: 12px   /* Cards, panels */
--radius-xl: 16px   /* Modals, large containers */
--radius-full: 9999px /* Pills, circles */
```

---

### Shadows

**Elevation system:**

```css
--shadow-sm: 0 1px 2px rgba(0,0,0,0.25)
--shadow-md: 0 4px 8px rgba(0,0,0,0.3)
--shadow-lg: 0 8px 16px rgba(0,0,0,0.35)
--shadow-xl: 0 16px 32px rgba(0,0,0,0.4)
```

**Glows (colored):**
```css
--glow-primary: 0 0 20px rgba(99,102,241,0.4)
--glow-success: 0 0 20px rgba(16,185,129,0.4)
--glow-error: 0 0 20px rgba(239,68,68,0.4)
```

---

### Components Styles

#### Buttons

**Primary:**
```css
bg: var(--accent-primary)
color: white
padding: 8px 16px
border-radius: var(--radius-md)
font-weight: 500
transition: all 200ms cubic-bezier(0.4, 0, 0.2, 1)

hover:
  bg: var(--accent-primary-hover)
  transform: translateY(-1px)
  shadow: var(--shadow-md)

active:
  transform: scale(0.98)
```

**Secondary:**
```css
bg: transparent
color: var(--text-secondary)
border: 1px solid var(--border-medium)
padding: 8px 16px
border-radius: var(--radius-md)

hover:
  color: var(--text-primary)
  border-color: var(--border-strong)
  bg: var(--bg-elevated)
```

**Ghost:**
```css
bg: transparent
color: var(--text-secondary)
padding: 8px 16px

hover:
  color: var(--text-primary)
  bg: var(--bg-elevated)
```

**Destructive:**
```css
bg: var(--color-error)
color: white
/* Same structure as primary */
```

#### Cards

**Base:**
```css
bg: var(--bg-surface)
border: 1px solid var(--border-subtle)
border-radius: var(--radius-lg)
padding: var(--space-4)
transition: all 200ms ease

hover:
  border-color: var(--border-medium)
  transform: translateY(-2px)
  shadow: var(--shadow-md)
```

**Elevated:**
```css
bg: var(--bg-elevated)
shadow: var(--shadow-sm)
/* No border */
```

#### Inputs

**Text Input:**
```css
bg: var(--bg-input)
border: 1px solid var(--border-subtle)
border-radius: var(--radius-md)
padding: 8px 12px
color: var(--text-primary)
font-size: 14px

placeholder:
  color: var(--text-tertiary)

focus:
  border-color: var(--accent-primary)
  outline: none
  box-shadow: 0 0 0 3px var(--accent-primary-subtle)

disabled:
  bg: var(--bg-surface)
  color: var(--text-disabled)
  cursor: not-allowed
```

**Range Slider:**
```css
/* Track */
bg: var(--bg-input)
height: 4px
border-radius: 9999px

/* Fill */
bg: var(--accent-primary)

/* Thumb */
width: 16px
height: 16px
bg: white
border-radius: 50%
box-shadow: var(--shadow-sm)

hover (thumb):
  transform: scale(1.2)
  box-shadow: var(--shadow-md)
```

**Select Dropdown:**
```css
/* Same as text input */
/* + Icon arrow on right */
```

---

### Animations

#### Timing Functions

```css
--ease-out-expo: cubic-bezier(0.16, 1, 0.3, 1)      /* Smooth deceleration */
--ease-in-out-circ: cubic-bezier(0.85, 0, 0.15, 1)  /* Circular ease */
--spring: cubic-bezier(0.4, 0, 0.2, 1)              /* Spring-like */
```

#### Durations

```css
--duration-instant: 100ms    /* Hover states */
--duration-fast: 200ms       /* Transitions */
--duration-normal: 300ms     /* Modals, panels */
--duration-slow: 500ms       /* Page transitions */
```

#### Common Animations

**Fade In:**
```css
@keyframes fadeIn {
  from { opacity: 0; transform: translateY(8px); }
  to { opacity: 1; transform: translateY(0); }
}
animation: fadeIn var(--duration-fast) var(--ease-out-expo);
```

**Scale In:**
```css
@keyframes scaleIn {
  from { opacity: 0; transform: scale(0.95); }
  to { opacity: 1; transform: scale(1); }
}
animation: scaleIn var(--duration-normal) var(--spring);
```

**Slide In (from right):**
```css
@keyframes slideInRight {
  from { transform: translateX(100%); }
  to { transform: translateX(0); }
}
animation: slideInRight var(--duration-normal) var(--ease-out-expo);
```

**Pulse (playing indicator):**
```css
@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.6; transform: scale(1.05); }
}
animation: pulse 2s var(--ease-in-out-circ) infinite;
```

**Shimmer (loading):**
```css
@keyframes shimmer {
  0% { background-position: -200% 0; }
  100% { background-position: 200% 0; }
}
background: linear-gradient(90deg,
  transparent 0%,
  rgba(255,255,255,0.05) 50%,
  transparent 100%
);
background-size: 200% 100%;
animation: shimmer 1.5s infinite;
```

---

### Micro-Interactions

**Button Press:**
```css
active {
  transform: scale(0.98);
  transition-duration: 50ms;
}
```

**Card Hover:**
```css
hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-md);
  border-color: var(--border-medium);
}
```

**Playing Track Glow:**
```css
/* Pulsing ring */
@keyframes playingGlow {
  0%, 100% { box-shadow: 0 0 0 2px var(--track-color); }
  50% { box-shadow: 0 0 0 4px var(--track-color), var(--glow-primary); }
}
```

**Waveform Streaming:**
```css
/* Reveal left to right */
clip-path: inset(0 ${100 - progress}% 0 0);
transition: clip-path 0.1s linear;
```

---

## 📱 RESPONSIVE & ADAPTABILITY

### Window Sizing

**Minimum:**
- Width: 1280px
- Height: 720px

**Optimal:**
- Width: 1440px
- Height: 900px

**Comportement < 1280px:**
- Warning toast: "For best experience, resize window to at least 1280px"
- Compact mode auto-activé
- Inspector panel auto-collapsed

### Panel Behaviors

**Nav Rail:**
- Fixed 64px
- Can't resize
- Can collapse to icon-only (32px) with setting

**Main Workspace:**
- Flexible, takes remaining space
- Min-width: 600px

**Inspector Panel:**
- Default: 360px
- Resizable: 300-500px range
- Collapsible (toggle Cmd+D)
- Resize handle: 6px visible bar
- State persisted in localStorage

**Now Playing Bar:**
- Auto-height based on playing tracks
- Collapsible (Ctrl+P)
- State persisted

### Density Modes

**Comfortable (default):**
- Spacing: 8px scale
- Font: 14px body
- Cards: 80x80px
- Padding generous

**Compact:**
- Spacing: 4px scale
- Font: 13px body
- Cards: 64x64px
- Reduced padding (-25%)

**Spacious:**
- Spacing: 12px scale
- Font: 15px body
- Cards: 96x96px
- Increased padding (+25%)

Setting: `Settings > Appearance > Density`

---

## ✨ ANIMATIONS & INTERACTIONS DÉTAILLÉES

### Page Transitions

**Section Switch:**
```css
/* Out */
opacity: 0
transform: translateX(-20px)
duration: 200ms

/* In */
opacity: 1
transform: translateX(0)
duration: 300ms
delay: 100ms
```

### Modal Animations

**Open:**
```css
Overlay: fade in (0 → 1) 200ms
Modal: scale (0.95 → 1) + fade in 300ms spring
```

**Close:**
```css
Modal: scale (1 → 0.95) + fade out 200ms
Overlay: fade out 200ms delay 100ms
```

### Toast Notifications

**Appear:**
```css
transform: translateX(400px)
↓ 300ms spring
transform: translateX(0)
```

**Dismiss:**
```css
transform: translateX(0)
↓ 200ms ease-out
transform: translateX(400px)
```

### Waveform Interactions

**Hover:**
```css
Cursor: crosshair
Tooltip appears showing timestamp
Vertical line preview
```

**Click:**
```css
Momentum marker animates to click position (spring)
Brief glow pulse on marker
```

**Drag:**
```css
Marker follows cursor smoothly (no lag)
Live preview during drag (optional)
Drop: gentle settle animation
```

### Key Card Interactions

**Press (playing sound):**
```css
Scale pulse (1 → 1.05 → 1) 300ms
Ring glow appears
Icon bounces
```

**Drag & Drop:**
```css
Pickup: lift + scale 1.05 + shadow-xl
Dragging: opacity 0.8, cursor grabbing
Drop target: highlight with track color
Drop: smooth settle animation
```

---

## 🎯 COMPARAISON AVANT/APRÈS

### Layout

| Aspect | Avant | Après |
|--------|-------|-------|
| Sidebar | 224px avec 4 sections qui se battent | 64px Nav Rail + sections dédiées |
| Main Content | Dense, tout ensemble | Spacieux, contextualisé |
| Details | SoundDetails 586 lignes, scrolling infini | Inspector contextuel, juste nécessaire |
| Discovery | Caché en bas sidebar | Section dédiée, immersive |
| Now Playing | Dans sidebar, compresse autres sections | Bottom bar global, toujours visible |

### Navigation

| Avant | Après |
|-------|-------|
| Tout dans une page surchargée | 4 sections claires (Keys, Discovery, Library, Settings) |
| Discovery invisible | Discovery = expérience immersive |
| Settings = modal dense | Settings = navigation par catégorie |
| Profile switching hidden | Profile cards visible, stats |

### Clarté Visuelle

| Avant | Après |
|-------|-------|
| KeyGrid: tailles variables | KeyGrid: grille uniforme |
| Tracks: liste horizontale compacte | Tracks: cards spacieuses, color-coded |
| Filtrage: opacity-30 (trop subtil) | Filtrage: opacity-50 + blur |
| Multi-track: texte "2T" | Multi-track: badges colorés |

### Workflow

| Avant | Après |
|-------|-------|
| Ajout son: modal dense multi-tab | Ajout son: intégré selon contexte |
| Édition son: scroll infini | Édition son: inspector focus |
| Discovery: squeeze en bottom | Discovery: mode focus dédié |
| Settings: modal tout-en-un | Settings: catégories organisées |

---

## 📊 MÉTRIQUES DE SUCCÈS

### Objectifs UX

- [ ] **Discoverability**: Nouvelles features trouvables en <30s
- [ ] **Efficacité**: Tâches courantes (add sound, assign key) en <5 clicks
- [ ] **Clarté**: Utilisateurs comprennent l'interface sans tutorial
- [ ] **Respiration**: Aucun élément ne se sent "serré" ou cluttered

### KPIs

- **Time to First Sound**: <2 minutes (nouveau user)
- **Feature Discovery Rate**: 80% des users trouvent Discovery
- **Error Rate**: <5% des actions résultent en erreur
- **User Satisfaction**: 4.5/5 stars minimum

### Tests

- [ ] Usability testing (5-10 users)
- [ ] A/B testing ancien vs nouveau layout
- [ ] Accessibility audit (Lighthouse score >90)
- [ ] Performance testing (60fps animations)

---

## 🚀 PLAN D'IMPLÉMENTATION

### Phase 1: Fondations (3-4 semaines)

**Objectif:** Nouveau layout + design system

- [ ] Design system complet (tokens, composants)
- [ ] Layout principal (Header, Nav Rail, Workspace, Inspector)
- [ ] Now Playing Bar
- [ ] Navigation entre sections
- [ ] Système d'animations

**Fichiers:**
- `src/styles/tokens.css` (nouveau)
- `src/styles/animations.css` (nouveau)
- `src/components/Layout/` (refonte complète)
- `src/components/common/Button.tsx` (nouveau composant)
- `src/components/common/Card.tsx` (nouveau)
- `src/components/common/Input.tsx` (nouveau)

---

### Phase 2: Keys Section (2-3 semaines)

**Objectif:** Vue principale optimisée

- [ ] Tracks Bar redesign
- [ ] KeyGrid avec grille uniforme
- [ ] Inspector Panel pour sound details
- [ ] Search/Filter bar moderne
- [ ] Track color-coding système

**Fichiers:**
- `src/components/Keys/` (refonte)
- `src/components/Tracks/TrackCard.tsx` (nouveau)
- `src/components/Inspector/SoundInspector.tsx` (nouveau)

---

### Phase 3: Discovery Section (2 semaines)

**Objectif:** Expérience immersive

- [ ] Carousel redesign (3-card layout)
- [ ] Quick Add Setup bar
- [ ] Disliked videos panel
- [ ] Preview player amélioré

**Fichiers:**
- `src/components/Discovery/` (refonte)
- `src/components/Discovery/CarouselCard.tsx` (nouveau)

---

### Phase 4: Library Section (2 semaines)

**Objectif:** Organisation claire

- [ ] Profile cards design
- [ ] All Sounds liste table-style
- [ ] Inspector pour profile stats
- [ ] Import/Export UI

**Fichiers:**
- `src/components/Library/` (nouveau)
- `src/components/Profiles/ProfileCard.tsx` (refonte)

---

### Phase 5: Settings Section (1-2 semaines)

**Objectif:** Navigation claire

- [ ] Sidebar navigation
- [ ] Panels par catégorie
- [ ] Tous les settings organisés

**Fichiers:**
- `src/components/Settings/` (refonte complète)
- `src/components/Settings/SettingsSidebar.tsx` (nouveau)

---

### Phase 6: Polish & Animations (1-2 semaines)

**Objectif:** Micro-interactions fluides

- [ ] Toutes les animations spring
- [ ] Hover states partout
- [ ] Loading states (shimmer)
- [ ] Success animations
- [ ] Error states visuels

---

### Phase 7: Testing & Refinement (1 semaine)

- [ ] Usability testing
- [ ] Bug fixes
- [ ] Performance optimization
- [ ] Accessibility audit
- [ ] Documentation

---

**Total estimé: 12-16 semaines**

---

## 📝 NOTES FINALES

### Préserve 100% des Fonctionnalités

Cette refonte **ne retire AUCUNE fonctionnalité**. Tout ce qui existe actuellement existera encore, juste mieux organisé et présenté.

### Migration Utilisateur

**Onboarding pour existing users:**
- "What's New" modal au premier lancement v2.0
- Guided tour optionnel (skip possible)
- Settings migrés automatiquement
- Profiles et données inchangés

### Évolutivité

Ce nouveau layout permet d'ajouter facilement :
- Nouvelles sections (Analytics, Community, etc.)
- Nouveaux types de tracks
- Plugins/Extensions
- Collaborative features

### Maintenance

- Design system documenté (Storybook?)
- Composants réutilisables
- Tokens centralisés
- Tests visuels (Chromatic?)

---

## ✅ PROCHAINES ÉTAPES

1. **Validation** de cette vision par l'équipe/owner
2. **Feedback** sur les choix de design
3. **Prototypage** d'un mockup interactif (Figma?)
4. **Planning** détaillé des sprints
5. **Implémentation** phase par phase

---

**Cette refonte transformera KeyToMusic en une application qui respire, moderne, professionnelle, et qui fait vraiment 2026.** 🚀
