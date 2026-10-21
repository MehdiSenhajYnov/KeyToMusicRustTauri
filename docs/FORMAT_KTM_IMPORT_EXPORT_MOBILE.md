# Format `.ktm` : export, import et compatibilité mobile

Ce document décrit le format réellement produit par KeyToMusic et la manière de
réimplémenter son import dans une application mobile. Il distingue :

- le comportement exact de l'application desktop, à reproduire pour rester
  compatible ;
- les protections supplémentaires recommandées sur mobile ;
- l'import d'une URL de playlist YouTube, qui est un flux différent.

Le comportement décrit correspond au code du dépôt au 28 juillet 2026.

## 1. Résumé

Dans KeyToMusic, la « playlist » portable est en réalité un **profil complet**.
Le bouton `Export Profile` crée un fichier `.ktm`, mais ce fichier est simplement
une archive ZIP compressée avec Deflate.

L'archive contient :

```text
Mon profil.ktm
├── profile.json
├── metadata.json
├── sounds/
│   ├── intro.m4a
│   ├── battle.mp3
│   └── battle (2).mp3
└── waveforms.json          # facultatif
```

Un import `.ktm` ne nécessite ni YouTube ni Internet : les fichiers audio sont
embarqués dans l'archive. Les sons provenant de YouTube sont eux aussi exportés
comme des fichiers locaux ; leur URL d'origine n'est pas conservée dans
`source`.

Le fichier `.ktm` ne contient pas :

- les réglages globaux de l'application (`masterVolume`, raccourcis, périphérique
  audio, etc.) ;
- le profil actuellement sélectionné ;
- les caches de découverte ;
- les exécutables `yt-dlp` ou `ffmpeg`.

## 2. Sources de vérité dans le dépôt

Les fichiers à consulter en priorité si le format évolue sont :

- [`src-tauri/src/import_export/export.rs`](../src-tauri/src/import_export/export.rs) :
  construction de l'archive ;
- [`src-tauri/src/import_export/import.rs`](../src-tauri/src/import_export/import.rs) :
  import desktop ;
- [`src-tauri/src/import_export/mod.rs`](../src-tauri/src/import_export/mod.rs) :
  métadonnées et résultat d'import ;
- [`src-tauri/src/types.rs`](../src-tauri/src/types.rs) :
  schéma sérialisé du profil ;
- [`src-tauri/src/audio/analysis.rs`](../src-tauri/src/audio/analysis.rs) :
  schéma des waveforms ;
- [`src-tauri/src/commands.rs`](../src-tauri/src/commands.rs) :
  raccordement Tauri, progression et cache des waveforms.

Le code Rust fait foi si une ancienne spécification ou un exemple diverge.

## 3. Contrat de l'archive

### 3.1 Règles générales

- L'extension conventionnelle est `.ktm`.
- Le contenu est un ZIP standard utilisant Deflate.
- Il n'y a pas de dossier racine englobant.
- `profile.json` à la racine est obligatoire.
- `metadata.json` est toujours écrit par l'exporteur actuel, mais reste
  facultatif pour l'importeur.
- `waveforms.json` est facultatif.
- Les fichiers audio normaux se trouvent sous `sounds/`.
- Les noms d'entrées ZIP utilisent `/`, indépendamment du système d'origine.
- L'importeur desktop ne vérifie pas l'extension : tout fichier qui contient un
  ZIP compatible peut être ouvert.
- Aucune somme de contrôle applicative, signature ou clé de chiffrement n'est
  ajoutée. Le format dépend seulement des contrôles d'intégrité du ZIP.

### 3.2 `metadata.json`

Exemple :

```json
{
  "version": "1.0.0",
  "exportedAt": "2026-07-28T12:34:56.789+00:00",
  "appVersion": "1.0.0",
  "platform": "linux"
}
```

