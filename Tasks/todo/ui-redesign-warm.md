# 🎨 KeyToMusic 2.0 - Refonte UI (Version Chaleureuse)

**Status:** Proposition alternative - Version accueillante (pas "usine")

**Philosophie:** "Moderne et sobre, mais chaleureux et invitant - pas un moteur de jeu ou un outil de prod"

---

## 📋 POURQUOI CETTE VERSION ?

### Le Problème de la Version Précédente

La première proposition (ui-redesign-complete.md) était trop :
- ❌ **"Usine"** - Nav Rail, Inspector Panel, trop Figma/Linear
- ❌ **Technique** - Rappelait Unity/Unreal Engine
- ❌ **Froid** - Workspace mindset, pas consumer-friendly
- ❌ **Rigid** - Trop de grilles, panels, séparateurs

### Cette Nouvelle Vision

Cette version est :
- ✅ **Chaleureuse** - Accueillante, invitante
- ✅ **Fluide** - Flow naturel, pas de grilles rigides
- ✅ **Vivante** - Gradients, animations spring, personnalité
- ✅ **Moderne 2026** - Clean mais pas stérile

---

## 🎯 INSPIRATIONS

### OUI ✅ (Consumer Apps Modernes)

**Spotify:**
- Sidebar chaleureuse avec sections claires
- Cards avec artwork
- Flow naturel, pas rigide
- Colors vivantes mais sobres

**Apple Music:**
- Clean mais pas froid
- Typography invitante
- Gradients subtils
- Animations douces

**Discord:**
- Personnalité sans être too much
- Sidebar colorée mais sobre
- Rounds corners everywhere
- Fun mais professionnel

**Raycast:**
- Rapide, moderne
- Mais warm, pas sterile
- Micro-interactions soignées
- Clean avec personnalité

**Things 3:**
- Minimaliste mais invitant
- Whitespace généreux
- Typography belle
- Animations spring

### NON ❌ (Outils de Prod/Dev)

- Unity/Unreal Engine (trop technique, panels partout)
- Figma/Linear (trop workspace)
- VS Code (dev tool aesthetic)
- Blender (trop dense, trop gris)

---

## 📐 ARCHITECTURE GLOBALE

### Layout Principal

```
┌──────────────────────────────────────────────────────────────────┐
│  ◉ KeyToMusic          【 My Manga Profile ▼ 】        ♫ 75%  ⚙  │
├──────────┬───────────────────────────────────────────────────────┤
│          │                                                        │
│  Home    │                                                        │
│  Keys    │                                                        │
│  Library │            MAIN CONTENT                                │
│  ──────  │            (Flow naturel, pas de grille rigide)        │
│  Discover│                                                        │
│          │                                                        │
│  ──────  │                                                        │
│  Now     │                                                        │
│  Playing │                                                        │
│          │                                                        │
│  ──────  │                                                        │
│  Settings│                                                        │
│          │                                                        │
└──────────┴───────────────────────────────────────────────────────┘
```

### Zones Principales

#### 1. Header (Haut, 56px)
```
◉ KeyToMusic    【 My Manga Profile ▼ 】    ♫ Master 75%    ⚙️  ?
```

**Éléments:**
- Logo + nom app (gauche)
- Profile switcher - Dropdown avec preview (centre)
- Master volume - Slider compact avec icon (droite)
- Settings & Help - Icons (extrême droite)

**Style:**
- Background avec gradient subtil
- Separator doux en bottom (pas ligne dure)
- Padding généreux

#### 2. Sidebar (Gauche, 200px)

**Sections:**
```
Home       ← Dashboard accueillant
Keys       ← Vue principale
Library    ← Organisation
───────
Discover   ← Séparé visuellement
───────
Now Playing ← État actuel
───────
Settings   ← En bas
```

**Style:**
- Text + icons (pas juste icons)
- Spacing généreux entre sections
- Active state: gradient background + border left accent
- Hover: subtle background elevate
- Border-radius sur les items (12px)

**Now Playing Section:**
```
┌─────────────────┐
│ Now Playing     │
├─────────────────┤
│ 🎵 Demon Slayer │
│    OST • 1:23   │
│    [Progress]   │
│    ⏹           │
├─────────────────┤
│ 🎵 Rain         │
│    AMB • 0:45   │
│    [Progress]   │
│    ⏹           │
└─────────────────┘
```

- Compact cards par track
- Waveform miniature animée
- Click pour expand details

#### 3. Main Content (Centre, flexible)

**Caractéristiques:**
- Padding généreux (32px)
- Background légèrement différent du sidebar
- Scroll smooth
- **Pas de panels rigides**
- Flow naturel de haut en bas
- Content s'adapte au contexte

---

## 🏠 SECTION: HOME (Dashboard Accueillant)

**Vue par défaut - Warm welcome**

