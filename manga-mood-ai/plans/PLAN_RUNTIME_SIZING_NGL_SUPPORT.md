# Plan Runtime Sizing / `-ngl` Adaptatif / Suppression CPU Auto

## Scope

Ce plan couvre uniquement les trois points suivants:

1. remplacer le runtime fixe `32768x4` par un sizing base sur la charge reelle
2. ajouter un `-ngl` adaptatif
3. retirer le CPU du chemin auto "supporte" pour la feature manga mood

Ce document ne couvre pas la strategie CUDA / backend vendor-specific, qui est suivie a part.

---

## Objectif produit

Rendre le runtime `llama-server`:

- plus leger
- plus stable
- plus proche du vrai besoin du pipeline gagnant `wide5_selective`
- moins susceptible de faire freezer la machine
- plus honnete sur les machines non supportees

Le but n'est pas de changer l'algorithme de detection de mood.
Le but est d'optimiser l'execution du pipeline existant.

---

## Etat actuel

### Ce qu'on a aujourd'hui

- `LlamaServerStartOptions` sait deja recevoir:
  - `context_size`
  - `parallel_slots`
- le bench RealTest sait deja definir des profils runtime via `RuntimeProfile`
- le demarrage `llama-server` force encore:
  - `-c 32768` par defaut
  - `-ngl 99`
  - `-np` seulement si on le passe explicitement

### Probleme actuel

- le `32768x4` est une valeur de recherche trop large, pas une bonne base produit
- `-np 4` n'apporte pas grand-chose si les requetes sont en pratique sequentielles
- `-ngl 99` suppose un full offload ideal qui n'est pas toujours le meilleur compromis
- le CPU reste implicite dans certains chemins de selection, alors qu'il n'est pas viable pour la feature mood

---

## Decision de haut niveau

### Ce qu'on veut mettre en place

- des profils runtime explicites par type de tache
- un choix automatique du profil selon l'usage
- un choix automatique de `-ngl` selon la memoire estimee
- un refus propre de la feature si aucun GPU viable n'est disponible

### Ce qu'on ne veut pas

- un seul profil geant pour toutes les requetes
- un fallback CPU silencieux
- un systeme magique trop complexe a raisonner

---

## Architecture cible

## 1. Runtime intent

Introduire une notion de "type de requete runtime" au lieu de n'avoir qu'un seul profil grossier.

Exemples cibles:

- `PrimaryWide5`
  - backbone du pipeline gagnant
- `SelectiveReprompt`
  - reprompt `focus` / `narrative`
- `ResearchLarge`
  - profil reserve aux benchs lourds / historique

Chaque intent choisit:

- `context_size`
- `parallel_slots`
- politique `-ngl`

---

## 2. Profils cibles

Valeurs de depart a tester lors de l'implementation:

### `PrimaryWide5`

- `context_size`: `8192` ou `12288`
- `parallel_slots`: `1`
- `-ngl`: adaptatif

Raison:
- pipeline `wide5_selective` traite les pages sequentiellement
- `np=4` reserve trop de ressources pour peu de gain reel
- il faut rester confortable en VRAM

### `SelectiveReprompt`

- `context_size`: `4096` ou `8192`
- `parallel_slots`: `1`
- `-ngl`: adaptatif

Raison:
- reprompt sur 3 pages seulement
- prompt plus local
- besoin memoire plus faible

### `ResearchLarge`

- `context_size`: `32768`
- `parallel_slots`: `4` ou configurable
- `-ngl`: adaptatif mais permissif

Raison:
- garder un mode bench/recherche comparable aux anciens runs
- ne pas melanger les besoins prod et bench historique

---

## 3. Politique `-ngl`

### Principe

`-ngl` ne doit plus etre hardcode a `99`.

Il doit etre choisi selon:

- la memoire GPU estimee
- le type de modele
- le profil runtime demande

### Version simple a implementer d'abord

Support en 3 niveaux:

- **Tier A**
  - VRAM confortable
  - `-ngl 99`
- **Tier B**
  - VRAM moyenne / marge faible
  - `-ngl` intermediaire
- **Tier C**
  - VRAM insuffisante pour full offload
  - `-ngl` faible ou refus de lancer si la feature ne sera pas viable

### Source de verite proposee

Ordre de priorite:

1. valeur explicite env override
2. detection VRAM dispo si possible
3. fallback conservateur par modele

### Point important

Le fallback final ne doit **pas** etre CPU auto.
Si on tombe en dessous d'un seuil de viabilite:

- on refuse la feature
- ou on exige un override explicite

---

## 4. Politique CPU

### Nouvelle regle

- CPU retire du chemin "auto supporte"
- CPU conserve seulement pour:
  - debug
  - tests manuels
  - override explicite

### UX cible

Si aucun GPU viable n'est disponible:

- ne pas demarrer la feature mood silencieusement
- remonter un statut clair:
  - "GPU acceleration required for manga mood"
  - ou equivalent cote UI/logs

