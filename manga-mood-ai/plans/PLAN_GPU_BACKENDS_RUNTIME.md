# Plan GPU Backends / CUDA Runtime

## Objectif

Documenter la strategie future pour le runtime `llama-server` de la feature manga mood:

- support multi-plateforme
- support multi-vendor GPU
- pas de faux support CPU "prod"
- optimisation du temps de calcul et de la fluidite machine
- garder une base simple a maintenir

Ce document est un plan d'action futur. Il ne decrit pas ce qui est deja implemente.

---

## Contraintes produit

- La feature mood doit rester locale.
- Le runtime doit etre raisonnable pour de la lecture manga en arriere-plan.
- Le support doit viser en priorite Windows + Linux.
- macOS reste desirable, mais pas prioritaire a court terme.
- On ne connait pas a l'avance la marque GPU:
  - NVIDIA
  - AMD
  - Intel
- Le fallback CPU ne doit pas etre considere comme un mode supporte pour la feature mood, car le temps de calcul devient non viable.

---

## Decision produit

### Ce qu'on veut

- un backend GPU auto-selectionne proprement
- un override manuel si besoin
- un support "best backend available" selon OS + GPU
- des profils runtime adaptes au vrai besoin
- refuser proprement la feature si aucun backend GPU viable n'est disponible

### Ce qu'on ne veut pas

- faire semblant de supporter le CPU en prod pour le mood
- un seul backend universel impose a tout le monde
- des builds exotiques trop couteux a maintenir si le gain produit est faible

---

## Position CPU

### Politique cible

- **CPU = debug / secours manuel / dev**
- **CPU != backend supporte officiellement pour manga mood**

### Consequence produit

Si aucun backend GPU viable n'est detecte:

- la feature mood doit etre marquee comme non supportee sur cette machine
- l'utilisateur peut eventuellement forcer un mode CPU a ses risques, mais pas via le chemin normal recommande

### Pourquoi

- le budget cible est deja serre avec GPU
- un VLM sur CPU est trop lent pour une UX de lecture continue
- garder un fallback CPU automatique reviendrait a promettre une feature qui sera en pratique inutilisable

---

## Support matrix cible

### Windows

- NVIDIA:
  - backend cible: `CUDA`
  - backend de secours: `Vulkan`
- AMD:
  - backend cible: `HIP` si valide chez nous
  - backend de secours: `Vulkan`
- Intel:
  - backend cible: `Vulkan`
  - `SYCL` pourra etre reconsidere plus tard si on veut optimiser Intel explicitement

### Linux

- NVIDIA:
  - court terme: `Vulkan`
  - moyen terme optionnel: `CUDA` via build/pipeline maintenu par nous
- AMD:
  - backend cible: `ROCm` si packaging/runtime propre
  - backend de secours: `Vulkan`
- Intel:
  - backend cible: `Vulkan`

### macOS

- Apple Silicon:
  - backend cible: `Metal`
- Intel:
  - non prioritaire pour cette feature

---

## Priorites de support

### Tier 1

- Windows NVIDIA -> CUDA
- Windows AMD -> Vulkan ou HIP selon validation
- Linux NVIDIA -> Vulkan
- Linux AMD -> Vulkan ou ROCm selon validation

### Tier 2

- Windows Intel -> Vulkan
- Linux Intel -> Vulkan

### Tier 3

- macOS Apple Silicon -> Metal

### Hors scope actuel

- CPU comme mode auto
- multi-GPU avance
- Linux NVIDIA CUDA officiel si cela exige trop de maintenance build/distribution

---

## Plan d'action CUDA / backends

## Phase 1 - Abstraction backend

Objectif:
- sortir d'une logique binaire "Vulkan ou CPU"

Actions:
- introduire une notion explicite de backend runtime:
  - `Cuda`
  - `Hip`
  - `Rocm`
  - `Vulkan`
  - `Metal`
  - `Sycl`
  - `Cpu`
- separer:
  - detection hardware
  - selection backend
  - selection asset telechargeable
  - flags runtime associes

Livrable:
- une fonction centrale de decision backend, testable independamment

