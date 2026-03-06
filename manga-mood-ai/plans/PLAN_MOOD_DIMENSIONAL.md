# Plan d'action : Mood dimensionnel (Mood x Intensite)

> **Mise a jour (mars 2026) :** ce document reste valable pour la migration du schema de mood (8 moods x 3 intensites), mais les scores benchmark cites ailleurs dans les anciens plans ont ete depasses. Pour les references actuelles, voir `manga-mood-ai/research/RESEARCH_SYNTHESIS.md`.

## Objectif

Remplacer les 10 moods plats (`epic_battle`, `tension`, `sadness`, `comedy`, `romance`, `horror`, `peaceful`, `emotional_climax`, `mystery`, `chase_action`) par un systeme a 2 axes :

- **Axe 1 — Mood** (8 valeurs) : `epic`, `tension`, `sadness`, `comedy`, `romance`, `horror`, `peaceful`, `mystery`
- **Axe 2 — Intensite** (3 niveaux) : `1` (calme/ambient), `2` (modere), `3` (peak/climax)

Les anciennes categories problematiques disparaissent :
- `emotional_climax` → n'importe quel mood a intensite 3
- `chase_action` → `tension` intensite 3 ou `epic` intensite 2
- `epic_battle` → `epic` intensite 2-3

L'utilisateur tague ses OST avec mood + intensite. Le VLM classifie sur 2 axes independants (ce qu'il fait bien). Le MoodDirector lisse les deux axes.

---

## Fichiers impactes

### Backend Rust (src-tauri/src/)

| Fichier | Changements |
|---------|------------|
| `types.rs` | Remplacer `MoodCategory` enum (10 → 8), ajouter `MoodIntensity` enum (1-3), ajouter `MoodTag` struct, migrer `KeyBinding.mood` |
| `mood/inference.rs` | Nouveau prompt VLM (mood + intensite), nouveau parsing, adapter `analyze_mood()` et `analyze_mood_scored()` |
| `mood/director.rs` | Adapter `MoodScores` (10 → 8), transition matrix (10x10 → 8x8), logique de lissage d'intensite, `DirectorDecision` avec intensite |
| `mood/server.rs` | Adapter endpoints API (`/api/analyze`, `/api/moods`, `/api/trigger`, etc.), payloads JSON |
| `mood/cache.rs` | `CachedMoodEntry` avec intensite |
| `commands.rs` | Adapter `analyze_mood` command et events |
| `state.rs` | Aucun changement (les types sont dans director/types) |

### Frontend React (src/)

| Fichier | Changements |
|---------|------------|
| `types/index.ts` | Remplacer `MoodCategory` type, ajouter `MoodIntensity`, `MoodTag`, adapter `KeyBinding` |
| `utils/moodHelpers.ts` | Nouvelles constantes : `BASE_MOODS`, `INTENSITY_LEVELS`, `MOOD_DISPLAY`, `MOOD_COLORS` |
| `stores/moodStore.ts` | `lastDetectedMood` et `committedMood` deviennent `MoodTag` (mood + intensite) |
| `hooks/useMoodPlayback.ts` | Matching mood+intensite avec les bindings, logique de range |
| `components/Sounds/SoundDetails.tsx` | Double dropdown (mood + intensite) ou dropdown combine |
| `components/Keys/KeyGrid.tsx` | Badge avec mood + intensite |
| `components/common/SearchFilterBar.tsx` | Filtre `m:` adapte (ex: `m:sadness`, `m:sadness:3`) |
| `components/Layout/MainContent.tsx` | Filtre mood adapte |
| `components/Layout/Sidebar.tsx` | `MoodIndicator` affiche mood + intensite |
| `components/Settings/SettingsModal.tsx` | Aucun changement (toggle/port/thresholds restent les memes) |
| `stores/settingsStore.ts` | Aucun changement |

---

## Etape 1 : Types Rust (types.rs)

### 1.1 Nouveau enum `BaseMood`