| Champ | Type | Sens |
|---|---|---|
| `version` | chaîne | Version du format `.ktm`, actuellement `1.0.0` |
| `exportedAt` | chaîne | Date UTC RFC 3339 |
| `appVersion` | chaîne | Version du paquet Rust ayant exporté |
| `platform` | chaîne | OS source : typiquement `linux`, `windows` ou `macos` |

Le desktop parse ce fichier lorsqu'il est présent, mais ne compare pas encore
`version` à une liste de versions supportées. Un `metadata.json` absent est
accepté ; un fichier présent mais illisible ou invalide fait échouer l'import.

### 3.3 `profile.json`

Les noms de champs JSON sont en `camelCase`. Voici le contrat utile à un
importeur mobile :

```ts
type SoundSource =
  | { type: "local"; path: string }
  | { type: "youtube"; url: string; cachedPath: string };

type LoopMode = "off" | "random" | "single" | "sequential";

type Mood =
  | "epic"
  | "tension"
  | "sadness"
  | "comedy"
  | "romance"
  | "horror"
  | "peaceful"
  | "mystery";

// Représentation Serde Rust actuelle dans profile.json.
type SerializedMoodIntensity = "Low" | "Medium" | "High";

interface KtmSound {
  id: string;
  name: string;
  source: SoundSource;
  momentum: number;
  volume: number;
  duration: number;
  resolvedVideoId?: string;
}

interface KtmTrack {
  id: string;
  name: string;
  volume: number;
}

interface KtmKeyBinding {
  keyCode: string;
  trackId: string;
  soundIds: string[];
  loopMode: LoopMode;
  currentIndex: number;
  name?: string | null;
  mood?: Mood;
  moodIntensity?: SerializedMoodIntensity;
}

interface KtmProfile {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
  sounds: KtmSound[];
  tracks: KtmTrack[];
  keyBindings: KtmKeyBinding[];
  dislikedVideos?: string[];
}
```

Exemple minimal :

```json
{
  "id": "3a23b3f8-068e-4c20-890e-c81414f264a4",
  "name": "Lecture manga",
  "createdAt": "2026-07-28T10:00:00+00:00",
  "updatedAt": "2026-07-28T11:00:00+00:00",
  "sounds": [
    {
      "id": "c998830f-17d2-4aad-9ea7-05f762318483",
      "name": "Battle Theme",
      "source": {
        "type": "local",
        "path": "sounds/battle.m4a"
      },
      "momentum": 42.5,
      "volume": 0.8,
      "duration": 186.2,
      "resolvedVideoId": "abcdefghijk"
    }
  ],
  "tracks": [
    {
      "id": "14c07df7-8c29-455c-855f-bc3d883192b7",
      "name": "OST",
      "volume": 1.0
    }
  ],
  "keyBindings": [
    {
      "keyCode": "KeyB",
      "trackId": "14c07df7-8c29-455c-855f-bc3d883192b7",
      "soundIds": [
        "c998830f-17d2-4aad-9ea7-05f762318483"
      ],
      "loopMode": "sequential",
      "currentIndex": 0,
      "name": "Combat",
      "mood": "epic",
      "moodIntensity": "Medium"
    }
  ],
  "dislikedVideos": []
}
```

Notes de compatibilité :

- L'exporteur actuel remplace toujours `source` par une source `local` pointant
  vers `sounds/<nom>`, même si le son venait de YouTube.
- `resolvedVideoId`, s'il existe, reste présent séparément.
- `momentum` et `duration` sont exprimés en secondes.
- `volume` est normalement compris entre `0.0` et `1.0`.
- `keyCode` utilise les codes clavier Web comme `KeyA`, `Digit1` ou `Space`.
  Une app mobile peut ne pas les utiliser, mais doit les conserver pour un
  futur réexport.
- `trackId` relie une affectation à une piste ; `soundIds` relie l'affectation
  à un ou plusieurs sons.
- `currentIndex` mémorise la position courante du mode séquentiel.
- `dislikedVideos` est sérialisé par le backend actuel et vaut `[]` par défaut
  lorsqu'il manque dans une ancienne archive.
