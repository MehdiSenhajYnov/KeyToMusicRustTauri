# Documentation Audit

> Audit realise le 2026-03-08.
>
> Objectif: distinguer clairement la documentation canonique, la documentation de travail, les notes historiques et les artefacts de benchmark.

## Regles

- `Current`: doc a jour et utilisable comme source de verite pour le produit actuel
- `Current index`: doc a jour qui pointe vers les bonnes sources
- `Historical`: utile pour comprendre l'historique, mais pas pour decrire le produit actuel
- `Task log`: suivi de livraison ou backlog; pas une spec produit
- `Artifact`: resultat genere ou benchmark conserve pour reference
- `Archive`: documentation volontairement sortie du chemin actif

## Canonical Current Docs

| Path | Status | Role | Notes |
|---|---|---|---|
| [AGENTS.md](/home/mehdi/Dev/KeyToMusicRustTauri/AGENTS.md) | Current tooling doc | repository operating rules | Instruction doc for agents/contributors, not product documentation. |
| [README.md](/home/mehdi/Dev/KeyToMusicRustTauri/README.md) | Current | entree principale du repo | Pointe vers les bonnes docs canoniques. |
| [docs/MANGA_MOOD_CURRENT_ARCHITECTURE.md](/home/mehdi/Dev/KeyToMusicRustTauri/docs/MANGA_MOOD_CURRENT_ARCHITECTURE.md) | Current | source de verite du systeme Manga Mood | Doc produit canonique pour extension + backend + runtime. |
| [WebExtension/manga-mood/README.md](/home/mehdi/Dev/KeyToMusicRustTauri/WebExtension/manga-mood/README.md) | Current | doc operatoire de l'extension | Installation, flux actuel, debug popup. |
| [CLAUDE.md](/home/mehdi/Dev/KeyToMusicRustTauri/CLAUDE.md) | Current | reference agent/outillage | Mise a jour sur le sous-systeme mood; reste orientee assistance code. |
| [docs/KeyToMusic_Technical_Specification.md](/home/mehdi/Dev/KeyToMusicRustTauri/docs/KeyToMusic_Technical_Specification.md) | Current index | spec large de l'application | Garde la spec globale et renvoie vers la doc canonique mood pour ce sous-systeme. |
| [Tasks/README.md](/home/mehdi/Dev/KeyToMusicRustTauri/Tasks/README.md) | Current index | index backlog/livraison | Corrige les liens et clarifie ce qui est actif vs historique. |

## Mood Research And Planning

| Path | Status | Role | Notes |
|---|---|---|---|
| [manga-mood-ai/plans/IMPLEMENTATION.md](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/plans/IMPLEMENTATION.md) | Current reference + historical context | spec benchmark/runtime mood | Contient l'etat technique courant, mais reste plus labo que doc produit. |
| [manga-mood-ai/research/RESEARCH_SYNTHESIS.md](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/research/RESEARCH_SYNTHESIS.md) | Current research index | synthese benchmark | Source de verite benchmark, pas spec API produit. |
| [manga-mood-ai/plans/PLAN_RUNTIME_SIZING_NGL_SUPPORT.md](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/plans/PLAN_RUNTIME_SIZING_NGL_SUPPORT.md) | Current/historical mixed | sizing runtime | Toujours utile car il documente une implementation deja activee. |
| [manga-mood-ai/plans/PLAN_GPU_BACKENDS_RUNTIME.md](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/plans/PLAN_GPU_BACKENDS_RUNTIME.md) | Historical future plan | plan runtime futur | Pas encore la source de verite produit. |
| [manga-mood-ai/plans/PLAN_MOOD_DIMENSIONAL.md](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/plans/PLAN_MOOD_DIMENSIONAL.md) | Historical migration plan | plan de migration 8 moods | Utile pour l'historique de schema. |
| [manga-mood-ai/plans/PLAN.md](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/plans/PLAN.md) | Historical | ancien plan V6 | A garder comme contexte uniquement. |
| [manga-mood-ai/plans/PIPELINE_V2.md](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/plans/PIPELINE_V2.md) | Historical | ancien pipeline describe/classify | Ne decrit plus le produit actuel. |
| [manga-mood-ai/plans/PLAN_CONTEXT_DESCRIPTIONS.md](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/plans/PLAN_CONTEXT_DESCRIPTIONS.md) | Historical | plan V6 contextuel | A garder comme trace de recherche. |
| [manga-mood-ai/plans/PLAN_V7_BIDIRECTIONAL.md](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/plans/PLAN_V7_BIDIRECTIONAL.md) | Historical | variante benchmark | Historique. |
| [manga-mood-ai/plans/PLAN_V8_ASYMMETRIC.md](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/plans/PLAN_V8_ASYMMETRIC.md) | Historical | variante benchmark | Historique. |
| [manga-mood-ai/research/FINDINGS.md](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/research/FINDINGS.md) | Historical research | compte rendu benchmark | A ne pas lire comme spec produit. |
| [manga-mood-ai/research/RESULTS.md](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/research/RESULTS.md) | Historical research | resultats anciens benchmark | Historiques 10 moods / premieres passes. |
| [manga-mood-ai/research/RechercheAmeliorationIA.md](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/research/RechercheAmeliorationIA.md) | Historical research | revue de litterature | Source idees futures, pas doc produit. |
| [manga-mood-ai/research/NEW/1.md](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/research/NEW/1.md) | Historical research note | note exploratoire | Conserver comme brouillon de recherche. |
| [manga-mood-ai/research/NEW/2.md](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/research/NEW/2.md) | Historical research note | note exploratoire | Conserver comme brouillon de recherche. |