```
┌──────────────────────────────────────────────────────────────┐
│                                                               │
│   Good afternoon, Mehdi! 👋                                  │
│                                                               │
│   ┌─────────────────────────────────────────────────────┐   │
│   │                                                      │   │
│   │  🎵 127 sounds    🎹 89 keys    ⏱️ 8h 23m music    │   │
│   │                                                      │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                               │
│   Quick Actions                                              │
│                                                               │
│   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│   │              │  │              │  │              │    │
│   │      🎵      │  │      ⭐      │  │      📚      │    │
│   │              │  │              │  │              │    │
│   │  Add Sound   │  │   Discover   │  │    Import    │    │
│   │              │  │   New Music  │  │   Profile    │    │
│   │              │  │              │  │              │    │
│   └──────────────┘  └──────────────┘  └──────────────┘    │
│                                                               │
│   Recent Activity                                            │
│                                                               │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ 🎵 Added "Bleach OP 13"              2 hours ago    │   │
│   │    ⚡ Auto-momentum detected at 2.3s                │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                               │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ 🎵 Played "Demon Slayer OP"          5 hours ago    │   │
│   │    🔁 Looped 12 times on track OST                  │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                               │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ 📊 Discovered 8 new songs            Yesterday      │   │
│   │    ✨ 3 added to your library                       │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                               │
│   Most Played This Week                                      │
│                                                               │
│   ┌─────────────────────────────────────────────────────┐   │
│   │                                                      │   │
│   │  🥇  Demon Slayer OP              342 plays         │   │
│   │      [Mini waveform] ▁▂▃▅▇█▇▅▃▂▁                   │   │
│   │                                                      │   │
│   │  🥈  Attack on Titan OP            289 plays         │   │
│   │      [Mini waveform] ▁▃▅▇█▅▃▁▂▄▆█                   │   │
│   │                                                      │   │
│   │  🥉  JJK Opening                   203 plays         │   │
│   │      [Mini waveform] ▂▄▆█▆▄▂▁▃▅▇█                   │   │
│   │                                                      │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                               │
│   Tips & Tricks                                              │
│                                                               │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ 💡 Did you know?                                    │   │
│   │    Press Shift while assigning a key to use         │   │
│   │    momentum automatically! ⚡                        │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

### Design Details - Home

#### Greeting Personnalisé
```
Good morning! ☀️      (6h-12h)
Good afternoon! 👋    (12h-18h)
Good evening! 🌙      (18h-6h)
```
- Utilise le prénom du user (settings)
- Emoji selon l'heure
- Font display (28px, bold)

#### Stats Card
- Gradient background subtil
- Icons colorés
- Numbers grands, labels petits
- Border-radius: 16px
- Subtle shadow

#### Quick Actions
**Cards (160px × 140px):**
- Hover: lift + glow
- Click: scale down (spring)
- Icon grand (48px) coloré avec gradient
- Text en dessous (16px medium)
- Border-radius: 16px
- Cursor: pointer

**Gradients par action:**
- Add Sound: Purple-pink gradient
- Discover: Cyan-blue gradient
- Import: Orange-yellow gradient

#### Recent Activity
**Timeline-style cards:**
- Icon + text + timestamp
- Secondary info en petit (grey)
- Hover: subtle highlight
- Click: navigate to item
- Border-radius: 12px
- Gap: 12px entre items

#### Most Played
**Leaderboard cards:**
- Medal emoji (🥇🥈🥉) pour top 3
- Mini waveform animée en background
- Play count prominent
- Hover: expand avec more info
- Gradient border pour #1

#### Tips & Tricks
**Rotating tips:**
- Change chaque jour
- Bulb icon
- Light background (pas dark)
- Dismiss button (subtle X)
- Link "Learn more" si applicable

---

## 🎹 SECTION: KEYS (Vue Principale)

**Flow naturel, pas de grille rigide ni inspector panel**

```
┌──────────────────────────────────────────────────────────────┐
│                                                               │
│   Your Tracks                                                │
│                                                               │
│   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│   │             │  │             │  │             │        │
│   │    OST      │  │  Ambiance   │  │     SFX     │    +   │
│   │             │  │             │  │             │        │
│   │  ▁▂▃▅▇█▇▅  │  │  ▁▃▅▇█▅▃▁  │  │  ▂▄▆█▆▄▂▁  │        │
│   │             │  │             │  │             │        │
│   │   🔊 80%    │  │   🔊 60%    │  │   🔊 90%    │        │
│   │ 🟢 Playing  │  │             │  │             │        │
│   │             │  │             │  │             │        │
│   └─────────────┘  └─────────────┘  └─────────────┘        │
│                                                               │
│   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                               │
│   🔍 Search keys, tracks, or sounds...                       │
│                                                               │
│   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                               │
│   Key Bindings                                   🎵 Add Sound│
│                                                               │
│   ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐                │
│   │    │ │    │ │    │ │    │ │    │ │    │                │
│   │ A  │ │ Z  │ │ E  │ │ R  │ │ T  │ │ Y  │                │
│   │    │ │    │ │    │ │    │ │    │ │    │                │
│   │ 🎵 │ │🎵🎵│ │    │ │ 🎵 │ │ 🎵 │ │    │                │
│   │OST │ │Mix │ │    │ │SFX │ │AMB │ │    │                │
│   │    │ │    │ │    │ │    │ │    │ │    │                │
│   └────┘ └────┘ └────┘ └────┘ └────┘ └────┘                │
│                                                               │
│   ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐                │
│   │ Q  │ │ S  │ │ D  │ │ F  │ │ G  │ │ H  │                │
│   │ 🎵 │ │ 🎵 │ │ 🎵 │ │ 🎵 │ │    │ │    │                │
│   │VCE │ │OST │ │OST │ │SFX │ │    │ │    │                │
│   └────┘ └────┘ └────┘ └────┘ └────┘ └────┘                │
│                                                               │
│   ... (grid continue avec spacing généreux)                 │
│                                                               │
│   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                               │
│   Selected: Key Z                                            │
│                                                               │
│   ┌──────────────────────────────────────────────────────┐  │
│   │                                                       │  │
│   │  🎵 Demon Slayer OP                          OST     │  │
│   │  ─────────────────────────────────────────────────   │  │
│   │                                                       │  │
│   │  [Waveform Display - Click to set momentum]          │  │
│   │  ▁▂▃▅▇█▇▅▃▂▁▂▃▅▇█▇▅▃▂▁▂▃▅▇█▇▅▃▂▁                  │  │
│   │              ↑ 2.3s (AI suggested ✨)                │  │
│   │                                                       │  │
│   │  ─────────────────────────────────────────────────   │  │
│   │                                                       │  │
│   │  Volume    ████████░░ 80%                            │  │
│   │  Loop      Sequential ▼                              │  │
│   │  Momentum  2.3s                                      │  │
│   │                                                       │  │
│   │  ─────────────────────────────────────────────────   │  │
│   │                                                       │  │
│   │  ▶ Preview    ✏️ Edit Details    🗑️ Remove          │  │
│   │                                                       │  │
│   └──────────────────────────────────────────────────────┘  │
│                                                               │
│   ┌──────────────────────────────────────────────────────┐  │
│   │  🎵 Attack on Titan OP                       OST     │  │
│   │  (same card structure)                               │  │
│   └──────────────────────────────────────────────────────┘  │
│                                                               │
│   ┌────────────────────────────────────────┐                │
│   │  ✏️ Reassign to another key            │                │
│   │  🗑️ Remove all sounds from this key    │                │
│   └────────────────────────────────────────┘                │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

### Design Details - Keys Section

#### Tracks Bar

**Cards (180px × 160px):**
```
┌─────────────┐
│             │
│    OST      │  ← Track name (18px, medium)
│             │
│ ▁▂▃▅▇█▇▅▃▂ │  ← Mini waveform animée (playing)
│             │
│  🔊 80%     │  ← Volume (hover: slider appears)
│ 🟢 Playing  │  ← Status badge
│             │
└─────────────┘
```

**Styling:**
- Background: Gradient subtil avec track color
- Border: 2px track color (subtle)
- Border-radius: 16px
- Shadow: Soft elevation
- Playing state: Pulsing glow

**Hover:**
- Lift slightly
- Volume slider appears (smooth transition)
- Solo/Mute icons appear

**Click:**
- Opens track settings (inline modal)
- Rename, change color, delete

**+ Track button:**
- Dashed border
- Icon centered
- Hover: solid border + lift

#### Search Bar

**Full-width input:**
```
🔍 Search keys, tracks, or sounds...
```

- Large (48px height)
- Icon left (20px from edge)
- Placeholder grey
- Border-radius: 12px
- Focus: Glow accent
- Clear button appears when typing

**Search behavior:**
- Instant filter (no debounce needed)
- Highlights matches
- Shows count "12 results"
- Escape to clear

#### Key Grid

