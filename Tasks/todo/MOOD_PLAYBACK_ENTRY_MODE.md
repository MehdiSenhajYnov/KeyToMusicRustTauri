# Mood Playback Entry Mode

> **Catégorie:** Feature / Amelioration
> **Priorité:** Moyenne
> **Statut:** ⏳ A faire
> **Date ajoutee:** 2026-03-08

## Description

Ajouter un vrai mode d'entree de lecture pour les sons declenches par le mood AI.

Aujourd'hui, le playback automatique mood utilise pratiquement toujours le `momentum` des qu'un son en a un. Cela pose un probleme narratif:

- certains sons doivent partir au debut pour laisser l'intro et le build-up s'installer
- d'autres doivent partir au momentum pour frapper immediatement

Le chantier vise a introduire une logique explicite `start vs momentum`, idealement avec un mode `auto`.

## Probleme actuel

Dans [useMoodPlayback.ts](/home/mehdi/Dev/KeyToMusicRustTauri/src/hooks/useMoodPlayback.ts), le playback mood fait aujourd'hui:

- `useMomentum = config.autoMomentum || sound.momentum > 0`

Consequence:

- si un son a un momentum defini, le mood playback auto le lance au momentum
- il n'existe pas de decision produit "ce son doit partir du debut dans ce contexte"
- le systeme confond `point d'entree disponible` et `point d'entree souhaite`

## Objectif produit

Quand un mood est committe, le systeme doit pouvoir choisir intelligemment entre:

- `start`: lancer le son depuis `0`
- `momentum`: lancer le son depuis `sound.momentum`
- `auto`: choisir entre les deux selon le contexte

Le but n'est pas seulement technique. Il s'agit d'ameliorer la coherence soundtrack:

- laisser respirer les morceaux d'ambiance, romance, mystery, peaceful quand c'est possible
- eviter de perdre plusieurs secondes de build-up quand la scene demande un impact immediat

## Portee

### Inclus

- playback automatique declenche par `mood_committed`
- nouveaux modes d'entree audio pour les sons et/ou pour le systeme mood
- heuristique `auto`
- UI pour regler et comprendre ce comportement
- telemetry/debug minimal pour comprendre pourquoi un son est parti au debut ou au momentum

### Exclu dans une premiere iteration

- changement du moteur d'analyse mood lui-meme
- fine-tuning/ML dedie pour predire l'entree audio
- rework complet du moteur audio

## Comportement cible

### Regle produit

Le systeme ne doit plus supposer:

- `sound.momentum > 0` => partir au momentum

Il doit raisonner ainsi:

- `entry_mode = start`
  - toujours partir au debut
- `entry_mode = momentum`
  - toujours partir au momentum
- `entry_mode = auto`
  - choisir selon le contexte narratif et le temps disponible

### Priorite de decisions

1. Si le son est configure en `start`, partir au debut
2. Si le son est configure en `momentum`, partir au momentum
3. Si le son est configure en `auto`, appliquer une heuristique explicite
4. Le toggle global `autoMomentum` doit etre reclarifie:
   - soit il reste un override legacy
   - soit il est remplace par un mode plus clair oriente "forcer momentum"

## Proposition d'architecture

### 1. Nouveau concept: playback entry mode

Introduire un enum commun frontend/backend du type:

- `start`
- `momentum`
- `auto`

Optionnel plus tard:

- `continue`

Le mode `continue` n'est pas necessaire pour la V1, mais il peut devenir utile si l'on veut plus tard conserver une ambiance deja installee au lieu de relancer un son.

### 2. Decision centralisee

La decision ne doit pas etre recodee a plusieurs endroits.

Creer une fonction centrale du type:

- `resolveMoodPlaybackStartPosition(...)`

Elle prendrait au minimum:

- le son
- sa duree
- son momentum
- le mood committe
- l'intensite
- la source / raison du commit
- la configuration globale
- eventuellement des infos de contexte lecture plus tard

Et elle retournerait:

- `start_position`
- `decision_mode`
- `decision_reason`

### 3. Separation entre metadata audio et decision narrative

Le systeme doit distinguer:

- ce que le fichier audio permet
  - duree totale
  - momentum
  - longueur d'intro approximative (`momentum`)
- ce que la scene demande
  - urgence immediate
  - temps probable avant changement
  - nature setup vs payoff

## Heuristique V1 recommandee

### Principes

La V1 ne doit pas dependre d'un nouveau signal IA. Elle doit etre heuristique, stable, et comprehensible.

### Variables disponibles

- `sound.momentum`
- `sound.duration`
- `mood`
- `intensity`
- config globale

### Regles de base proposees

#### Partir plutot au `start`

- `peaceful`, `romance`, `mystery` faible ou moyen
- `sadness` quand le morceau a une intro courte ou moyenne utile
- sons avec momentum faible ou intro courte
- contextes de setup / ambiance durable

#### Partir plutot au `momentum`

- `epic` intensite haute
- `tension` intensite haute
- pages de payoff ou d'impact immediat
- sons dont l'intro avant momentum est longue
- cas ou on estime qu'on n'aura pas le temps de laisser l'intro respirer

### Regle concrete minimale

V1 simple et robuste:

- si `entry_mode = auto`
- calculer `intro_len = sound.momentum`
- definir une table d'urgence par mood/intensite
- si `intro_len` depasse un seuil selon l'urgence, utiliser `momentum`
- sinon utiliser `start`