- `volume` d'une piste vaut `1.0` par défaut lorsqu'il manque.
- Les anciens moods `epic_battle` et `chase_action` sont encore convertis
  respectivement en `epic` et `tension` par le desktop.
- L'ancien mood ambigu `emotional_climax` est supprimé lors de la lecture
  (`mood` devient absent).

Attention à `moodIntensity` : le type TypeScript de l'interface actuelle utilise
`1 | 2 | 3`, alors que l'enum Rust dérivé par Serde utilise actuellement les
chaînes `"Low"`, `"Medium"` et `"High"` dans le JSON persistant. Un importeur
mobile robuste devrait accepter les deux représentations :

| Valeur chaîne | Valeur numérique équivalente |
|---|---:|
| `"Low"` | `1` |
| `"Medium"` | `2` |
| `"High"` | `3` |

Pour produire aujourd'hui un `.ktm` destiné au desktop, utiliser les chaînes.

### 3.4 `waveforms.json`

Le fichier est un objet JSON dont chaque clé est le chemin d'un son dans le ZIP :

```json
{
  "sounds/battle.m4a": {
    "points": [0.02, 0.14, 0.42, 0.21],
    "duration": 186.2,
    "sampleRate": 44100,
    "suggestedMomentum": 42.5
  }
}
```

Schéma :

```ts
interface WaveformData {
  points: number[];
  duration: number;
  sampleRate: number;
  suggestedMomentum: number | null;
}

type KtmWaveforms = Record<string, WaveformData>;
```

Les waveforms sont une optimisation d'affichage et d'analyse, pas une condition
de lecture audio. L'export ne contient que celles déjà présentes dans le cache
mémoire au moment de l'opération. Le fichier peut donc :

- être absent ;
- ne décrire qu'une partie des sons ;
- être ignoré sans empêcher l'import.

L'importeur desktop ignore silencieusement `waveforms.json` si sa lecture ou son
JSON échoue.

## 4. Algorithme d'export desktop actuel

### 4.1 Préparation

1. Le frontend propose par défaut `<nom-du-profil>.ktm`.
2. Le backend charge le profil depuis le stockage local.
3. Il récupère dans le cache mémoire les waveforms disponibles pour les chemins
   audio de ce profil.
4. Il clone le profil afin de pouvoir remplacer les chemins sans modifier le
   profil enregistré.

### 4.2 Collecte des fichiers audio

Pour chaque son du profil cloné :

1. Si `source.type == "local"`, le chemin source est `source.path`.
2. Si `source.type == "youtube"`, le chemin source est `source.cachedPath`.
3. Le fichier doit exister. Si un seul son est absent, l'export complet s'arrête
   avant la création du ZIP.
4. La déduplication utilise le chemin source exact. Si plusieurs sons pointent
   vers le même chemin, l'audio n'est ajouté qu'une fois.
5. Le nom de base du fichier devient son nom dans `sounds/`.
6. Si ce nom est déjà utilisé par un autre fichier, un compteur est ajouté :
   `battle.mp3`, `battle (2).mp3`, `battle (3).mp3`, etc.
7. La source du son dans le clone devient :

   ```json
   { "type": "local", "path": "sounds/battle.mp3" }
   ```

La déduplication ne canonicalise pas les chemins. Deux écritures différentes
désignant physiquement le même fichier peuvent donc produire deux entrées.

### 4.3 Écriture et finalisation

1. `profile.json` et `metadata.json` sont sérialisés.
2. L'archive est d'abord écrite à côté de la destination avec l'extension
   temporaire `.ktm.tmp`.
3. Le chemin temporaire est mémorisé dans
   `export_in_progress.txt`, afin de pouvoir le nettoyer après un crash.
4. Le ZIP reçoit `profile.json`, `metadata.json`, puis les fichiers sous
   `sounds/`.