Remplacer `MoodCategory` par `BaseMood` :

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaseMood {
    Epic,
    Tension,
    Sadness,
    Comedy,
    Romance,
    Horror,
    Peaceful,
    Mystery,
}
```

Avec les memes helpers que l'ancien `MoodCategory` :
- `BaseMood::ALL` — tableau constant des 8 valeurs
- `index()` → 0..7
- `from_index(i)` avec fallback `Peaceful`
- `as_str()` → `"epic"`, `"tension"`, etc.
- `from_str_opt(s)` → `Option<BaseMood>`

### 1.2 Enum `MoodIntensity`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MoodIntensity {
    Low = 1,    // Calm, ambient, background
    Medium = 2, // Moderate, standard
    High = 3,   // Peak, climax, intense
}
```

Avec :
- `as_u8()` → 1, 2, 3
- `from_u8(n)` → clampe a 1-3
- `as_str()` → `"low"`, `"medium"`, `"high"`

### 1.3 Struct `MoodTag`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MoodTag {
    pub mood: BaseMood,
    pub intensity: MoodIntensity,
}
```

### 1.4 Adapter `KeyBinding`

```rust
pub struct KeyBinding {
    // ... inchange ...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mood: Option<BaseMood>,           // Le mood de base
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mood_intensity: Option<MoodIntensity>,  // L'intensite minimale pour trigger
}
```

**Design du matching** : L'utilisateur tague un binding avec `mood=sadness, intensity=2`. Ce binding se declenche quand le VLM detecte `sadness` avec intensite >= 2. Ca permet :
- `sadness` sans intensite = trigger sur tout sadness (1, 2, 3)
- `sadness` intensite 2 = trigger sur sadness 2 et 3 (pas 1)
- `sadness` intensite 3 = trigger uniquement sur sadness climax

### 1.5 Migration de l'ancien format

Pour la retrocompatibilite des profils existants, ajouter un `serde` deserialize custom ou une migration dans `load_profile()`. Mapping :

| Ancien mood | Nouveau mood | Intensite |
|-------------|-------------|-----------|
| `epic_battle` | `epic` | `High` (3) |
| `tension` | `tension` | `Medium` (2) |
| `sadness` | `sadness` | `Medium` (2) |
| `comedy` | `comedy` | `Medium` (2) |
| `romance` | `romance` | `Medium` (2) |
| `horror` | `horror` | `Medium` (2) |
| `peaceful` | `peaceful` | `Low` (1) |
| `emotional_climax` | aucun — supprimer le tag | — |
| `mystery` | `mystery` | `Medium` (2) |
| `chase_action` | `tension` | `High` (3) |

Implementation : dans le `Deserialize` de `KeyBinding`, si le champ `mood` contient une ancienne valeur (`epic_battle`, `emotional_climax`, `chase_action`), la convertir automatiquement. Utiliser `#[serde(deserialize_with = "...")]` ou un `impl Deserialize` custom.

**Alternative plus simple** : un `serde(alias)` ne suffit pas ici car on change la structure. Faire une migration dans `storage/profile.rs` au chargement : scanner les bindings, remapper les vieilles valeurs.

---

## Etape 2 : Prompt VLM et Parsing (inference.rs)

### 2.1 Nouveau prompt

Remplacer `GUIDED_V3_PROMPT` par :

```rust
pub(crate) const MOOD_INTENSITY_PROMPT: &str = "\
What is the mood and intensity of this manga page?

Mood (pick ONE):
- epic: heroic moments, battles, triumph, power display
- tension: conflict, confrontation, suspense, buildup
- sadness: grief, loss, tears, regret, melancholy
- comedy: humor, funny situations, lighthearted
- romance: love, affection, tender connection
- horror: fear, dread, disturbing atmosphere
- peaceful: calm daily life, relaxation, serene
- mystery: unknown, intrigue, investigation

Intensity (pick ONE):
- 1: calm, subtle, background atmosphere
- 2: moderate, clear emotion, standard scene
- 3: peak, climax, overwhelming emotion, maximum impact

Reply format: mood intensity
Example: sadness 3";
```