**Layout:**
- CSS Grid with auto-fill
- Cards: 88px × 88px (bigger than avant)
- Gap: 16px (plus généreux)
- Responsive (moins de cards par row si window small)

**Card Design:**
```
┌────┐
│    │
│ A  │  ← Keycode (16px, bold, mono)
│    │
│ 🎵 │  ← Icon (filled style)
│OST │  ← Track name (12px, track color)
│    │
└────┘
```

**États:**

1. **Empty:**
   - Dashed border (subtle)
   - No background
   - Text "Empty" (opacity 0.3)
   - Hover: Solid border + "Add sound"

2. **Assigned (1 sound):**
   - Solid border (track color, 2px)
   - Background gradient (track color, very subtle)
   - Icon filled (track color)
   - Track name visible

3. **Multi-sounds:**
   - Badge "×2" top-right corner
   - Multiple colored dots bottom (one per track)
   - Icon: Multiple music notes

4. **Playing:**
   - Pulsing glow (track color)
   - Animated icon (bounce)
   - Border thicker (3px)

5. **Selected:**
   - Border accent (4px, purple)
   - Background slightly elevated
   - Details show below grid

6. **Filtered (search):**
   - Opacity: 0.4
   - Scale: 0.98
   - Blur: 1px

**Interactions:**
- Click: Select (expand details below)
- Double-click: Play sound preview
- Ctrl+Click: Multi-select
- Drag: Reassign (shows ghost + drop targets)
- Right-click: Context menu

#### Sound Details (Below Grid)

**Apparaît quand key sélectionnée**

**Card per sound:**
```
┌──────────────────────────────────────────────┐
│                                               │
│  🎵 Sound Name                       Track ▼  │
│  ───────────────────────────────────────────  │
│                                               │
│  [Waveform - Interactive]                    │
│  ▁▂▃▅▇█▇▅▃▂▁▂▃▅▇█▇▅▃▂▁                      │
│          ↑ 2.3s (AI ✨)                       │
│                                               │
│  ───────────────────────────────────────────  │
│                                               │
│  Volume    [Slider] 80%                      │
│  Loop      [Dropdown] Sequential             │
│  Momentum  [Input] 2.3s                      │
│                                               │
│  ───────────────────────────────────────────  │
│                                               │
│  ▶ Preview    ✏️ Edit    🗑️ Remove           │
│                                               │
└──────────────────────────────────────────────┘
```

**Styling:**
- Background: Surface elevated
- Border-radius: 16px
- Padding: 24px
- Shadow: Medium
- Separators: Subtle lines