### Effet attendu

- moins de faux positifs "feature supportee mais inutilisable"
- moins de temps perdu sur des machines non viables

---

## Plan d'implementation

## Phase 1 - Refactor du runtime config

Objectif:
- sortir d'un demarrage `llama-server` pilote par quelques options isolees

Actions:
- etendre `LlamaServerStartOptions` pour inclure:
  - `gpu_layers`
  - eventuellement un `runtime_intent`
- centraliser la construction des flags runtime dans une fonction dediee
- arreter de logger un faux `-c 32768 -ngl 99` si les valeurs deviennent dynamiques

Livrable:
- un builder runtime unique et lisible

## Phase 2 - Profils runtime explicites

Objectif:
- differencier backbone / reprompt / recherche

Actions:
- introduire une enum de profils runtime
- mapper chaque appel a un profil:
  - pipeline gagnant principal -> `PrimaryWide5`
  - reprompts selectifs -> `SelectiveReprompt`
  - bench legacy -> `ResearchLarge`
- garder les profils bench actuels dans `director_realtest_suite`, mais les faire reposer sur le meme systeme

Livrable:
- plus de runtime "one size fits all"

## Phase 3 - Detection / estimation memoire

Objectif:
- pouvoir choisir `-ngl` sans hardcode universel

Actions:
- ajouter une fonction de detection simple:
  - support NVIDIA via `nvidia-smi` si dispo
  - support override via env
  - fallback conservateur sinon
- separer:
  - memoire totale estimee
  - memoire libre estimee si disponible

Livrable:
- petit module runtime hardware probe

## Phase 4 - Table de decision `-ngl`

Objectif:
- convertir l'estimation memoire en valeur `-ngl`

Actions:
- creer une table simple par modele / profil
- commencer par des heuristiques fixes
- ne pas chercher une formule compliquee au debut

Exemple de logique:

- `4B` + `PrimaryWide5` + grosse marge VRAM -> `99`
- `4B` + marge moyenne -> valeur intermediaire
- `4B` + marge faible -> valeur basse ou refus

Livrable:
- fonction pure testable: `choose_gpu_layers(...)`

## Phase 5 - Suppression CPU auto

Objectif:
- rendre le comportement produit honnete

Actions:
- retirer le CPU des chemins auto de selection/support de la feature
- si pas de GPU viable:
  - renvoyer un statut d'indisponibilite
  - ne pas lancer `llama-server` pour le mood
- garder un override explicite si on veut encore du CPU debug

Livrable:
- feature mood = GPU obligatoire par defaut

## Phase 6 - Retry / fallback intelligent

Objectif:
- eviter l'echec brutal si le profil choisi est trop ambitieux

Actions:
- si le serveur ne demarre pas:
  - retenter avec un profil plus conservateur
  - reduire `-ngl`
  - eventuellement reduire `context_size`
- limiter le nombre de retries

Ordre propose:

1. profil cible
2. meme profil avec `-ngl` plus bas
3. profil plus petit
4. abandon propre

Livrable:
- runtime auto plus robuste sans devenir opaque

---

## Validation / benchmarks a prevoir

## 1. Validation fonctionnelle

- le serveur demarre toujours sur la machine de dev principale
- les profils backbone / reprompt sont bien differencies
- le CPU n'est plus pris par defaut

## 2. Validation perf

Comparer:

- `wide5_selective` avec ancien runtime
- `wide5_selective` avec nouveau runtime

Mesures:

- temps moyen par page
- temps p95 par page
- VRAM max
- nb de reprompts
- score strict / relaxed inchanges ou meilleurs
- sensation de fluidite machine

## 3. Validation de robustesse

- echec volontaire de profil trop ambitieux
- verification que le retry degrade proprement
- verification qu'on n'atterrit pas silencieusement sur CPU

---

## Decisions concretes a conserver

### On met en place

- sizing dynamique `context_size` / `parallel_slots`
- `-ngl` adaptatif
- retrait du CPU du chemin auto supporte
- retries intelligents avec profil plus conservateur

### On ne met pas en place dans ce chantier

- changement de modele
- changement d'algorithme mood
- CUDA / strategie backend vendor-specific
- `--cache-ram`

---

## Questions a trancher au moment de l'implementation

- `PrimaryWide5` doit-il commencer a `8192` ou `12288` ?
- `SelectiveReprompt` doit-il commencer a `4096` ou `8192` ?
- refuse-t-on la feature selon VRAM totale, VRAM libre, ou les deux ?
- veut-on exposer un mode "force conservative runtime" dans les settings plus tard ?

---

## Definition of done

Le chantier est termine quand:

- le runtime n'utilise plus un `32768x4` implicite comme default general
- `-ngl` est choisi automatiquement
- le CPU n'est plus un fallback auto pour la feature mood
- `wide5_selective` passe toujours le benchmark de reference
- la machine reste au moins aussi fluide qu'avant, idealement plus
