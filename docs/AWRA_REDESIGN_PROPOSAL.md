# KeyToMusic × Awra — proposition de refonte

Statut : proposition, à valider avant toute implémentation.
Cible : nouvelle app native **KeyToMusicAwra** (`/home/mehdi/Dev/KeyToMusicAwra`), Rust + GTK 4.22 + crate `awra`, sans CSS locale (même règle que PokediaAwra). L'app Tauri reste en place pour Windows/macOS.

---

## 1. Architecture

```
KeyToMusicRustTauri/                 (workspace Cargo)
├── crates/keytomusic-core/          ← NOUVEAU : tout le métier, sans Tauri ni GTK
│   ├── audio/ keys/ youtube/ discovery/ mood/ storage/ import_export/ types
│   ├── events.rs     CoreEvent + trait EventSink (remplace app.emit)
│   ├── service.rs    logique extraite de commands.rs (mood lifecycle, preload, discovery, setup evdev…)
│   ├── playback.rs   portée depuis le TS : résolution touche→bindings, cooldown, loop modes, auto-replay, mood playback
│   ├── editor.rs     portée depuis le TS : mutations de profil, undo/redo (50), file de sauvegarde
│   └── discovery_session.rs  portée depuis le TS : pool/révélés/visités, pré-téléchargement, auto-assignation
├── src-tauri/                       adaptateur mince : #[tauri::command] = 1 ligne, TauriSink → mêmes events JSON
└── src/                             React inchangé

KeyToMusicAwra/                      ← NOUVEAU dépôt
└── dépend de keytomusic-core (path) + awra (path ../Awra/bindings/rust/awra)
```

Points clés :
- **Le backend est déjà presque libre de Tauri** : seuls `main.rs`, `commands.rs` et `keys/layout.rs` y touchent. Tout le reste passe déjà par des callbacks/mpsc.
- **~900 lignes de logique de lecture vivent aujourd'hui dans React** (`useKeyDetection`, `useLoopPlayback`, `useMoodPlayback`…). Elles passent en Rust dans le core : l'app native en a besoin, et elle devient testable unitairement.
- Runtime : un tokio multi-thread possédé par le core ; la boucle glib reste sur le thread principal ; les events arrivent via `async_channel` → `glib::spawn_future_local`.
- `keys/layout.rs` (gdk GTK3) est réécrit en gdk4 côté app native ; `rfd` remplacé par `gtk::FileDialog`.
- L'app Tauri doit continuer à compiler et se comporter pareil après l'extraction (vérifié par `cargo test` + `npm test` + lancement).

---

## 2. Concept d'interface : « le clavier est l'interface »

Aujourd'hui l'app est une suite de cartes, de modales et de panneaux empilés. Le geste central de KeyToMusic, c'est **une touche → un son**. La refonte met donc **un vrai clavier** au centre, sur la matière givrée d'Awra, et tout le reste gravite autour.