5. Une progression `(fichier courant, total, nom)` est envoyée entre les
   fichiers. `total` compte les fichiers audio dédupliqués, `current` commence
   à `0`, puis un dernier événement `(total, total, "")` signale la fin.
6. Si des waveforms existent, elles sont remappées vers les chemins
   `sounds/<nom>` puis ajoutées dans `waveforms.json`.
7. Le ZIP est finalisé.
8. Une destination existante est supprimée, puis le fichier temporaire est
   renommé vers le nom final.
9. `export_in_progress.txt` est supprimé.

L'annulation est vérifiée entre deux fichiers audio. Elle supprime le fichier
temporaire et le fichier de suivi. Elle n'interrompt pas au milieu de la lecture
ou de la compression d'un gros fichier.

## 5. Algorithme d'import desktop actuel

Voici le comportement à reproduire si l'objectif est une compatibilité stricte.

### 5.1 Ouverture et lecture du manifeste

1. Vérifier que le fichier sélectionné existe.
2. L'ouvrir comme ZIP.
3. Si `metadata.json` existe, le lire et le parser.
4. Lire obligatoirement `profile.json` et le désérialiser.
5. Les champs JSON inconnus sont ignorés par Serde.

### 5.2 Nouveaux identifiants

1. Générer un UUID v4 pour le nouveau profil.
2. Générer un UUID v4 pour chaque ancien identifiant de son.
3. Remplacer `profile.id`.
4. Renommer le profil en `<ancien nom> (Imported)`.
5. Remplacer `createdAt` et `updatedAt` par l'heure UTC courante au format
   RFC 3339.

Le desktop **ne régénère pas les identifiants de piste**. Les `trackId` présents
dans les affectations restent donc inchangés.

### 5.3 Résolution et extraction d'un son

Pour chaque son :

1. Remplacer son ID grâce à la table `ancienSoundId -> nouveauSoundId`.
2. Lire le chemin relatif depuis :
   - `source.path` pour une source locale ;
   - `source.cachedPath` pour une source YouTube.
3. Si le chemin ne commence pas par `sounds/`, préfixer `sounds/`.
4. Chercher d'abord ce chemin dans le ZIP.
5. Pour les anciens formats, si l'entrée n'existe pas, chercher le simple nom
   du fichier à la racine du ZIP.
6. Extraire le fichier vers :

   ```text
   <data-app>/imported_sounds/<nouveau-profile-id>/<nom-du-fichier>
   ```

7. Remplacer la source, quel que soit son type initial, par :

   ```json
   {
     "type": "local",
     "path": "<chemin-absolu-du-fichier-extrait>"
   }
   ```

L'utilisation du seul nom de fichier pour la destination évite d'extraire
directement un chemin arbitraire fourni par le ZIP.

Si plusieurs sons partagent la même entrée, l'importeur desktop l'extrait
plusieurs fois vers la même destination. Le résultat final est correct mais
l'opération est redondante.

### 5.4 Références, sauvegarde et waveforms

1. Dans chaque `keyBinding.soundIds`, remplacer les anciens IDs par les nouveaux.
2. Un ID référencé mais absent de la table est conservé tel quel.
3. Sauvegarder le profil sous
   `<data-app>/profiles/<nouveau-profile-id>.json`.
4. Lire `waveforms.json` s'il existe.
5. Pour chaque clé `sounds/<nom>`, la remplacer par le nouveau chemin local.
6. Injecter les waveforms obtenues dans le cache mémoire.
7. Retourner le nouvel ID, recharger la liste des profils et sélectionner le
   profil importé.

Après le chargement par l'interface desktop, un second comportement intervient :
les sons qui ne sont référencés par aucun `keyBinding.soundIds` sont retirés du
profil puis la version nettoyée est sauvegardée. L'import backend seul ne fait
pas ce nettoyage.

## 6. Réimplémentation recommandée sur mobile