## Benchmark Artifacts

| Path Pattern | Status | Role | Notes |
|---|---|---|---|
| `manga-mood-ai/results/realtest_suite_bl_1.json` | Artifact | root summary alias | Resume convenient du BL/1 principal; doit rester coherent avec `manga-mood-ai/results/realtests/realtest_suite_bl_1.json`. |
| `manga-mood-ai/results/realtest_suite_bl_1.md` | Artifact | root summary alias | Meme role que le JSON racine, version lisible. |
| `manga-mood-ai/results/*.md` | Artifact | benchmark exported summaries | A lire comme sorties de run, jamais comme spec stable. |
| `manga-mood-ai/results/comparisons/*.md` | Artifact | comparaisons benchmark | Sorties derivees. |
| `manga-mood-ai/results/realtests/*.md` | Artifact | subsets benchmark | Sorties derivees. |

## Tasks And Delivery Logs

| Path Pattern | Status | Role | Notes |
|---|---|---|---|
| `Tasks/todo/*.md` | Task log | backlog actif | Docs de travail, pas spec finale. |
| `Tasks/post-dev/*.md` | Task log | checklist post-dev | Toujours actives si non executees. |
| `Tasks/ideas/*.md` | Task log | idees / parking lot | Non engagees. |
| `Tasks/done/*.md` | Historical task log | livraisons terminees | Gardees comme historique de livraison. |
| `Tasks/done/features/**/*.md` | Historical task log | comptes rendus de features | Pas une source de verite produit. |
| `Tasks/done/fixes/**/*.md` | Historical task log | comptes rendus de fixes | Pas une source de verite produit. |
| `Tasks/done/infrastructure/*.md` | Historical task log | historique setup/perf | Historique. |

## Resources And Packaging Docs

| Path | Status | Role | Notes |
|---|---|---|---|
| [resources/icons/README.md](/home/mehdi/Dev/KeyToMusicRustTauri/resources/icons/README.md) | Current | packaging resources | Toujours valide. |
| [resources/sounds/README.md](/home/mehdi/Dev/KeyToMusicRustTauri/resources/sounds/README.md) | Current | packaging resources | Toujours valide. |

## Archived Docs

| Path Pattern | Status | Role | Notes |
|---|---|---|---|
| `docs/archive/*.md` | Archive | docs retirees | A ne pas utiliser pour piloter le code actuel. |

## Practical Reading Order

Si tu veux comprendre l'etat actuel du projet sans te perdre :

1. [README.md](/home/mehdi/Dev/KeyToMusicRustTauri/README.md)
2. [docs/MANGA_MOOD_CURRENT_ARCHITECTURE.md](/home/mehdi/Dev/KeyToMusicRustTauri/docs/MANGA_MOOD_CURRENT_ARCHITECTURE.md)
3. [WebExtension/manga-mood/README.md](/home/mehdi/Dev/KeyToMusicRustTauri/WebExtension/manga-mood/README.md)
4. [docs/KeyToMusic_Technical_Specification.md](/home/mehdi/Dev/KeyToMusicRustTauri/docs/KeyToMusic_Technical_Specification.md)
5. [manga-mood-ai/plans/IMPLEMENTATION.md](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/plans/IMPLEMENTATION.md) si besoin des details benchmark/runtime