```
┌─ Header (Chrome) ──────────────────────────────────────────────────────────────┐
│ [Profil ▾]   [⌘K Rechercher une touche, un son, une action…]   ● Écoute  🔊━━  ■ Stop │
├─ Sidebar ─┬─ Contenu ──────────────────────────────────────────┬─ Inspecteur ───┤
│ ⌨ Clavier │  Filtre : [t:OST] [m:epic] ……              12 / 34   │ Touche  A      │
│ ✦ Découvrir│  ┌──┬──┬──┬──┬──┬──┬──┬──┬──┬──┐                   │ OST · Ambiance │
│ ◐ Mood AI │  │A●│Z │E●│R │T │Y●│U │I │O │P │   ← vrai clavier  │ ─────────────  │
│ ♫ Biblioth.│  ├──┼──┼──┼──┼──┼──┼──┼──┼──┼──┤     (AZERTY réel)│ ▁▃▇█▆▃▂ ▲0:12  │
│ ⚙ Réglages │  │Q │S●│D │F │G │H │J │K●│L │M │   couleur = piste │ Boucle [Off|1|⟳|→]│
│           │  └──┴──┴──┴──┴──┴──┴──┴──┴──┴──┘   liseré = mood    │ Mood ●●●●●●●●  │
│           │  Combos : [Ctrl+A] [Shift+Num4] …                    │ + Ajouter un son│
├───────────┴──────────────────────────────────────────────────────┴────────────────┤
│ Pistes (dock) : OST ▶ Sad Theme ━━━━●──── 1:02/3:40 ■ 🔊━   │ Ambiance ▶ Rain … ■ │ + │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### Les zones

| Zone | Contenu | Remplace |
|---|---|---|
| **Header** | Sélecteur de profil (dropdown + créer/dupliquer/renommer/supprimer/importer/exporter), barre de commande centrale, interrupteur **Écoute** (détection), Auto-Momentum, volume maître, **Stop** avec compteur de pistes actives, Annuler/Rétablir, menu. | Header + Sidebar profils + GlobalToggles + StopAll |
| **Clavier** (page par défaut) | Clavier physique dessiné selon la disposition réelle (AZERTY…). Touche libre = discrète ; touche liée = teintée par piste, liseré de mood, badge multi-pistes ; touche en lecture = pulsation + mini-égaliseur ; touche pressée = enfoncement animé. Rangée « Combos » pour les accords et touches hors clavier. Filtre `t: l: s: m:` conservé et terminé (intensité incluse). Bascule clavier ↔ liste dense. | KeyGrid + SearchFilterBar |
| **Inspecteur** (panneau de droite, se ferme) | Touche sélectionnée : pistes en onglets, nom, changer de touche, liste des sons avec **une seule** waveform interactive (momentum glissable + valeur précise au clavier, suggestion en un clic), volume, preview, déplacer/retirer ; boucle en contrôle segmenté ; mood en 8 pastilles + intensité. Multi-sélection → édition groupée (piste, boucle, **mood**, suppression). | SoundDetails + MultiKeyDetails |
| **Dock Pistes** (bas, toujours visible) | Une rangée par piste : nom, volume, ce qui joue, seek, stop, ajout/renommage/suppression. | TrackView + NowPlaying |
| **Découvrir** (page entière) | Grandes cartes avec waveform, preview, momentum ; assignation en **cliquant une touche sur un mini-clavier** ou en la pressant ; ajouter / passer / ne plus proposer ; liste des « non merci » consultable et annulable sur place. | DiscoveryPanel coincé dans la sidebar de 224 px + panneau Dislikes des réglages |
| **Mood AI** (page entière) | Assistant d'installation en 3 étapes (serveur → modèle → démarrer) avec progression ; état runtime/API/extension ; **tableau des 8 moods** : chaque mood montre ses touches, on y glisse des touches pour les taguer en masse ; mood courant mis en avant. | Section enfouie dans les réglages + MoodIndicator |
| **Bibliothèque** | Tous les sons du profil (table triable : nom, source, durée, touches, fichier manquant) ; fichiers manquants à résoudre ici (Localiser / Re-télécharger / Retirer) ; import/export `.ktm`, import legacy. | FileNotFoundModal + section Data |
| **Réglages** (page) | Raccourcis, Détection (évdev Linux, cooldown, fenêtre d'accord, modificateur momentum), Audio (périphérique, crossfade), Apparence (accent), À propos. Formulaires `AwraForm`. | SettingsModal unique qui défile |

### Ajouter des sons : un seul geste

- **Glisser des fichiers directement sur une touche** du clavier : ils y sont liés (piste = piste active du dock). Les glisser ailleurs : panneau d'ajout.
- **Panneau d'ajout** (panneau latéral flottant, pas une modale empilée) : un seul champ qui comprend tout, recherche YouTube, URL de vidéo ou URL de playlist, plus une zone de dépôt de fichiers. Résultats avec preview, file d'attente avec waveforms et momentum, puis « appuie sur les touches » pour répartir. Tout en une étape annulable.

### Simplifications de flux

1. **Premier lancement** : profil « Mon profil » + piste « Principale » créés automatiquement. On arrive directement sur le clavier vide avec « Glisse des sons sur une touche ou cherche sur YouTube ». (Aujourd'hui : 3 écrans vides successifs.)
2. **Annuler plutôt que confirmer** : les actions annulables (retirer un son, supprimer une touche, une piste) ne demandent plus confirmation ; un toast « Supprimé · Annuler » suffit. Confirmation gardée seulement pour l'irréversible (supprimer un profil).
3. **Tout est découvrable** : renommer, dupliquer, supprimer par menu contextuel (clic droit) **et** par boutons visibles ; Annuler/Rétablir visibles dans le header ; toutes les actions dans la palette `Ctrl+K`.
4. **Une seule capture de touche** partout (ajout, changement, découverte, raccourcis), même comportement.
5. **Une seule édition de momentum** : la waveform (glisser) + valeur précise. Plus de triplet slider/champ/waveform répété.

### Ajouts proposés (carte blanche)

- **Mode lecture** : réduit la fenêtre en une petite barre flottante (piste en cours, mood, stop) pour lire un manga à côté. Retour à la fenêtre complète d'un clic.
- **Tags de mood en masse** depuis le tableau Mood AI et l'édition groupée (aujourd'hui touche par touche).
- **Indicateur de touche libre/occupée** visible sur le clavier pendant une capture : on voit immédiatement si on écrase une touche.
- **Historique d'annulation** consultable (liste des 50 dernières actions) dans le menu Annuler.

### Esthétique et mouvement

- Une seule fenêtre **Canvas givrée** (blur natif Wayland), Chrome pour le header, cartes Layer silencieuses, panneaux et popovers Floating. Accent système.
- Couleur de domaine uniquement là où elle porte du sens : **pistes** (palette sombre, contraste ≥ 4,5:1) et **moods** (8 teintes fixes) via `AwraBadge:custom-color` et le futur rôle du clavier.
- Mouvement : enfoncement des touches, pulsation en lecture, waveform qui se remplit, transitions de pages et d'inspecteur via `awra_motion_*` (Fast/Normal/Slow), tout désactivé en « mouvement réduit ».

---

## 3. Ce qui va dans Awra (réutilisable) vs dans l'app

Critère : *une autre app Awra pourrait-elle s'en servir telle quelle ?*

| Élément | Où | Pourquoi |
|---|---|---|
| **`AwraKeyboardView` + `AwraKeyCap`** — clavier rendu selon la disposition GDK, états (libre/lié/actif/pressé/sélectionné/atténué), teinte et liseré personnalisables, sélection simple/multiple, navigation flèches, cible de dépôt par touche | Awra | Tout éditeur de raccourcis, launcher, app de macros, jeu. |
| **`AwraShortcutCapture`** — bouton « appuie sur une touche/combinaison », normalisation (modificateurs triés), validation et signalement de conflits fournis par l'app | Awra | Les réglages de raccourcis de toutes les apps. |
| **`AwraWaveformView`** — enveloppe RMS, curseur de lecture, marqueurs déplaçables (momentum, suggestion), remplissage progressif, accessible au clavier | Awra | Toute app audio/vidéo (éditeur, lecteur, podcast). |
| **`AwraMediaRow`** — titre/sous-titre, seek avec temps écoulé/total, lecture/stop, volume compact | Awra | Lecteurs, files d'attente ; la page Patterns « media workspace » le bricole aujourd'hui. |
| **`AwraDropZone`** + survol de dépôt sur n'importe quel widget (types MIME/extensions acceptés, état de survol) | Awra | Import de fichiers dans n'importe quelle app ; Awra n'a aucun DnD aujourd'hui. |
| **Action sur les toasts** (`AwraToastOverlay` : bouton « Annuler ») | Awra | Le pattern « annuler plutôt que confirmer » sert partout. |
| **`AwraSlider` : préréglage volume** (icône, molette, valeur affichée) | Awra | Toute app média. |
| Barre flottante du mode lecture (fenêtre compacte, toujours visible) | À évaluer : Awra si un patron « mini-player/overlay window » générique se dégage, sinon app | |
| Palette mood, couleurs de pistes, tableau des moods, logique de lecture, découverte | App | Purement métier. |

Chaque ajout suit le processus Awra : en-tête + source C, enums, GIR + baseline API, régénération Rust, tests widget/accessibilité/isolation de style, page Showcase, docs, entrée dans le journal d'usage.

---

## 4. Parité fonctionnelle (rien ne se perd)

Chaque fonctionnalité actuelle a sa place ci-dessus. Liste de contrôle à cocher pendant le port : profils (CRUD, duplication, renommage, sauvegarde auto + état/réessai) · pistes (≤20, volume, renommage) · touches et accords · multi-pistes par touche · sélection multiple et édition groupée · réassignation/fusion/déplacement d'un son · boucles (4 modes) · momentum (modificateur, auto-momentum, suggestion) · volumes son/piste/maître · crossfade · périphérique audio · cooldown · fenêtre d'accord · raccourcis globaux (Stop all, auto-momentum, détection) + conflits · évdev/Wayland (statut + activation pkexec) · disposition clavier (AZERTY) · YouTube (recherche, preview streaming, URL, playlist + sélection, installation yt-dlp, téléchargements parallèles) · fichiers locaux + glisser-déposer · waveforms (cache, progression) · découverte (flux, carousel, preview, momentum, assignation auto, ajout, dislike + gestion, refresh, chargement en arrière-plan, pré-téléchargement) · Mood AI (installation serveur/modèle, progression, démarrer/arrêter, port API, statut extension, tags + intensité, lecture du mood validé, notification) · import/export `.ktm` (progression, annulation, fermeture protégée) · import legacy · fichiers manquants (localiser, re-télécharger, retirer, passer) · annuler/rétablir · toasts · aide des raccourcis · dossiers données/logs.

---

## 5. Plan par étapes

| # | Étape | Résultat vérifiable |
|---|---|---|
| 0 | **Extraction du core** dans un workspace ; `EventSink` ; Tauri devient adaptateur | L'app Tauri compile, ses tests passent, comportement identique |
| 1 | **Port de la logique React en Rust** (lecture/boucles/mood, éditeur + undo, session de découverte) avec tests | Tests unitaires core |
| 2 | **Composants Awra** : KeyboardView/KeyCap, ShortcutCapture, WaveformView, MediaRow, DropZone, action de toast, préréglage volume | Showcase + tests Awra verts |
| 3 | **Squelette natif** : fenêtre, header, clavier, inspecteur, dock pistes ; lecture réelle depuis le clavier global | On joue des sons en lisant |
| 4 | **Pages** : ajout de sons, Découvrir, Mood AI, Bibliothèque, Réglages, mode lecture | Liste de parité cochée |
| 5 | **Finition** : comparaison côte à côte Dark/Light, mouvement, garde « public boundary », journal Awra | Lancement validé par toi |

Taille estimée : grosse refonte (étapes 0–2 sont le socle ; 3–4 l'essentiel du code UI). Je propose de livrer et te montrer l'app après l'étape 3, pour valider la direction visuelle avant de porter toutes les pages.