Le mobile doit conserver le contrat de données ci-dessus, mais il vaut mieux ne
pas recopier littéralement toutes les limites de l'importeur desktop.

### 6.1 Phase A — acquisition et prévalidation

1. Ouvrir le document avec le mécanisme natif :
   - Android : Storage Access Framework et `content://` ;
   - iOS : document picker et URL sécurisée.
2. Copier le flux dans un fichier temporaire privé si la bibliothèque ZIP exige
   un accès aléatoire.
3. Vérifier la signature ZIP, construire une seule fois l'index des entrées et
   refuser :
   - les chemins absolus ;
   - `..` après normalisation ;
   - les entrées qui sortiraient du dossier de staging ;
   - les fichiers spéciaux et liens symboliques si la bibliothèque les expose.
4. Imposer des limites produit sur :
   - la taille de l'archive ;
   - la taille totale décompressée ;
   - la taille d'une entrée ;
   - le nombre d'entrées ;
   - le ratio de compression.
5. Parser `metadata.json` s'il existe.
6. Parser et valider `profile.json` avant d'extraire le moindre audio.

Pour la version de format, une politique simple est :

- accepter `1.x` ;
- afficher un avertissement si `metadata.json` manque ;
- refuser par défaut une version majeure supérieure tant qu'elle n'a pas été
  testée ;
- proposer éventuellement un mode « essayer quand même ».

Cette politique est plus sûre que le desktop actuel, qui ne vérifie pas encore
la version.

### 6.2 Validation du profil

Avant extraction, vérifier au minimum :

- que les collections attendues sont bien des tableaux ;
- que les IDs de sons sont non vides et uniques ;
- que les IDs de pistes sont non vides et uniques ;
- que chaque `trackId` référencé existe ;
- que chaque `soundId` référencé existe ;
- que `loopMode` appartient aux quatre valeurs supportées ;
- que `currentIndex` est positif et compatible avec `soundIds` ;
- que `volume`, `duration` et `momentum` sont des nombres finis ;
- que chaque source fournit un chemin non vide avec un nom de fichier ;
- que chaque audio attendu existe dans l'index ZIP ;
- que deux chemins audio différents ne seront pas écrasés parce qu'ils ont le
  même nom de base.

Pour être tolérant avec les anciennes archives, essayer les entrées dans cet
ordre :

```text
1. chemin déclaré s'il commence par sounds/
2. sounds/ + chemin déclaré
3. nom de fichier seul à la racine
```

Le troisième cas reproduit le fallback historique du desktop. Il ne doit pas
autoriser un chemin de destination fourni par l'archive.

### 6.3 Phase B — plan de transformation

Construire tout le plan avant de modifier le stockage permanent :

- `newProfileId = UUIDv4()`;
- `oldSoundId -> newSoundId`;
- `expectedZipPath -> actualZipEntry`;
- `expectedZipPath -> futureLocalUri`;
- liste des entrées audio uniques à extraire ;
- noms de destination uniques et contrôlés par l'application.

Pour la parité desktop, conserver les IDs de pistes. Si l'architecture mobile
stocke les pistes dans un espace global plutôt que dans le profil, il est
préférable de régénérer aussi leurs IDs et de remapper tous les
`keyBinding.trackId`.

### 6.4 Phase C — extraction en staging

Extraire vers un dossier temporaire privé :

```text
imports/.staging/<newProfileId>/
```

Bon comportement :

- extraire chaque entrée ZIP unique une seule fois ;
- copier en streaming sans charger un gros audio entièrement en mémoire ;
- vérifier la taille réellement écrite et le CRC fourni par le ZIP ;
- publier une progression en octets, plus précise que le compteur de fichiers ;
- supporter l'annulation pendant la copie d'un gros fichier ;
- supprimer entièrement le staging en cas d'erreur ou d'annulation.

Ne jamais concaténer directement un chemin ZIP non vérifié avec le dossier de
destination.

### 6.5 Phase D — transformation et commit