Avantages :
- 8 choix au lieu de 10 → moins de confusion
- L'intensite est un jugement visuel simple (les VLM sont bons pour ca)
- Prompt court → inference rapide
- Pas de confusion sadness/emotional_climax → c'est juste sadness 2 vs sadness 3

### 2.2 Nouveau parsing

Nouvelle fonction `parse_mood_intensity_response()` :

```rust
pub(crate) fn parse_mood_intensity_response(json: &serde_json::Value) -> Result<MoodTag, String> {
    // 1. Extraire le texte de la reponse (meme logique qu'avant)
    // 2. Supprimer les <think>...</think> tags
    // 3. Chercher le pattern "mood intensity" dans le texte restant
    //    - Regex: r"(?i)(epic|tension|sadness|comedy|romance|horror|peaceful|mystery)\s+([123])"
    //    - Prendre le DERNIER match (le VLM met souvent sa reponse a la fin)
    // 4. Construire MoodTag { mood: BaseMood::from_str_opt(m), intensity: MoodIntensity::from_u8(n) }
    // 5. Fallback : si seul le mood est trouve sans intensite → intensite 2 (medium)
    // 6. Fallback : si rien trouve → peaceful 1
}
```

### 2.3 Adapter `analyze_mood()`

```rust
pub async fn analyze_mood(&self, image_base64: &str) -> Result<MoodTag, String> {
    // Meme logique qu'avant, mais utilise MOOD_INTENSITY_PROMPT
    // et parse avec parse_mood_intensity_response()
}
```

### 2.4 Adapter `analyze_mood_scored()`

Le format scored change aussi. L'intensite est un champ separe :

```rust
pub async fn analyze_mood_scored(&self, image_base64: &str, context: Option<&NarrativeContext>) -> Result<(MoodScores, MoodIntensity, NarrativeRole), String> {
    // Le prompt scored demande :
    // SCORES: epic=X.XX tension=X.XX ... (8 moods)
    // INTENSITY: 1|2|3
    // ROLE: continuation|escalation|...
}
```

`MoodScores` passe de `[f32; 10]` a `[f32; 8]`.

### 2.5 Supprimer le code mort

- `GUIDED_V3_PROMPT` → supprimer
- `emotion_to_mood_fallback()` → supprimer (plus de mapping emotions → moods)
- `extract_features_manga()` → supprimer (plus de feature extraction)
- `parse_hybrid_response()` → supprimer (plus de hybrid)
- `correct_moods_batch()` → supprimer (V5 abandonne)
- `describe_page()` → supprimer (plus utilise en production)
- `default_mood_categories()` → adapter aux 8 moods
- `refine_moods_from_labels()` → supprimer (abandonne)
- `analyze_mood_scored()` version avec scored prompt complexe → simplifier

**Attention** : garder la logique de base qui fonctionne (HTTP call, image prepare, response extraction). Ne supprimer que les prompts/parsers des approches echouees.

---

## Etape 3 : MoodDirector (director.rs)

### 3.1 `MoodScores` → 8 moods

```rust
pub struct MoodScores {
    pub scores: [f32; 8],  // Indexe par BaseMood::index()
}
```

Adapter tous les helpers (`get`, `set`, `dominant`, `from_single`).

### 3.2 Ajouter l'intensite au pipeline

```rust
pub struct PageAnalysis {
    pub scores: MoodScores,
    pub intensity: MoodIntensity,       // NOUVEAU
    pub narrative_role: NarrativeRole,
    pub dominant_mood: BaseMood,        // Renomme de MoodCategory
}

pub struct DirectorDecision {
    pub raw_mood: BaseMood,
    pub raw_intensity: MoodIntensity,   // NOUVEAU
    pub committed_mood: BaseMood,       // Renomme
    pub committed_intensity: MoodIntensity, // NOUVEAU
    pub mood_changed: bool,
    pub intensity_changed: bool,        // NOUVEAU — changement d'intensite sans changement de mood
    pub raw_scores: MoodScores,
    pub narrative_role: NarrativeRole,
    pub window_scores: MoodScores,
    pub dwell_count: u32,
}
```