## Phase 2 - Policy auto-selection

Objectif:
- choisir automatiquement le meilleur backend viable

Actions:
- detecter l'OS
- detecter le vendor GPU principal si possible
- definir un ordre de preference par plate-forme
- permettre un override utilisateur / variable d'env

Regles cibles:
- Windows NVIDIA -> CUDA
- Linux NVIDIA -> Vulkan par defaut
- si le backend prefere n'est pas disponible -> essayer le suivant
- si aucun backend GPU viable -> feature non supportee

Livrable:
- matrice de fallback claire, logguee au demarrage

## Phase 3 - Downloader / packaging

Objectif:
- telecharger le bon binaire `llama-server` selon backend

Actions:
- etendre la logique actuelle de `llama_manager`
- ne plus supposer "Linux = Vulkan sinon CPU"
- supporter au minimum:
  - Windows CUDA
  - Windows Vulkan
  - Windows HIP si valide
  - Linux Vulkan
  - Linux ROCm si valide
  - macOS Metal plus tard

Important:
- Linux CUDA n'a pas aujourd'hui le meme niveau de packaging prebuild que Windows CUDA dans les releases officielles
- si on veut Linux CUDA propre, il faudra probablement maintenir notre propre build/pipeline

Livrable:
- table asset -> backend -> OS

## Phase 4 - Runtime flags par backend

Objectif:
- avoir des flags adaptes au backend reel

Actions:
- garder des flags communs:
  - `--flash-attn`
  - `--cache-type-k q8_0`
  - `--cache-type-v q8_0`
- definir les flags qui varient:
  - `-ngl`
  - `-c`
  - `-np`
- ne pas fixer un seul profil "32768x4" partout

Livrable:
- profils runtime centralises et choisis selon la charge reelle

## Phase 5 - Linux NVIDIA / CUDA etude

Objectif:
- decider si Linux NVIDIA merite un backend CUDA maintenu par nous

Actions:
- benchmark `Vulkan Linux NVIDIA` vs `CUDA Linux NVIDIA`
- mesurer:
  - temps page
  - temps warmup
  - peak VRAM
  - stabilite
  - fluidite machine
- estimer le cout de maintenance:
  - build CI
  - distribution
  - dependances runtime CUDA

Decision cible:
- si gain net significatif et maintenance acceptable -> supporter Linux CUDA
- sinon rester sur Vulkan pour Linux NVIDIA

## Phase 6 - UX produit

Objectif:
- clarifier ce qui est supporte ou non

Actions:
- afficher le backend choisi
- afficher pourquoi un backend de secours a ete choisi
- si aucun backend GPU valide:
  - ne pas lancer la feature silencieusement
  - afficher un message explicite de non-support
- garder un override avance pour debug

---

## Ce qu'on veut mesurer quand on fera ce chantier

- temps moyen par page
- temps p95 par page
- temps de warmup serveur
- VRAM max
- impact sur fluidite machine
- taux de crash / echec demarrage
- score benchmark identique ou non

Important:
- un nouveau backend doit d'abord etre traite comme un gain **perf/stabilite**
- pas comme un gain de precision
- la precision ne peut monter qu'indirectement si la marge de perf permet un meilleur protocole

---

## Decision provisoire par sujet

### A faire plus tard

- abstraction backend propre
- policy auto-selection multi-OS / multi-vendor
- suppression du CPU du chemin auto "supporte"
- Windows CUDA prioritaire pour NVIDIA
- etude Linux CUDA specifique

### A ne pas faire tout de suite

- Linux CUDA maintenu par nous sans benchmark montrant un vrai gain produit
- support officiel CPU pour la feature mood
- investissement Intel/SYCL prioritaire avant Vulkan

---

## Sources a re-verifier au moment de l'implementation

- llama.cpp README:
  - https://github.com/ggml-org/llama.cpp
- llama.cpp releases:
  - https://github.com/ggml-org/llama.cpp/releases
- llama.cpp feature matrix:
  - https://github.com/ggml-org/llama.cpp/wiki/Feature-matrix

Les releases/backend packages evoluent vite. Revalider l'etat exact au moment de l'implementation.