Une fois tous les fichiers présents :

1. Remplacer l'ID et le nom du profil.
2. Mettre les deux timestamps à `now()` en UTC RFC 3339.
3. Remplacer chaque ID de son.
4. Remplacer chaque source par l'URI ou le chemin local privé correspondant.
5. Remapper les `soundIds` de toutes les affectations.
6. Conserver les codes clavier même si le mobile ne les exploite pas.
7. Charger les waveforms valides et remapper leurs clés vers les URI locales.
8. Déplacer atomiquement le staging vers son emplacement définitif lorsque le
   système de fichiers le permet.
9. Enregistrer le profil dans une transaction de base de données ou via un
   fichier temporaire suivi d'un renommage.
10. Activer le nouveau profil seulement après le commit complet.

Si le commit de la base échoue après le déplacement des sons, supprimer les
fichiers déplacés. Si le déplacement échoue, ne jamais créer la ligne de profil.

### 6.6 Pseudocode portable

```text
function importKtm(documentUri):
    tempArchive = copyDocumentToPrivateTemp(documentUri)
    stagingDir = null

    try:
        zip = openZip(tempArchive)
        index = buildAndValidateZipIndex(zip, configuredLimits)

        metadata = readOptionalJson(index, "metadata.json")
        checkSupportedVersion(metadata?.version)

        rawProfile = readRequiredJson(index, "profile.json")
        profile = decodeTolerantProfile(rawProfile)
        validateProfileGraph(profile)

        newProfileId = uuidV4()
        soundIdMap = {}
        for sound in profile.sounds:
            if soundIdMap contains sound.id:
                fail("duplicate sound id")
            soundIdMap[sound.id] = uuidV4()

        extractionPlan = []
        expectedPathToLocalUri = {}
        actualEntryToLocalUri = {}

        for sound in profile.sounds:
            declaredPath = pathFromSource(sound.source)
            expectedPath = withSoundsPrefix(declaredPath)
            actualEntry = resolveCompatibleEntry(
                index,
                expectedPath,
                basename(declaredPath)
            )

            if actualEntry not in actualEntryToLocalUri:
                safeName = reserveUniqueDestinationName(basename(declaredPath))
                futureUri = privateAudioUri(newProfileId, safeName)
                actualEntryToLocalUri[actualEntry] = futureUri
                extractionPlan.append(actualEntry, futureUri)

            expectedPathToLocalUri[expectedPath] =
                actualEntryToLocalUri[actualEntry]

        stagingDir = createPrivateStagingDir(newProfileId)
        extractPlanStreaming(zip, extractionPlan, stagingDir, configuredLimits)

        imported = clone(profile)
        imported.id = newProfileId
        imported.name = profile.name + " (Imported)"
        imported.createdAt = nowUtcRfc3339()
        imported.updatedAt = imported.createdAt

        for sound in imported.sounds:
            oldId = sound.id
            declaredPath = pathFromSource(sound.source)
            expectedPath = withSoundsPrefix(declaredPath)
            sound.id = soundIdMap[oldId]
            sound.source = localSource(expectedPathToLocalUri[expectedPath])

        for binding in imported.keyBindings:
            binding.soundIds = binding.soundIds.map(id => soundIdMap[id])

        waveforms = readOptionalWaveforms(index)
        remappedWaveforms = {}
        for (zipPath, waveform) in waveforms:
            if expectedPathToLocalUri contains zipPath:
                remappedWaveforms[expectedPathToLocalUri[zipPath]] = waveform

        validateProfileGraph(imported)
        commitFilesAndProfileAtomically(stagingDir, imported, remappedWaveforms)
        stagingDir = null
        return imported.id

    finally:
        deleteIfExists(tempArchive)
        deleteIfExists(stagingDir)
```

Le pseudocode choisit de faire échouer une référence de son invalide. Le desktop
conserve aujourd'hui l'ancien ID lorsqu'il ne trouve pas de correspondance, mais
cela crée une affectation inutilisable ; le refus explicite est préférable.