Exemple de logique:

- `epic 3`, `tension 3`, `horror 3` -> seuil intro tres court
- `mystery 1-2`, `peaceful`, `romance` -> seuil intro plus permissif

## V2 envisageable

Une V2 pourra affiner la decision avec du contexte produit reel:

- vitesse de lecture detectee par l'extension
- nombre de pages de buffer deja prêtes
- type de prediction backend
  - setup
  - bridge
  - payoff
- score de confiance mood
- eventuelle stabilite du mood courant

Cette V2 pourrait ajouter un signal de haut niveau du style:

- `playback_entry_urgency`

Mais ce n'est pas requis pour la premiere livraison.

## Impact UX / produit

Le changement doit rester compréhensible pour l'utilisateur: il faut rendre le comportement plus intelligent sans transformer le setup audio en panneau de réglages obscur.

## UI a ajouter ou modifier

### Son individuel

Dans les ecrans de sons:

- afficher un select `Mood playback entry`
- valeurs:
  - `Auto`
  - `Start from beginning`
  - `Start from momentum`

### Reglages globaux

Le toggle actuel `autoMomentum` est ambigu.

Il faudra le reevaluer:

- soit le conserver comme `Force momentum globally`
- soit le migrer vers une option plus claire
- soit le retirer du chemin mood auto et le reserver au clavier manuel

### Debug

Le produit doit pouvoir expliquer un declenchement:

- `Started from beginning`
- `Started from momentum`
- `Reason: auto -> epic/high urgency + long intro`

Sans cette visibilite, il sera difficile d'ajuster l'heuristique.

## Fichiers probablement concernes

- [src/types/index.ts](/home/mehdi/Dev/KeyToMusicRustTauri/src/types/index.ts)
- [src/stores/settingsStore.ts](/home/mehdi/Dev/KeyToMusicRustTauri/src/stores/settingsStore.ts)
- [src/hooks/useMoodPlayback.ts](/home/mehdi/Dev/KeyToMusicRustTauri/src/hooks/useMoodPlayback.ts)
- [src/components/Sounds/SoundDetails.tsx](/home/mehdi/Dev/KeyToMusicRustTauri/src/components/Sounds/SoundDetails.tsx)
- [src/components/Sounds/AddSoundModal.tsx](/home/mehdi/Dev/KeyToMusicRustTauri/src/components/Sounds/AddSoundModal.tsx)
- [src/components/Settings/SettingsModal.tsx](/home/mehdi/Dev/KeyToMusicRustTauri/src/components/Settings/SettingsModal.tsx)
- [src-tauri/src/audio/analysis.rs](/home/mehdi/Dev/KeyToMusicRustTauri/src-tauri/src/audio/analysis.rs)
- [src-tauri/src/types.rs](/home/mehdi/Dev/KeyToMusicRustTauri/src-tauri/src/types.rs)
- [docs/KeyToMusic_Technical_Specification.md](/home/mehdi/Dev/KeyToMusicRustTauri/docs/KeyToMusic_Technical_Specification.md)

## Migration / compatibilite

Le chantier doit preserver les profils existants.

Proposition:

- si un son n'a pas encore `moodPlaybackEntryMode`, utiliser `auto`
- ne pas casser le `momentum` manuel via clavier
- garder une compatibilite claire avec `autoMomentum` pendant une phase transitoire

## Risques

- heuristique trop agressive qui casse les ambiances
- heuristique trop conservatrice qui rend la feature inutile
- confusion produit entre:
  - momentum manuel
  - mood playback auto
  - override global legacy
- multiplication de reglages peu comprehensibles si l'UI n'est pas tres claire

## Strategie d'implementation recommandee

### Phase 1

- introduire le concept `start | momentum | auto`
- brancher la decision uniquement dans `useMoodPlayback`
- ne pas toucher au workflow clavier manuel
- ajouter une raison de decision dans le debug/toast/log

### Phase 2

- ajouter UI par son
- clarifier ou migrer `autoMomentum`
- ajouter tests unitaires de decision

### Phase 3

- ajouter signaux de contexte plus riches si necessaire
- eventuellement integrer vitesse de lecture / urgence narrative

## Taches

- [ ] Definir l'enum `MoodPlaybackEntryMode`
- [ ] Ajouter le champ de config necessaire sur les sons ou bindings
- [ ] Revoir la semantique de `autoMomentum`
- [ ] Creer une fonction centrale de resolution `start vs momentum`
- [ ] Implementer la V1 heuristique basee sur `momentum`, `duree`, `mood`, `intensite`
- [ ] Brancher cette decision dans `useMoodPlayback`
- [ ] Ajouter l'UI de reglage par son
- [ ] Ajouter un feedback de debug visible dans l'app
- [ ] Ajouter des tests unitaires sur la logique de decision
- [ ] Documenter la nouvelle semantique utilisateur

## Criteres d'acceptation

- un son peut explicitement partir du debut, du momentum, ou en auto
- le mood playback auto n'utilise plus implicitement le momentum des qu'il existe
- le comportement est explicable par l'UI ou les logs
- les profils existants restent lisibles sans migration destructive
- le clavier manuel continue de se comporter comme avant

## Notes

- Ce document decrit un chantier futur. Aucune implementation n'est demandee dans cette tache.
- La V1 doit rester heuristique et simple. Ne pas ouvrir tout de suite un sous-projet de prediction ML pour cette decision.