### 3.3 Transition matrix 8x8

```rust
const TRANSITION_MATRIX: [[f32; 8]; 8] = [
    //  EPI   TEN   SAD   COM   ROM   HOR   PEA   MYS
    [1.0, 0.7, 0.5, 0.2, 0.2, 0.4, 0.3, 0.3], // epic →
    [0.9, 1.0, 0.5, 0.3, 0.3, 0.7, 0.4, 0.8], // tension →
    [0.3, 0.5, 1.0, 0.2, 0.6, 0.3, 0.7, 0.4], // sadness →
    [0.3, 0.3, 0.3, 1.0, 0.7, 0.2, 0.8, 0.3], // comedy →
    [0.2, 0.3, 0.6, 0.7, 1.0, 0.2, 0.8, 0.3], // romance →
    [0.4, 0.8, 0.5, 0.1, 0.1, 1.0, 0.3, 0.7], // horror →
    [0.4, 0.4, 0.6, 0.8, 0.7, 0.3, 1.0, 0.5], // peaceful →
    [0.4, 0.8, 0.4, 0.3, 0.3, 0.7, 0.4, 1.0], // mystery →
];
```

### 3.4 Lissage de l'intensite

L'intensite est lissee separement du mood :

```rust
// Dans MoodDirector::process()
// L'intensite est une moyenne glissante sur la fenetre (pas un vote)
let avg_intensity = self.window.iter()
    .zip(weights.iter())
    .map(|(page, &w)| page.intensity.as_u8() as f32 * w)
    .sum::<f32>() / weights.iter().sum::<f32>();
let committed_intensity = MoodIntensity::from_f32(avg_intensity); // round to nearest 1/2/3
```

Un changement d'intensite (sans changement de mood) emet un event different — ca permet de passer d'une OST sadness calme a une OST sadness intense sans "couper" la musique (crossfade au sein du meme mood).

### 3.5 Supprimer le code de fusion