## 7. Export compatible depuis le mobile

Pour permettre un aller-retour mobile vers desktop :

1. Charger le profil et les sons dans un snapshot cohérent.
2. Pour chaque son, ouvrir son fichier privé ou son `content://`.
3. Dédupliquer les sources identiques.
4. Réserver un nom de base unique :
   - `name.ext`;
   - `name (2).ext`;
   - `name (3).ext`.
5. Dans la copie du profil, remplacer toutes les sources par
   `{ "type": "local", "path": "sounds/<nom>" }`.
6. Écrire un `metadata.json` avec `version: "1.0.0"`.
7. Écrire `profile.json`, `metadata.json`, les audios et éventuellement
   `waveforms.json` dans un ZIP Deflate sans dossier racine.
8. Écrire d'abord dans un fichier temporaire, finaliser le ZIP, puis publier le
   document.

Pour correspondre au desktop actuel :

- utiliser des UUID v4 sous forme de chaînes ;
- sérialiser les champs en `camelCase` ;
- utiliser les quatre valeurs minuscules de `loopMode` ;
- écrire `mood` en `snake_case` ;
- écrire actuellement `moodIntensity` sous forme `"Low"`, `"Medium"` ou
  `"High"` ;
- indexer les waveforms par `sounds/<nom>`, jamais par le chemin privé mobile.

Un chemin Android `content://...` ou une URL de sandbox iOS ne doit jamais
apparaître dans le `profile.json` exporté.

## 8. Playlist YouTube : flux distinct

L'option `Download entire playlist` ne lit et n'écrit pas un fichier `.ktm`.
Sur desktop :

1. l'URL est passée à `yt-dlp --flat-playlist --dump-json --no-download` ;
2. les vidéos supprimées, privées ou indisponibles sont ignorées ;
3. le titre, l'ID, la durée, la chaîne et la miniature de chaque entrée sont
   présentés à l'utilisateur ;
4. toutes les entrées sont sélectionnées par défaut ;
5. chaque sélection est ensuite téléchargée comme un son individuel ;
6. ces sons rejoignent le profil courant et seront embarqués lors d'un futur
   export `.ktm`.

Il n'existe donc pas, à ce jour, de manifeste `.ktmplaylist` produit par cette
fonction. Une idée de format de playlist Discovery est mentionnée dans les
tâches, mais elle n'est pas implémentée.

Sur mobile, l'import d'une URL YouTube doit être conçu séparément du lecteur
`.ktm` : API distante, service backend ou autre stratégie conforme aux règles
de la plateforme. Cela ne change pas l'algorithme d'import `.ktm`, qui reste
entièrement local.

## 9. Compatibilité et cas limites

| Cas | Comportement desktop actuel | Recommandation mobile |
|---|---|---|
| Extension différente de `.ktm` | Acceptée si le contenu est un ZIP valide | Accepter après confirmation ou vérifier le contenu |
| `profile.json` absent | Échec | Échec |
| `metadata.json` absent | Accepté | Accepter avec avertissement |
| `metadata.json` invalide | Échec | Échec |
| Version future | Parsée mais non contrôlée | Contrôler la version majeure |
| `waveforms.json` absent | Accepté | Accepter |
| `waveforms.json` invalide | Ignoré | Ignorer et journaliser |
| Waveform partielle | Acceptée | Accepter |
| Audio absent | Échec | Échec avant commit |
| Audio corrompu mais copiable | Importé, erreur possible à la lecture | Sonder le format avant commit si possible |
| Source YouTube dans le JSON | `cachedPath` est lu, puis la source devient locale | Même comportement |
| Audio placé à la racine | Fallback par nom de fichier | Supporter pour les anciens exports |
| Deux sources avec le même nom | Risque d'écrasement sur archive externe mal formée | Réserver deux noms locaux distincts |
| Même source utilisée plusieurs fois | Réextraction redondante | Extraire une seule fois |
| Référence de son inconnue | Ancien ID conservé | Refuser ou retirer explicitement la référence |
| Échec au milieu de l'import | Dossier partiel possiblement laissé | Staging et rollback |
| Archive ZIP surdimensionnée | Pas de quota applicatif | Limites anti-ZIP-bomb |
| IDs de pistes | Conservés | Conserver, ou remapper si stockage global |
| Son sans affectation | Importé puis supprimé au chargement UI | Choisir et documenter la politique |