**Waveform:**
- Height: 100px (plus grand qu'avant)
- Interactive: Click to set momentum
- Hover: Crosshair cursor + timestamp tooltip
- Drag marker: Smooth, spring animation
- AI suggestion: Cyan dashed line + sparkle icon

**Controls:**
- Sliders: Large, easy to grab
- Dropdowns: Rounded, avec icons
- Inputs: Clear, large touch targets

**Actions:**
- Preview: Plays in preview track (not main)
- Edit: Opens expanded modal si needed
- Remove: Confirmation tooltip inline

**Si multiple sounds:**
- Cards stack vertically
- Gap 16px entre cards
- Collapsible (click header to collapse)

#### Bulk Actions (When multi-select)

**Shows at bottom:**
```
┌────────────────────────────────────────┐
│  3 keys selected                       │
│  ───────────────────────────────────   │
│  Change Track    Delete All    Cancel  │
└────────────────────────────────────────┘
```

---

## ⭐ SECTION: DISCOVER (Immersive, Focus Mode)

**Single card focus, pas de carousel multi-cards**

```
┌──────────────────────────────────────────────────────────────┐
│                                                               │
│   Discover New Music ✨                                      │
│   Based on what you love                                     │
│                                                               │
│   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                               │
│   ┌──────────────────────────────────────────────────┐      │
│   │                                                   │      │
│   │                                                   │      │
│   │              ┌──────────────────┐                │      │
│   │              │                  │                │      │
│   │              │                  │                │      │
│   │              │   [Thumbnail]    │                │      │
│   │     ◄        │                  │        ►       │      │
│   │              │  Bleach OP 13    │                │      │
│   │              │                  │                │      │
│   │              │  ▁▃▅▇█▅▃▁▂▄▆█   │                │      │
│   │              │                  │                │      │
│   │              │  ⚡ 2.3s  📊 3:45│                │      │
│   │              │                  │                │      │
│   │              │  ▶ Preview       │                │      │
│   │              │  Vol ░░░░░░░▓▓  │                │      │
│   │              │                  │                │      │
│   │              │  ┌──────┐ ┌───┐ │                │      │
│   │              │  │ Add  │ │ 👎│ │                │      │
│   │              │  └──────┘ └───┘ │                │      │
│   │              │                  │                │      │
│   │              └──────────────────┘                │      │
│   │                                                   │      │
│   │                  ● ○ ○ ○ ○                       │      │
│   │                 1 / 30                            │      │
│   │                                                   │      │
│   └──────────────────────────────────────────────────┘      │
│                                                               │
│   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                               │
│   ┌─────────────────────────────────────────────────────┐   │
│   │  Quick Add:  Press a key ⌨️   Track: OST ▼   ⚡ Auto│   │
│   └─────────────────────────────────────────────────────┘   │
│                                                               │
│   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                               │
│   Disliked Videos (5)    [Expand ▼]                          │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

### Design Details - Discovery

#### Header
```
Discover New Music ✨
Based on what you love
```
- Display font (24px bold)
- Sparkle emoji pour fun
- Secondary text grey

#### Main Card (320px × 480px)

**Structure:**
```
┌──────────────────┐
│  [Thumbnail]     │  ← 320×180px, 16:9
│                  │
│  Title           │  ← 18px medium, 2 lines max
│                  │
│  [Waveform]      │  ← 60px height, interactive
│  ⚡ 2.3s         │  ← Momentum badge
│  📊 3:45         │  ← Duration
│                  │
│  ▶ Preview       │  ← Play button
│  [Volume slider] │  ← Appears on hover/playing
│                  │
│  [Add] [Dislike] │  ← Primary actions
└──────────────────┘
```

**Thumbnail:**
- YouTube artwork si dispo
- Sinon: Gradient avec waveform preview
- Border-radius: 12px top
- Lazy loading

**Waveform:**
- Same style que Keys section
- Interactive (click to set momentum)
- Suggested momentum highlighted (cyan)

**Preview:**
- Large button, inviting
- Plays in preview track
- Volume slider appears (horizontal)
- Playing state: ⏸ icon, waveform animée

**Actions:**
- **Add button**: Large, gradient background, white text
- **Dislike**: Secondary style, smaller
- Both have haptic-style feedback

**Navigation:**
- Large ◄ ► arrows clickable
- Keyboard: Arrow keys
- Swipe gesture (trackpad)

**Pagination:**
- Dots indicator (● ○ ○)
- Text "1 / 30" below
- Current dot: gradient fill

#### Container
- Centered in viewport
- Gradient background (very subtle)
- Shadow around card (elevation)

#### Quick Add Setup

**Bar at bottom:**
```
┌─────────────────────────────────────────────┐
│ Quick Add:  [Press...⌨️]  [OST▼]  [⚡Auto] │
└─────────────────────────────────────────────┘
```

- Sticky à la vue (scroll avec content)
- Background surface elevated
- Border-radius: 12px
- Shadow: Medium

**Elements:**
- Key capture: Large button, purple when capturing
- Track selector: Dropdown avec track colors
- Momentum mode: Dropdown (Auto/Manual/None)
- Settings persist across navigation

#### Disliked Panel

**Collapsed:**
```
Disliked Videos (5)  [Expand ▼]
```

**Expanded:**
```
┌─────────────────────────────────────────┐
│ Disliked Videos                         │
├─────────────────────────────────────────┤
│ 🎵 Video Title 1         [Undislike]   │
│ 🎵 Video Title 2         [Undislike]   │
│ 🎵 Video Title 3         [Undislike]   │
├─────────────────────────────────────────┤
│ [Clear All]                              │
└─────────────────────────────────────────┘
```

- Max height: 200px, scrollable
- Items: Hover highlight
- Undislike: Instant, no confirmation
- Clear All: Confirmation dialog

---

## 📚 SECTION: LIBRARY (Visual, Pas Table)

```
┌──────────────────────────────────────────────────────────────┐
│                                                               │
│   Your Library                                               │
│                                                               │
│   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                               │
│   Profiles                                                   │
│                                                               │
│   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│   │    📁       │  │    📁       │  │    📁       │        │
│   │             │  │             │  │             │        │
│   │   Manga     │  │   Chill     │  │   Gaming    │    +   │
│   │             │  │             │  │             │        │
│   │  [Preview]  │  │  [Preview]  │  │  [Preview]  │        │
│   │             │  │             │  │             │        │
│   │ 127 sounds  │  │  45 sounds  │  │  89 sounds  │        │
│   │ 4 tracks    │  │  3 tracks   │  │  5 tracks   │        │
│   │ 2.3 GB      │  │  0.8 GB     │  │  1.5 GB     │        │
│   │             │  │             │  │             │        │
│   │ ⭐ Active   │  │   Switch    │  │   Switch    │        │
│   │  ⋮ More    │  │   ⋮ More   │  │   ⋮ More   │        │
│   │             │  │             │  │             │        │
│   └─────────────┘  └─────────────┘  └─────────────┘        │
│                                                               │
│   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                               │
│   All Sounds                                                 │
│                                                               │
│   🔍 Search...              Sort: Recent ▼    Filter: All ▼  │
│                                                               │
│   127 sounds  •  8h 23m total                                │
│                                                               │
│   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                               │
│   ┌──────────────────────────────────────────────────────┐  │
│   │                                                       │  │
│   │  🎵 Demon Slayer OP                                  │  │
│   │                                                       │  │
│   │  ▁▂▃▅▇█▇▅▃▂▁▂▃▅▇  3:45  •  OST  •  Key Z  •  ⚡2.3s│  │
│   │  🔊 80%  •  🔁 Sequential                            │  │
│   │                                                       │  │
│   │  ▶ Preview    ✏️ Edit    🗑️ Delete                  │  │
│   │                                                       │  │
│   └──────────────────────────────────────────────────────┘  │
│                                                               │
│   ┌──────────────────────────────────────────────────────┐  │
│   │  🎵 Attack on Titan OP                               │  │
│   │  ▁▃▅▇█▅▃▁  4:12  •  OST  •  Key A  •  ⚡0.0s        │  │
│   │  🔊 100%  •  🔁 Random                                │  │
│   │  ▶ Preview    ✏️ Edit    🗑️ Delete                  │  │
│   └──────────────────────────────────────────────────────┘  │
│                                                               │
│   ... (liste continue)                                       │
│                                                               │
│   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                               │
│   ┌─────────────────────────────────────────────────────┐   │
│   │  💾 Import Profile    📤 Export    🗑️ Cleanup Cache │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

### Design Details - Library

#### Profile Cards (200px × 280px)

**Structure:**
```
┌─────────────┐
│    📁       │  ← Icon (customizable)
│             │
│   Manga     │  ← Name (18px medium)
│             │
│  [Preview]  │  ← Waveform montage ou thumbnail
│             │
│ 127 sounds  │  ← Stats (14px grey)
│ 4 tracks    │
│ 2.3 GB      │
│             │
│ ⭐ Active   │  ← Badge (si active) ou "Switch" button
│  ⋮ More    │  ← Menu
│             │
└─────────────┘
```

**Preview:**
- Soit: Montage de 3-4 waveforms
- Soit: Custom thumbnail (upload dans settings)
- Border-radius: 12px
- Subtle animation on hover

**Active profile:**
- Badge gradient "⭐ Active"
- Border accent (2px purple)
- Slightly elevated

**Switch button:**
- Primary style pour non-active
- Click: Confirmation si sounds playing
- Smooth transition

**More menu (⋮):**
- Rename
- Change Icon/Thumbnail
- Duplicate
- Export
- Delete

**+ New button:**
- Dashed border card
- Plus icon centered
- Hover: Solid + lift

#### All Sounds Section

**Header:**
```
🔍 Search...    Sort: Recent ▼    Filter: All ▼
127 sounds  •  8h 23m total
```

- Search: Same style as Keys section
- Sort: Dropdown (Name, Date Added, Duration, Most Played)
- Filter: Dropdown (All, Track, Assigned, Unassigned)
- Stats: Below in grey

**Sound Cards:**

**Structure (full width):**
```
┌──────────────────────────────────────────────┐
│ 🎵 Sound Name                                │
│                                               │
│ [Waveform] Duration • Track • Key • Momentum │
│ Volume • Loop                                │
│                                               │
│ Actions                                      │
└──────────────────────────────────────────────┘
```

**Layout:**
- Icon left (24px)
- Name (18px medium, truncated)
- Waveform mini (full width, 40px height)
- Metadata en bullets (•) pas en colonnes
- Actions en bottom

**Hover:**
- Background elevate
- Actions buttons appear
- Waveform highlights

**Click:**
- Expands avec full details (inline)
- Même structure que Keys section sound cards
- Collapse click again

**Actions:**
- Preview: Play in preview track
- Edit: Expand card ou open modal
- Delete: Inline confirmation

#### Actions Bar (Bottom)

```
┌─────────────────────────────────────────────┐
│ 💾 Import    📤 Export    🗑️ Cleanup Cache  │
└─────────────────────────────────────────────┘
```

- Ghost buttons
- Hover: Background + lift
- Import: Opens file picker
- Export: Opens save dialog
- Cleanup: Confirmation dialog avec stats

---

## ⚙️ SECTION: SETTINGS (Simple List, Pas Navigation Latérale)

**Simple scrollable list, grouped par catégorie**

```
┌──────────────────────────────────────────────────────────────┐
│                                                               │
│   Settings                                                   │
│                                                               │
│   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                               │
│   Audio                                                      │
│                                                               │
│   ┌─────────────────────────────────────────────────────┐   │
│   │                                                      │   │
│   │  Output Device                                      │   │
│   │  ┌────────────────────────────────────────────┐    │   │
│   │  │ Realtek High Definition Audio          ▼  │    │   │
│   │  └────────────────────────────────────────────┘    │   │
│   │  [🔄 Refresh Devices]                              │   │
│   │                                                      │   │
│   │  ──────────────────────────────────────────────    │   │
│   │                                                      │   │
│   │  Crossfade Duration                                │   │
│   │  [Slider] 500ms                                    │   │
│   │  Smooth transitions between sounds                 │   │
│   │                                                      │   │
│   │  ──────────────────────────────────────────────    │   │
│   │                                                      │   │
│   │  Master Volume                                     │   │
│   │  [Slider] 75%                                      │   │
│   │                                                      │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                               │
│   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                               │
│   Keyboard                                                   │
│                                                               │
│   ┌─────────────────────────────────────────────────────┐   │
│   │                                                      │   │
│   │  Global Shortcuts                                   │   │
│   │                                                      │   │
│   │  Stop All Sounds    [Ctrl + Shift + S]  [Change]   │   │
│   │  Toggle Detection   [Ctrl + K]          [Change]   │   │
│   │  Auto-Momentum      [Ctrl + M]          [Change]   │   │
│   │                                                      │   │
│   │  ──────────────────────────────────────────────    │   │
│   │                                                      │   │
│   │  Key Detection                                      │   │
│   │  ☑ Enabled                                          │   │
│   │                                                      │   │
│   │  Cooldown    [Slider] 200ms                        │   │
│   │  Prevent accidental double presses                 │   │
│   │                                                      │   │
│   │  Chord Window    [Slider] 30ms                     │   │
│   │  Time to press multiple keys                       │   │
│   │                                                      │   │
│   │  ──────────────────────────────────────────────    │   │
│   │                                                      │   │
│   │  Momentum Modifier                                  │   │
│   │  [Dropdown] Shift ▼                                │   │
│   │  Hold while pressing a key to use momentum         │   │
│   │                                                      │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                               │
│   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                               │
│   Appearance                                                 │
│                                                               │
│   ┌─────────────────────────────────────────────────────┐   │
│   │                                                      │   │
│   │  Theme                                              │   │
│   │  ┌────┐ ┌────┐ ┌────┐ ┌────┐                      │   │
│   │  │ 🟣 │ │ 🌹 │ │ 🌊 │ │ 🌅 │                      │   │
│   │  │Indigo│Rose│Teal│Amber                      │   │
│   │  └────┘ └────┘ └────┘ └────┘                      │   │
│   │  ⭐ Active                                          │   │
│   │                                                      │   │
│   │  ──────────────────────────────────────────────    │   │
│   │                                                      │   │
│   │  Interface Density                                  │   │
│   │  ┌──────┐ ┌──────┐ ┌──────┐                       │   │
│   │  │Compact│ Default│Spacious                       │   │
│   │  └──────┘ └──────┘ └──────┘                       │   │
│   │          ⭐ Active                                  │   │
│   │                                                      │   │
│   │  ──────────────────────────────────────────────    │   │
│   │                                                      │   │
│   │  Animations                                         │   │
│   │  ☑ Enable smooth animations                        │   │
│   │  ☐ Reduce motion (accessibility)                   │   │
│   │                                                      │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                               │
│   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                               │
│   Advanced                                                   │
│                                                               │
│   ┌─────────────────────────────────────────────────────┐   │
│   │                                                      │   │
│   │  Performance                                        │   │
│   │  Waveform Cache Size    [Slider] 50 entries        │   │
│   │  Current usage: 28 / 50                            │   │
│   │  [Clear Cache]                                      │   │
│   │                                                      │   │
│   │  ──────────────────────────────────────────────    │   │
│   │                                                      │   │
│   │  Developer                                          │   │
│   │  [Open Data Folder]    [Open Logs]                 │   │
│   │  ☐ Enable Debug Mode                               │   │
│   │                                                      │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                               │
│   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│                                                               │
│   About                                                      │
│                                                               │
│   ┌─────────────────────────────────────────────────────┐   │
│   │                                                      │   │
│   │  KeyToMusic                                         │   │
│   │  Version 2.0.0                                      │   │
│   │  Build 2026.02.02                                   │   │
│   │                                                      │   │
│   │  A soundboard for manga reading                     │   │
│   │                                                      │   │
│   │  ──────────────────────────────────────────────    │   │
│   │                                                      │   │
│   │  🌐 Website    📖 Docs    🐛 Report    💡 Request  │   │
│   │                                                      │   │
│   │  [Check for Updates]                                │   │
│   │                                                      │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

### Design Details - Settings

**Simple scrollable list** - Pas besoin de navigation latérale

**Structure par catégorie:**
1. Category title (18px medium)
2. Big card avec tous les settings
3. Separator
4. Next category

**Card styling:**
- Background: Surface elevated
- Border-radius: 16px
- Padding: 24px
- Internal separators: Subtle lines

**Setting patterns:**

**Label + Control:**
```
Setting Name
[Control]
Help text (grey, small)
```

**Shortcuts:**
```
Action Name    [Current]  [Change]
```
- Click Change: Key capture mode
- Capturing: Purple glow, "Press keys..."

**Sliders:**
- Large, easy to drag
- Value display inline
- Help text below

**Toggles:**
- Checkbox style (☑/☐)
- Large touch target
- Label clickable

**Dropdowns:**
- Rounded, avec icon
- Hover: Elevate
- Open: Smooth slide down

**Theme selector:**
- Visual cards (emoji + name)
- Click to activate
- Active: Badge + glow

**Density selector:**
- 3 buttons (radio style)
- Visual preview on hover
- Active: Highlighted

---

## 🎨 DESIGN SYSTEM (Chaleureux & Moderne)

### Couleurs Plus Vivantes

#### Backgrounds avec Gradients

**App Background:**
```css
background: linear-gradient(135deg, #0A0A0A 0%, #0F0A12 100%);
/* Léger gradient violet très subtil */
```

**Surface:**
```css
background: linear-gradient(135deg, #161616 0%, #18151A 100%);
/* Pas juste gris flat, légère teinte */
```

**Elevated:**
```css
background: linear-gradient(135deg, #1E1E1E 0%, #1F1A22 100%);
```

#### Accents Plus Chaleureux

**Primary:**
```css
--accent-primary: #7C3AED;  /* Violet plus chaleureux */
--accent-primary-hover: #8B5CF6;

/* Gradient pour éléments importants */
--accent-gradient: linear-gradient(135deg, #7C3AED 0%, #EC4899 100%);
```

**Secondary:**
```css
--accent-secondary: #EC4899;  /* Pink */
--accent-tertiary: #06B6D4;   /* Cyan pour momentum */
```

**Semantic:**
```css
--color-success: #10B981;   /* Emerald, plus vibrant */
--color-warning: #F59E0B;   /* Amber */
--color-error: #EF4444;     /* Red */
--color-info: #3B82F6;      /* Blue */
```

#### Track Colors (Avec Gradients)

**Format: Solid + Gradient version**

```css
/* OST */
--track-ost: #7C3AED;
--track-ost-gradient: linear-gradient(135deg, #7C3AED 0%, #3B82F6 100%);

/* Ambiance */
--track-ambiance: #06B6D4;
--track-ambiance-gradient: linear-gradient(135deg, #06B6D4 0%, #10B981 100%);

/* SFX */
--track-sfx: #EC4899;
--track-sfx-gradient: linear-gradient(135deg, #EC4899 0%, #EF4444 100%);

/* Voice */
--track-voice: #F59E0B;
--track-voice-gradient: linear-gradient(135deg, #F59E0B 0%, #FBBF24 100%);
```

**Usage:**
- Border: Solid color
- Background: Gradient at 10% opacity
- Glow: Solid color at 40%

#### Text Colors

```css
--text-primary: #FFFFFF;
--text-secondary: rgba(255, 255, 255, 0.7);  /* Plus soft */
--text-tertiary: rgba(255, 255, 255, 0.4);   /* Hints */
--text-disabled: rgba(255, 255, 255, 0.2);
```

---

### Typography Plus Invitante

#### Font Stack

**Sans-serif (même qu'avant):**
```css
font-family: 'Inter Variable', -apple-system, BlinkMacSystemFont, sans-serif;
```

**Monospace:**
```css
font-family: 'JetBrains Mono', 'Fira Code', monospace;
```

#### Type Scale (Plus Généreux)

**Display (pour headers importants):**
```css
font-size: 28px;
line-height: 36px;
font-weight: 700;
letter-spacing: -0.02em;  /* Tighter pour modernité */
```

**Heading:**
```css
font-size: 20px;
line-height: 28px;
font-weight: 600;
```

**Subheading:**
```css
font-size: 18px;
line-height: 24px;
font-weight: 500;
```

**Body (15px au lieu de 14px):**
```css
font-size: 15px;
line-height: 22px;
font-weight: 400;
```

**Caption:**
```css
font-size: 13px;
line-height: 18px;
font-weight: 400;
```

**Tiny:**
```css
font-size: 11px;
line-height: 16px;
font-weight: 500;
text-transform: uppercase;
letter-spacing: 0.06em;
```

---

### Spacing (Plus Généreux)

**Base: 4px**

```css
--space-1: 4px
--space-2: 8px
--space-3: 12px
--space-4: 16px    /* Base unit */
--space-5: 20px
--space-6: 24px
--space-8: 32px    /* Common padding */
--space-10: 40px
--space-12: 48px
--space-16: 64px
```

**Usage recommandé:**
- Padding cards: 24px
- Gap entre cards: 16px
- Section spacing: 32px
- Page margins: 32px

---

### Border Radius (Plus Rond)

```css
--radius-sm: 8px     /* Badges, chips */
--radius-md: 12px    /* Buttons, inputs */
--radius-lg: 16px    /* Cards, panels */
--radius-xl: 24px    /* Modals, large containers */
--radius-full: 9999px /* Pills, circles */
```

**Tout plus rond = plus friendly**

---

### Shadows (Plus Douces avec Glow)

**Elevations:**
```css
--shadow-sm: 0 2px 8px rgba(0, 0, 0, 0.15);

--shadow-md: 0 4px 16px rgba(0, 0, 0, 0.2),
             0 0 32px rgba(124, 58, 237, 0.05);
/* Glow violet subtil */

--shadow-lg: 0 8px 24px rgba(0, 0, 0, 0.25),
             0 0 48px rgba(124, 58, 237, 0.08);

--shadow-xl: 0 16px 40px rgba(0, 0, 0, 0.3),
             0 0 64px rgba(124, 58, 237, 0.1);
```

**Glows colorés (pour playing, hover):**
```css
--glow-primary: 0 0 24px rgba(124, 58, 237, 0.4);
--glow-success: 0 0 24px rgba(16, 185, 129, 0.4);
--glow-error: 0 0 24px rgba(239, 68, 68, 0.4);
```

---

### Composants Styles

#### Buttons

**Primary (avec gradient):**
```css
background: linear-gradient(135deg, #7C3AED 0%, #8B5CF6 100%);
color: white;
padding: 10px 20px;  /* Plus généreux */
border-radius: 12px;
font-size: 15px;
font-weight: 500;
border: none;
box-shadow: 0 2px 8px rgba(124, 58, 237, 0.3);

hover:
  transform: translateY(-2px);
  box-shadow: 0 4px 16px rgba(124, 58, 237, 0.4);

active:
  transform: scale(0.98);
```

**Secondary:**
```css
background: rgba(124, 58, 237, 0.1);
color: #7C3AED;
border: 1.5px solid rgba(124, 58, 237, 0.3);
/* Same padding, radius */

hover:
  background: rgba(124, 58, 237, 0.15);
  border-color: rgba(124, 58, 237, 0.5);
```

**Ghost:**
```css
background: transparent;
color: rgba(255, 255, 255, 0.7);

hover:
  background: rgba(255, 255, 255, 0.05);
  color: white;
```

#### Cards

**Base card:**
```css
background: linear-gradient(135deg, #161616 0%, #18151A 100%);
border: 1px solid rgba(255, 255, 255, 0.05);
border-radius: 16px;
padding: 24px;
box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);

hover:
  transform: translateY(-4px) scale(1.01);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.25),
              0 0 32px rgba(124, 58, 237, 0.05);
  border-color: rgba(124, 58, 237, 0.2);
```

**Transitions:**
```css
transition: all 400ms cubic-bezier(0.34, 1.56, 0.64, 1);
/* Spring bounce effect */
```

#### Inputs

**Text Input:**
```css
background: rgba(255, 255, 255, 0.03);
border: 1.5px solid rgba(255, 255, 255, 0.1);
border-radius: 12px;
padding: 12px 16px;
font-size: 15px;
color: white;

placeholder:
  color: rgba(255, 255, 255, 0.3);

focus:
  background: rgba(255, 255, 255, 0.05);
  border-color: #7C3AED;
  box-shadow: 0 0 0 4px rgba(124, 58, 237, 0.15);
  outline: none;
```

**Range Slider:**
```css
/* Track */
height: 6px;  /* Plus épais */
background: rgba(255, 255, 255, 0.1);
border-radius: 9999px;

/* Fill */
background: linear-gradient(90deg, #7C3AED 0%, #8B5CF6 100%);

/* Thumb */
width: 20px;
height: 20px;
background: white;
border-radius: 50%;
box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);

hover (thumb):
  transform: scale(1.2);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
```

**Dropdown:**
```css
/* Same as text input */
/* + Arrow icon right */
/* + Dropdown menu with smooth slide-down */
```

---

### Animations (Spring, Bouncy)

#### Timing Functions

```css
--ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1);  /* Bounce */
--ease-smooth: cubic-bezier(0.4, 0, 0.2, 1);       /* Smooth */
--ease-out: cubic-bezier(0.16, 1, 0.3, 1);         /* Decelerate */
```

#### Durations

```css
--duration-instant: 100ms    /* Hover feedback */
--duration-fast: 200ms       /* State changes */
--duration-normal: 400ms     /* Movements */
--duration-slow: 600ms       /* Page transitions */
```

#### Keyframes

**Breathe (Playing Indicator):**
```css
@keyframes breathe {
  0%, 100% {
    transform: scale(1);
    opacity: 1;
  }
  50% {
    transform: scale(1.1);
    opacity: 0.7;
  }
}
animation: breathe 2s ease-in-out infinite;
```

**Bounce In (Modal Open):**
```css
@keyframes bounceIn {
  0% {
    opacity: 0;
    transform: scale(0.9);
  }
  50% {
    transform: scale(1.05);
  }
  100% {
    opacity: 1;
    transform: scale(1);
  }
}
animation: bounceIn 400ms cubic-bezier(0.34, 1.56, 0.64, 1);
```

**Slide Up (Toast):**
```css
@keyframes slideUp {
  from {
    opacity: 0;
    transform: translateY(100%);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
animation: slideUp 400ms cubic-bezier(0.34, 1.56, 0.64, 1);
```

**Shimmer (Loading):**
```css
@keyframes shimmer {
  0% {
    background-position: -200% 0;
  }
  100% {
    background-position: 200% 0;
  }
}
background: linear-gradient(
  90deg,
  transparent 0%,
  rgba(124, 58, 237, 0.1) 50%,
  transparent 100%
);
background-size: 200% 100%;
animation: shimmer 2s ease-in-out infinite;
```

---

### Micro-Interactions

**Button Press:**
```css
active {
  transform: scale(0.98);
  transition: transform 50ms;
}
```

**Card Hover:**
```css
hover {
  transform: translateY(-4px) scale(1.01);
  box-shadow: var(--shadow-lg);
  transition: all 400ms var(--ease-spring);
}
```

**Playing Track Pulse:**
```css
@keyframes playingPulse {
  0%, 100% {
    box-shadow: 0 0 0 2px var(--track-color);
  }
  50% {
    box-shadow: 0 0 0 6px var(--track-color),
                0 0 24px rgba(track-color, 0.4);
  }
}
animation: playingPulse 1.5s ease-in-out infinite;
```

**Waveform Stream:**
```css
/* Reveal left to right pendant loading */
clip-path: inset(0 ${100 - progress}% 0 0);
transition: clip-path 100ms linear;
```

---

### Icons (Filled, Pas Outline)

**Style:**
- Filled icons (pas outline) = plus visuel, moins technique
- Emoji mixing OK (🎵, ⭐, 📚, etc.)
- Size: 20px (small), 24px (medium), 32px (large)
- Color: Inherit from parent ou track color

**Sources:**
- Lucide Icons (filled variants)
- SF Symbols (macOS)
- Fluent Icons (Windows)
- Emoji natifs

---

## ✨ PERSONNALITÉ SUBTILE

### Greetings Personnalisés

**Selon l'heure:**
```javascript
const greeting = () => {
  const hour = new Date().getHours();
  if (hour < 12) return "Good morning! ☀️";
  if (hour < 18) return "Good afternoon! 👋";
  return "Good evening! 🌙";
};
```

**Utilise le prénom:**
```
Good afternoon, Mehdi! 👋
```
- Setting: User name (optional)
- Fallback: "Good afternoon!" si pas de nom

### Messages Encourageants

**Stats achievements:**
```
🎉 Great! You added 10 sounds this week
🌱 Your library is growing! 127 sounds now
✨ Discovered 8 new songs today
🔥 On fire! 15 sounds played today
```

**Triggers:**
- 10, 50, 100, 250 sounds milestones
- Daily discovery streaks
- High play counts
- Profile size milestones

**Display:**
- Toast notification (non-intrusive)
- Recent Activity section
- Dismissible

### Empty States Illustrés

**Structure:**
```
┌────────────────────┐
│                    │
│    [Illustration]  │  ← Simple, cute, colored
│                    │
│  Empty State Text  │  ← Friendly message
│                    │
│  ┌──────────────┐  │
│  │   Action     │  │  ← Clear CTA
│  └──────────────┘  │
│                    │
│  Help text         │  ← Optional hint
│                    │
└────────────────────┘
```

**Examples:**

**No sounds:**
```
    🎵
    ┌─┐
    │ │  Empty music box
    └─┘

No sounds yet

┌──────────────┐
│  Add Sound   │
└──────────────┘

Drag & drop audio files or click to browse
```

**No bindings:**
```
    ⌨️
    ┌───┐
    │   │  Empty keyboard
    └───┘

No key bindings

┌──────────────┐
│ Assign Keys  │
└──────────────┘

Select a sound and press a key
```

**No discovery:**
```
    ⭐
     ✨   Nothing discovered yet

┌──────────────┐
│   Discover   │
└──────────────┘

Based on your library sounds
```

### Tips Rotatifs

**Database de tips:**
```javascript
const tips = [
  "💡 Hold Shift while pressing a key to use momentum!",
  "💡 Double-click a key card to preview the sound",
  "💡 Use Ctrl+F to quickly find a key binding",
  "💡 Drag sounds between keys to reassign them",
  "💡 Click the waveform to set momentum manually",
  "💡 Discovery learns from your most played songs",
  // ... more tips
];
```

**Display:**
- Home section (rotates daily)
- Loading screens (si applicable)
- Dismissible (don't show again checkbox)
- Link to docs si applicable

---

## 🎯 COMPARAISON VERSION USINE vs CHALEUREUSE

| Aspect | Version "Usine" ❌ | Version "Chaleureuse" ✅ |
|--------|-------------------|-------------------------|
| **Layout** | Nav Rail (64px) + Inspector Panel | Sidebar (200px) + Flow naturel |
| **Navigation** | Icons only, rigid | Text + icons, sections claires |
| **Vue par défaut** | Directement Keys | Home dashboard accueillant |
| **Sections** | Workspace mindset | Dashboard + Content flow |
| **Couleurs** | Flat grays (#161616) + Indigo | Gradients subtils + Violet/Pink |
| **Typography** | 14px technique | 15px invitante |
| **Borders** | Sharp, 8-12px max | Ronds, 12-24px |
| **Shadows** | Flat, elevation only | Soft + colored glows |
| **Animations** | Linear, precise | Spring, bouncy |
| **Track cards** | Rectangulaires, technical | Rondes, avec waveform animée |
| **Key grid** | Grille rigide, fixed | Grille mais plus friendly |
| **Sound details** | Inspector panel séparé | Intégré dans le flow |
| **Discovery** | 3 cards carousel | 1 card focus immersif |
| **Library** | Table-like liste | Cards visuelles |
| **Settings** | Navigation latérale | Simple liste scrollable |
| **Empty states** | Text + icon | Illustrations + emoji |
| **Personality** | Sterile, aucune | Greetings, encouragements, tips |
| **Icons** | Outline style | Filled + emoji mix |
| **Buttons** | Solid colors | Gradients |
| **Gradients** | Aucun | Partout (subtils) |

---

## 📊 AVANTAGES DE CETTE VERSION

### 1. Plus Accueillante ✨
- Dashboard Home = warm welcome
- Greetings personnalisés
- Messages encourageants
- Tips rotatifs

### 2. Moins Technique 🎨
- Pas de Nav Rail froid
- Pas d'Inspector Panel séparé
- Flow naturel au lieu de grilles rigides
- Moins de séparateurs stricts

### 3. Plus Vivante 🌈
- Gradients subtils partout
- Track colors vibrantes
- Animations spring (bouncy)
- Glows colorés

### 4. Plus Moderne 2026 🚀
- Border radius généreux (ronds)
- Typography scale invitante
- Spring animations
- Filled icons + emoji

### 5. Plus Claire 🎯
- Sections bien définies mais pas rigid
- Home dashboard = overview rapide
- Sidebar logique
- Flow naturel dans content

### 6. Garde la Sobriété ⚖️
- Pas over-the-top
- Gradients subtils (pas criards)
- Animations purposeful
- Emoji utilisés avec modération

---

## 🚀 PLAN D'IMPLÉMENTATION

### Phase 1: Design System (2 semaines)

**Objectif:** Tokens, composants de base

- [ ] Définir tous les tokens CSS (colors, spacing, radius, shadows)
- [ ] Créer les composants réutilisables (Button, Card, Input)
- [ ] Implémenter les animations keyframes
- [ ] Setup gradients et glows

**Fichiers:**
- `src/styles/tokens.css` (nouveau)
- `src/styles/animations.css` (nouveau)
- `src/components/common/Button.tsx` (nouveau)
- `src/components/common/Card.tsx` (nouveau)
- `src/components/common/Input.tsx` (nouveau)

---

### Phase 2: Layout & Navigation (2 semaines)

**Objectif:** Structure principale

- [ ] Header redesign
- [ ] Sidebar avec sections
- [ ] Main content area
- [ ] Navigation entre sections
- [ ] Routing

**Fichiers:**
- `src/components/Layout/Header.tsx` (refonte)
- `src/components/Layout/Sidebar.tsx` (refonte)
- `src/components/Layout/MainContent.tsx` (refonte)
- `src/App.tsx` (routing updates)

---

### Phase 3: Home Dashboard (1 semaine)

**Objectif:** Page d'accueil accueillante

- [ ] Greeting personnalisé
- [ ] Stats cards
- [ ] Quick actions
- [ ] Recent activity
- [ ] Most played
- [ ] Tips system

**Fichiers:**
- `src/components/Home/` (nouveau dossier)
- `src/components/Home/Dashboard.tsx`
- `src/components/Home/QuickActions.tsx`
- `src/components/Home/RecentActivity.tsx`

---

### Phase 4: Keys Section (2-3 semaines)

**Objectif:** Vue principale optimisée

- [ ] Tracks bar redesign
- [ ] Key grid avec flow
- [ ] Sound details intégrés (pas inspector)
- [ ] Search bar
- [ ] Track color system

**Fichiers:**
- `src/components/Keys/` (refonte)
- `src/components/Tracks/TrackCard.tsx` (nouveau)
- `src/components/Keys/KeyGrid.tsx` (refonte)
- `src/components/Keys/SoundDetailsCard.tsx` (nouveau)

---

### Phase 5: Discovery Section (1-2 semaines)

**Objectif:** Focus immersif

- [ ] Single card carousel
- [ ] Quick add bar
- [ ] Disliked panel
- [ ] Preview player

**Fichiers:**
- `src/components/Discovery/` (refonte)
- `src/components/Discovery/FocusCard.tsx` (nouveau)

---

### Phase 6: Library Section (1-2 semaines)

**Objectif:** Organisation visuelle

- [ ] Profile cards
- [ ] All sounds liste
- [ ] Actions bar

**Fichiers:**
- `src/components/Library/` (nouveau)
- `src/components/Profiles/ProfileCard.tsx` (refonte)

---

### Phase 7: Settings Section (1 semaine)

**Objectif:** Liste simple

- [ ] Grouped settings
- [ ] Theme selector
- [ ] Density selector

**Fichiers:**
- `src/components/Settings/SettingsPage.tsx` (refonte)

---

### Phase 8: Personality & Polish (1 semaine)

**Objectif:** Micro-interactions et personnalité

- [ ] Greetings system
- [ ] Encouraging messages
- [ ] Empty states illustrés
- [ ] Tips rotatifs
- [ ] Toutes les animations spring

---

### Phase 9: Testing & Refinement (1 semaine)

- [ ] Usability testing
- [ ] Bug fixes
- [ ] Performance optimization
- [ ] Accessibility audit
- [ ] Polish animations

---

**Total estimé: 12-15 semaines**

---

## 📝 NOTES FINALES

### Préserve 100% des Fonctionnalités ✅

Aucune fonctionnalité retirée, juste réorganisée et embellie.

### Migration Utilisateur Smooth

- Settings migrés automatiquement
- Profiles inchangés
- "What's New" modal au premier lancement
- Optional guided tour

### Plus Chaleureux, Pas Moins Professionnel

- Sobre mais vivant
- Moderne mais accueillant
- Clean mais pas stérile
- Fun mais pas enfantin

### Évolutivité

Layout permet d'ajouter facilement:
- Nouvelles sections
- Plugins/Extensions
- Features futures

---

## ✅ VALIDATION

Cette version répond aux critères :
- ✅ Moderne et sobre
- ✅ Chaleureuse et accueillante
- ✅ Pas "usine" ou "moteur de jeu"
- ✅ Belle et claire
- ✅ Facile à utiliser
- ✅ Personnalité subtile
- ✅ Respiration et flow naturel
- ✅ Fait 2026

**Prêt pour validation et implémentation ! 🚀**