- `fuse_mood()` → supprimer (plus necessaire, la confusion sadness/emotional_climax n'existe plus)
- `rule_anti_ec_sadness_arc()` → supprimer

---

## Etape 4 : Server HTTP (server.rs)

### 4.1 Adapter `POST /api/analyze`

Response :
```json
{
    "mood": "sadness",
    "intensity": 3,
    "status": "ok",
    "committed_mood": "sadness",
    "committed_intensity": 2,
    "mood_changed": true,
    "intensity_changed": false,
    "scores": { "epic": 0.05, "tension": 0.10, "sadness": 0.72, ... },
    "narrative_role": "continuation",
    "dwell_count": 4
}
```

### 4.2 Adapter `GET /api/moods`

```json
{
    "moods": ["epic", "tension", "sadness", "comedy", "romance", "horror", "peaceful", "mystery"],
    "intensities": [1, 2, 3],
    "intensity_labels": { "1": "low", "2": "medium", "3": "high" }
}
```

### 4.3 Adapter `POST /api/trigger`

Ajouter `intensity` au payload :
```json
{ "mood": "sadness", "intensity": 2 }
```

Valider contre les 8 moods (pas 10).

### 4.4 Supprimer les endpoints V2

- `POST /api/analyze-v2` → supprimer (pipeline V5 abandonne)
- `POST /api/extract` → supprimer (feature extraction abandonnee)
- `POST /api/classify-batch` → supprimer (batch classify abandonne)

### 4.5 `DescriptionBuffer` et `MoodApiState`

Adapter les types internes pour utiliser `BaseMood` + `MoodIntensity` au lieu de `MoodCategory`.

---

## Etape 5 : Events Tauri (commands.rs)

### 5.1 Event `mood_detected`

```json
{
    "mood": "sadness",
    "intensity": 3,
    "source": "api"
}
```

### 5.2 Event `mood_committed`

```json
{
    "mood": "sadness",
    "intensity": 2,
    "source": "api",
    "previous_mood": "tension",
    "previous_intensity": 2,
    "dwell_count": 1,
    "intensity_changed": false
}
```

### 5.3 Nouvel event `mood_intensity_changed` (optionnel)

Si le mood reste le meme mais l'intensite change, emettre un event separe. Le frontend peut alors crossfader vers une OST du meme mood mais d'intensite differente.

```json
{
    "mood": "sadness",
    "old_intensity": 1,
    "new_intensity": 3,
    "source": "api"
}
```

**Decision a prendre** : est-ce qu'on veut un event separe, ou est-ce qu'on utilise `mood_committed` avec `mood_changed: false, intensity_changed: true` ? La deuxieme option est plus simple cote frontend.

→ **Recommandation** : utiliser `mood_committed` avec les deux flags. Le frontend gere les deux cas dans le meme listener.

---

## Etape 6 : Cache (cache.rs)

### 6.1 `CachedMoodEntry`

```rust
pub struct CachedMoodEntry {
    pub mood: BaseMood,
    pub intensity: MoodIntensity,
    pub scores: MoodScores,          // [f32; 8]
    pub narrative_role: NarrativeRole,
}
```

---

## Etape 7 : Frontend Types (types/index.ts)

### 7.1 Nouveaux types

```typescript
export type BaseMood =
  | "epic"
  | "tension"
  | "sadness"
  | "comedy"
  | "romance"
  | "horror"
  | "peaceful"
  | "mystery";

export type MoodIntensity = 1 | 2 | 3;

export interface MoodTag {
  mood: BaseMood;
  intensity: MoodIntensity;
}
```

### 7.2 Adapter `KeyBinding`

```typescript
export interface KeyBinding {
  keyCode: KeyCode;
  trackId: TrackId;
  soundIds: SoundId[];
  loopMode: LoopMode;
  currentIndex: number;
  name?: string;
  mood?: BaseMood;              // Tag mood (was: MoodCategory)
  moodIntensity?: MoodIntensity; // Intensite minimum pour trigger (NEW)
}
```

### 7.3 Adapter `KeyGridFilter`

```typescript
export interface KeyGridFilter {
  searchText: string;
  trackName: string | null;
  loopMode: LoopMode | null;
  status: "playing" | "stopped" | null;
  mood: BaseMood | null;               // Was: MoodCategory
  intensity: MoodIntensity | null;     // NEW
}
```

### 7.4 `AppConfig` — pas de changement

Les champs mood dans AppConfig (`moodAiEnabled`, `moodApiPort`, `moodEntryThreshold`, etc.) ne changent pas.

---

## Etape 8 : Frontend Helpers (moodHelpers.ts)

### 8.1 Nouvelles constantes

```typescript
export const BASE_MOODS: BaseMood[] = [
  "epic", "tension", "sadness", "comedy",
  "romance", "horror", "peaceful", "mystery",
];

export const MOOD_DISPLAY: Record<BaseMood, string> = {
  epic: "Epic",
  tension: "Tension",
  sadness: "Sadness",
  comedy: "Comedy",
  romance: "Romance",
  horror: "Horror",
  peaceful: "Peaceful",
  mystery: "Mystery",
};

export const MOOD_COLORS: Record<BaseMood, { bg: string; text: string }> = {
  epic: { bg: "bg-red-500/20", text: "text-red-400" },
  tension: { bg: "bg-amber-500/20", text: "text-amber-400" },
  sadness: { bg: "bg-blue-500/20", text: "text-blue-400" },
  comedy: { bg: "bg-yellow-500/20", text: "text-yellow-400" },
  romance: { bg: "bg-pink-500/20", text: "text-pink-400" },
  horror: { bg: "bg-purple-500/20", text: "text-purple-400" },
  peaceful: { bg: "bg-green-500/20", text: "text-green-400" },
  mystery: { bg: "bg-indigo-500/20", text: "text-indigo-400" },
};

export const INTENSITY_DISPLAY: Record<MoodIntensity, string> = {
  1: "Calm",
  2: "Moderate",
  3: "Intense",
};

export const INTENSITY_COLORS: Record<MoodIntensity, string> = {
  1: "opacity-50",    // Pill plus pale
  2: "",              // Normal
  3: "ring-1 ring-current font-bold", // Pill avec bordure, bold
};
```

---

## Etape 9 : Frontend Store (moodStore.ts)

### 9.1 Adapter le state

```typescript
interface MoodState {
  serverStatus: "stopped" | "starting" | "running" | "error";
  serverInstalled: boolean;
  modelInstalled: boolean;
  modelDownloadProgress: { downloaded: number; total: number } | null;
  lastDetectedMood: BaseMood | null;         // Was: MoodCategory
  lastDetectedIntensity: MoodIntensity | null; // NEW
  committedMood: BaseMood | null;
  committedIntensity: MoodIntensity | null;    // NEW
  isAnalyzing: boolean;
}
```

---

## Etape 10 : Frontend Playback (useMoodPlayback.ts)

### 10.1 Matching mood + intensite

Quand `mood_committed` arrive avec `{ mood: "sadness", intensity: 3 }` :

```typescript
// Trouver les bindings qui matchent
const matchingBindings = allBindings.filter(b => {
  if (b.mood !== detectedMood) return false;
  // Si le binding a une intensite minimum, verifier
  if (b.moodIntensity && detectedIntensity < b.moodIntensity) return false;
  return true;
});
```

**Exemple** : VLM detecte `sadness 3`
- Binding A : `mood=sadness` (pas d'intensite) → MATCH (accepte tout)
- Binding B : `mood=sadness, intensity=2` → MATCH (3 >= 2)
- Binding C : `mood=sadness, intensity=3` → MATCH (3 >= 3)
- Binding D : `mood=tension` → PAS MATCH

Si plusieurs bindings matchent, la logique existante s'applique (un par track, loop mode, etc.).

### 10.2 Changement d'intensite sans changement de mood

Quand le mood reste `sadness` mais l'intensite passe de 1 → 3, le frontend doit :
1. Chercher des bindings avec `mood=sadness, intensity=3` (qui ne matchaient pas avant)
2. Si trouves, crossfader vers ces OST
3. Sinon, ne rien faire (garder l'OST actuelle)

Gere via `mood_committed` avec `mood_changed: false, intensity_changed: true`.

---

## Etape 11 : Frontend UI

### 11.1 SoundDetails.tsx — Dropdown mood + intensite

Remplacer le single dropdown par deux controles :

```tsx
{/* Mood selector */}
<select value={binding.mood ?? ""} onChange={handleMoodChange}>
  <option value="">None</option>
  {BASE_MOODS.map(m => (
    <option key={m} value={m}>{MOOD_DISPLAY[m]}</option>
  ))}
</select>

{/* Intensity selector (visible seulement si mood selectionne) */}
{binding.mood && (
  <select value={binding.moodIntensity ?? ""} onChange={handleIntensityChange}>
    <option value="">Any</option>
    <option value="1">1 - Calm</option>
    <option value="2">2 - Moderate</option>
    <option value="3">3 - Intense</option>
  </select>
)}

{/* Badge */}
{binding.mood && (
  <span className={`... ${MOOD_COLORS[binding.mood].bg} ${MOOD_COLORS[binding.mood].text} ${binding.moodIntensity ? INTENSITY_COLORS[binding.moodIntensity] : ''}`}>
    {MOOD_DISPLAY[binding.mood]}
    {binding.moodIntensity && ` ${binding.moodIntensity}`}
  </span>
)}
```

### 11.2 KeyGrid.tsx — Badge adapte

```tsx
{group.mood && (
  <span className={`text-[9px] px-1 py-px rounded-full ${MOOD_COLORS[group.mood].bg} ${MOOD_COLORS[group.mood].text}`}>
    {MOOD_DISPLAY[group.mood]}
    {group.moodIntensity && ` ${group.moodIntensity}`}
  </span>
)}
```

### 11.3 SearchFilterBar.tsx — Filtre adapte

Syntaxe : `m:sadness` (tout intensite), `m:sadness:3` (intensite specifique).

```typescript
} else if (lower.startsWith("m:") && token.length > 2) {
  const parts = token.slice(2).split(":");
  const mood = parts[0].toLowerCase();
  const intensity = parts[1] ? parseInt(parts[1]) : null;
  if ((BASE_MOODS as readonly string[]).includes(mood)) {
    newChips.push({
      type: "mood",
      value: intensity ? `${mood}:${intensity}` : mood,
      label: `m:${mood}${intensity ? `:${intensity}` : ""}`,
    });
  }
}
```

### 11.4 Sidebar.tsx — MoodIndicator

```tsx
{committedMood && (
  <div className="flex items-center gap-1.5">
    <span className="text-text-muted text-xs">Playing:</span>
    <span className={`text-xs px-1.5 py-0.5 rounded-full ${colors.bg} ${colors.text}`}>
      {MOOD_DISPLAY[committedMood]}
    </span>
    {committedIntensity && (
      <span className="text-text-muted text-xs">
        lv.{committedIntensity}
      </span>
    )}
  </div>
)}
```

---

## Etape 12 : Migration des profils existants

### 12.1 Dans `storage/profile.rs` au chargement

Apres deserialization du profil, scanner les bindings :

```rust
fn migrate_mood_tags(profile: &mut Profile) {
    for binding in &mut profile.bindings {
        if let Some(ref old_mood) = binding.mood_legacy {
            let (new_mood, intensity) = match old_mood.as_str() {
                "epic_battle" => (Some("epic"), Some(MoodIntensity::High)),
                "chase_action" => (Some("tension"), Some(MoodIntensity::High)),
                "emotional_climax" => (None, None), // Supprimer — trop ambigu
                "tension" => (Some("tension"), None),
                "sadness" => (Some("sadness"), None),
                "comedy" => (Some("comedy"), None),
                "romance" => (Some("romance"), None),
                "horror" => (Some("horror"), None),
                "peaceful" => (Some("peaceful"), None),
                "mystery" => (Some("mystery"), None),
                other => (BaseMood::from_str_opt(other).map(|m| m.as_str()), None),
            };
            binding.mood = new_mood.and_then(BaseMood::from_str_opt);
            binding.mood_intensity = intensity;
        }
    }
}
```

**Alternative plus propre** : utiliser `#[serde(deserialize_with = "...")]` sur le champ `mood` de `KeyBinding` pour accepter les anciennes valeurs et les convertir automatiquement. Ca evite de devoir sauvegarder le profil apres migration.

### 12.2 Strategy de migration serde

Le champ `mood` dans le JSON est un string (`"epic_battle"`, `"sadness"`, etc.). On peut faire un deserialize custom qui :
1. Tente de parser comme `BaseMood` (nouveau format)
2. Si echec, tente de mapper depuis l'ancien format
3. Si echec, retourne `None`

```rust
fn deserialize_mood<'de, D>(deserializer: D) -> Result<Option<BaseMood>, D::Error>
where D: serde::Deserializer<'de> {
    let opt: Option<String> = Option::deserialize(deserializer)?;
    Ok(opt.and_then(|s| {
        BaseMood::from_str_opt(&s).or_else(|| match s.as_str() {
            "epic_battle" => Some(BaseMood::Epic),
            "chase_action" => Some(BaseMood::Tension),
            "emotional_climax" => None,  // drop
            _ => None,
        })
    }))
}
```

---

## Etape 13 : Benchmark test (director.rs)

### 13.1 Adapter le ground truth

Le ground truth des 31 pages Blue Lock doit etre re-annote avec le nouveau format :

| Ancien | Nouveau |
|--------|---------|
| `tension` | `tension 2` |
| `epic_battle` | `epic 3` |
| `sadness` | `sadness 2` |
| `emotional_climax` | `sadness 3` ou `tension 3` (selon la page) |
| `peaceful` | `peaceful 1` |
| `mystery` | `mystery 2` |
| `chase_action` | `tension 3` |

### 13.2 Nouveau scoring du benchmark

Deux metriques :
- **Mood accuracy** : est-ce que le mood de base est correct ? (8 choix au lieu de 10 → devrait etre plus eleve)
- **Full accuracy** : est-ce que mood + intensite sont corrects ? (plus strict mais plus informatif)
- **Relaxed accuracy** : mood correct OU intensite a ±1 du ground truth

### 13.3 Test command (inchange)

```bash
cargo test --manifest-path src-tauri/Cargo.toml bluelock_sequence -- --ignored --nocapture
```

---

## Etape 14 : Nettoyage du code mort

Supprimer tout le code des approches echouees :

| Code a supprimer | Fichier |
|-----------------|---------|
| `GUIDED_V3_PROMPT` | inference.rs |
| `analyze_mood_scored()` version complexe avec scored prompt | inference.rs |
| `extract_features_manga()` | inference.rs |
| `parse_hybrid_response()` | inference.rs |
| `emotion_to_mood_fallback()` | inference.rs |
| `correct_moods_batch()` | inference.rs |
| `describe_page()` | inference.rs |
| `refine_moods_from_labels()` | inference.rs |
| `NarrativeContext` (si on decide de ne plus l'utiliser) | inference.rs |
| `HybridResult`, `PageFeatures` | inference.rs |
| `fuse_mood()`, `rule_anti_ec_sadness_arc()` | director.rs |
| `POST /api/analyze-v2` | server.rs |
| `POST /api/extract` | server.rs |
| `POST /api/classify-batch` | server.rs |
| `DescriptionBuffer` (si plus utilise) | server.rs |

---

## Ordre d'implementation recommande

### Phase 1 — Types et backend (sans casser le frontend)

1. **types.rs** : ajouter `BaseMood`, `MoodIntensity`, `MoodTag`. Garder `MoodCategory` temporairement comme alias.
2. **inference.rs** : nouveau prompt + parsing. Adapter `analyze_mood()` pour retourner `MoodTag`.
3. **director.rs** : adapter `MoodScores` (8), transition matrix (8x8), `PageAnalysis`/`DirectorDecision` avec intensite.
4. **cache.rs** : adapter `CachedMoodEntry`.
5. **server.rs** : adapter les endpoints.
6. **commands.rs** : adapter les events.

### Phase 2 — Frontend

7. **types/index.ts** : nouveaux types.
8. **moodHelpers.ts** : nouvelles constantes.
9. **moodStore.ts** : adapter le state.
10. **useMoodPlayback.ts** : matching avec intensite.
11. **SoundDetails.tsx** : double dropdown.
12. **KeyGrid.tsx** : badge adapte.
13. **SearchFilterBar.tsx** + **MainContent.tsx** : filtre adapte.
14. **Sidebar.tsx** : MoodIndicator adapte.

### Phase 3 — Migration et cleanup

15. **storage/profile.rs** ou serde custom : migration des anciens profils.
16. Supprimer `MoodCategory` et tout le code mort.
17. Adapter le benchmark test.

### Phase 4 — Test

18. Lancer le benchmark `bluelock_sequence` avec le nouveau prompt.
19. Verifier que le frontend compile et fonctionne.
20. Tester la migration d'un profil avec des anciens mood tags.

---

## Decisions ouvertes (a confirmer)

1. **Intensite par defaut quand pas specifiee par l'utilisateur** : le binding match toutes les intensites (`None` = any). Confirme ?
2. **Changement d'intensite sans changement de mood** : doit-il trigger un crossfade vers une OST differente du meme mood ? Ou juste ignorer ?
3. **NarrativeContext** : on le garde pour le prompt scored, ou on le supprime ? (Il a ete teste et cause des feedback loops, mais avec le nouveau prompt plus simple ca pourrait etre different.)
4. **Nombre de base moods** : 8 est-il bon, ou faut-il en ajouter/enlever ? Par exemple, `action` pourrait etre un mood separe de `epic` et `tension`.