## 10. Limites connues du format 1.0.0

- Il n'existe pas de JSON Schema officiel embarqué dans l'archive.
- Il n'existe pas de somme SHA-256 par fichier.
- La version n'est pas encore utilisée par le desktop pour les migrations.
- L'import desktop n'est pas transactionnel et peut laisser un dossier audio
  partiel après une erreur.
- Les IDs de pistes ne sont pas régénérés.
- Les données YouTube d'origine sont en grande partie perdues, puisque la source
  devient locale.
- Les codecs audio acceptés par le desktop ne sont pas nécessairement pris en
  charge nativement par Android et iOS. L'import doit préserver les octets ;
  la lecture peut nécessiter un décodeur ou une conversion séparée.
- La divergence actuelle de représentation de `moodIntensity` doit être tolérée
  par les clients.

## 11. Tests de compatibilité conseillés

Créer au moins une fixture `.ktm` de référence, petite et autorisée à être
versionnée, puis couvrir :

1. un profil vide ;
2. un son local ;
3. un ancien son YouTube exporté comme local ;
4. plusieurs sons partageant le même fichier ;
5. deux fichiers différents ayant le même nom de base ;
6. un profil sans waveforms ;
7. un profil avec waveforms partielles ;
8. un ancien audio placé à la racine du ZIP ;
9. un ZIP corrompu ;
10. un `profile.json` invalide ;
11. un audio manquant ;
12. des IDs ou références dupliqués ;
13. une tentative de traversal comme `sounds/../../x`;
14. une archive dépassant les quotas ;
15. une annulation au milieu d'un gros fichier ;
16. un aller-retour desktop → mobile → desktop.

Pour inspecter manuellement un export :

```bash
unzip -l "Mon profil.ktm"
unzip -p "Mon profil.ktm" metadata.json | jq
unzip -p "Mon profil.ktm" profile.json | jq
unzip -p "Mon profil.ktm" waveforms.json | jq
```

La dernière commande doit naturellement accepter que `waveforms.json` soit
absent.

Critères de réussite d'un aller-retour :

- tous les audios sont lisibles et leurs octets sont inchangés si aucune
  conversion n'est prévue ;
- les IDs de sons ont changé à l'import ;
- toutes les références `soundIds` pointent vers les nouveaux IDs ;
- pistes, volumes, modes, Momentum, moods et ordre des sons sont conservés ;
- les chemins du JSON importé pointent vers le stockage privé de la nouvelle
  application ;
- aucun chemin privé de l'application précédente ne subsiste ;
- l'import interrompu ne laisse ni profil visible ni fichiers orphelins.

## 12. Checklist d'implémentation mobile

- [ ] Sélecteur de document `.ktm`
- [ ] Copie temporaire privée du document
- [ ] Lecteur ZIP Deflate en streaming
- [ ] Validation des chemins et quotas
- [ ] Modèles JSON tolérants
- [ ] Validation du graphe profil/pistes/sons/affectations
- [ ] UUID v4 pour le profil et les sons
- [ ] Extraction unique des audios dans un staging
- [ ] Remappage des sources et des `soundIds`
- [ ] Préservation des `keyCode` pour le réexport
- [ ] Import facultatif des waveforms
- [ ] Commit atomique et rollback
- [ ] Progression et annulation
- [ ] Tests avec une fixture exportée par le desktop
- [ ] Export mobile compatible pour valider le round-trip
