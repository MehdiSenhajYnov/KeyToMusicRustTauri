# MoodAI Qwen Student Plan

> **Categorie:** Amelioration / IA
> **Priorite:** Haute
> **Statut:** ⏳ A faire
> **Date ajoutee:** 2026-03-28

## Description

Mettre en place un student model local pour MoodAI afin de reduire fortement la latence, la pression GPU et la lourdeur globale du runtime, tout en conservant une qualite proche du pipeline actuel.

Le student retenu pour la premiere phase est `Qwen3-VL-2B-Instruct`.

Le but n'est pas de refaire toute l'architecture MoodAI. Le but est de remplacer la partie inference la plus couteuse par un modele plus petit, tout en gardant les couches logiques utiles deja presentes dans le backend.

## Pourquoi ce plan

Le pipeline actuel donne de bons resultats produit, mais il est trop lourd pour un usage confortable sur une machine qui fait autre chose en parallele.

Constats actuels :

- Le modele runtime actuel est `Qwen3-VL-4B-Thinking`.
- Le pipeline complet actuel repose sur `wide-5 + selective repairs + action hold`.
- La couche `OST committed mood` est une couche logique supplementaire pour le playback.
- Sur RTX 4070 12 GB, le mode actuel peut saturer le GPU et degrader fortement la cohabitation avec d'autres logiciels lourds.

Le choix `Qwen3-VL-2B-Instruct` est retenu pour la premiere iteration parce que :

- c'est un modele officiel Qwen recent ;
- il reste dans la meme famille que le teacher actuel ;
- il est plus petit que le 4B ;
- il existe en format adapte au runtime local actuel ;
- il est plus logique qu'un modele "Thinking" pour une sortie courte et contrainte.

## Objectif produit

Obtenir un mode MoodAI local qui :

- garde la meme experience utilisateur sur les cas normaux ;
- evite les erreurs graves qui ruineraient l'experience ;
- baisse nettement le temps d'analyse ;
- baisse nettement la pression VRAM ;
- cohabite mieux avec le reste des applications du PC ;
- reste compatible avec l'architecture actuelle Tauri + llama-server + pipeline backend.

## Hors scope initial

Cette premiere phase ne cherche pas a :

- optimiser le mobile ;
- remplacer la couche `OST committed mood` par du ML ;
- supprimer tout fallback vers le teacher ;
- re-ouvrir tout de suite la question d'un backbone purement classifier type `MobileNetV4-Conv-M`.

Ces sujets pourront etre traites dans une phase 2 ou 3.

## Cible exacte du student

La cible principale du student ne doit pas etre :

- `teacher seul`
- ni `teacher + pipeline + OST committed`

La bonne cible principale est :

- `teacher + pipeline`

Concretement, le student doit apprendre a predire la sortie finale par page qui sort du pipeline backend avant la couche `OST committed mood`.

Pourquoi :

- une partie importante de la qualite actuelle vient du pipeline, pas uniquement du backbone ;
- la couche `OST committed mood` est legere, deterministe, reglable et stateful ;
- il vaut mieux conserver la logique de playback comme couche explicite, plutot que l'absorber dans le modele.

Conclusion :

- cible d'entrainement principale : `prediction finale par page apres pipeline`
- cible secondaire de debug/eval : `output OST committed mood`

## Couche OST committed mood

La couche `OST committed mood` doit rester hors du student pendant la phase 1.

Strategie retenue :

- le student remplace la prediction de page couteuse ;
- le backend conserve `SoundtrackCommitter` tel quel ;
- les tests comparent aussi l'output committe, mais le student n'est pas entraine directement dessus dans la v1.

On garde cependant cette sortie dans le dataset pour :

- mesurer la proximite comportementale avec le systeme final ;
- faciliter un futur mini post-modele si un jour c'est necessaire.

## Choix du type d'entrainement

Pour cette phase, le bon choix est :

- `LoRA` en priorite
- `QLoRA` si necessaire pour tenir proprement sur la 4070

Ce que l'on ne fait pas en premiere intention :

- full fine-tune complet du modele

Pourquoi `LoRA` :

- plus leger en VRAM ;
- plus simple a iterer ;
- plus simple a checkpoint/reprendre ;
- plus adapte a une premiere distillation sur RTX 4070 ;
- assez souple pour une tache specialisee comme MoodAI.

Decision :

- v1 = `LoRA/QLoRA`
- full fine-tune seulement si la qualite plafonne clairement apres plusieurs iterations de dataset et de tuning.

## Dataset : ce qu'il faut vraiment construire

Le benchmark `RealTest` ne doit pas etre le dataset principal d'entrainement.

Il doit rester en priorite :

- benchmark ;
- jeu de validation ;
- garde-fou produit.

Le dataset d'entrainement doit etre genere automatiquement a partir du teacher actuel.

### Taille cible

Ordres de grandeur retenus :

- `1k` pages : smoke test uniquement
- `5k a 10k` pages : pilot utile
- `20k a 50k` pages : v1 serieuse

Decision recommandee :

- pilot initial : `5k a 10k`
- v1 credible : `20k+`

### Pourquoi RealTest seul ne suffit pas

RealTest est trop petit et trop expose au risque d'overfit comme dataset principal.

Il reste utile pour :

- comparer teacher et student ;
- verifier les regressions graves ;
- valider la generalisation entre series.

### Format recommande d'un sample

Chaque sample doit contenir au minimum :

- `series`
- `chapter`
- `page_index`
- `page_prev_path`
- `page_current_path`
- `page_next_path`
- `teacher_raw_window_output`
- `pipeline_final_mood`
- `pipeline_final_intensity`
- `pipeline_scores`
- `pipeline_narrative_role`
- `repair_note` si present
- `ost_committed_mood`
- `ost_committed_intensity`
- `teacher_confidence_proxy`

Optionnel mais utile :

- `wide5_member_paths`
- `source_runtime`
- `teacher_model_version`
- `created_at`

## Forme exacte des exemples d'entrainement

La v1 doit rester simple.

Approche recommandee :

- entree visuelle principale : `prev / current / next`
- cible : `mood + intensity` de la page centrale apres pipeline

Raison :

- c'est plus simple et plus stable que d'imiter tout le `wide-5` brut des le debut ;
- cela preserve deja une bonne partie du contexte narratif local ;
- cela reduit le cout du training par sample ;
- cela reste proche de cas d'usage utilisables en runtime.

On peut garder une variante v2 si besoin :

- entree `wide-5`
- ou melange `triad + subset hard cases wide-5`

## Strategie de generation automatique des labels

Le teacher labeler doit etre un pipeline batch separable et resumable.

Principe :

- on prend un corpus d'images manga ;
- on fait tourner le pipeline actuel complet ;
- on sauvegarde les sorties finales page par page ;
- on en derive ensuite les samples d'entrainement.

Le labeler doit sauvegarder :

- sortie teacher brute ;
- sortie pipeline finale ;
- sortie OST committed ;
- metadonnees de confiance ;
- statut du job.

## Reprise / interruption

Le labelling et l'entrainement doivent etre reprenables.

Exigences :

- checkpoint par page ou par chapitre ;
- manifest des pages deja traitees ;
- ecriture atomique des fichiers ;
- possibilite de reprendre sans recalculer les pages deja terminees ;
- checkpoints training standards `checkpoint-*` pour LoRA.

Le pipeline doit supporter :

- stop manuel ;
- reprise apres crash ;
- reprise apres reboot ;
- changement de cadence sans corrompre les sorties.

## Split train / val / test

Le split ne doit pas etre aleatoire page par page.

Il doit etre fait par :

- chapitre au minimum ;
- serie de preference pour une vraie verification de generalisation.

Plan recommande :

- `train` : majorite des chapitres du corpus auto-labelise
- `val` : sous-ensemble separe de chapitres non vus
- `test produit` : `RealTest` garde largement separe

## Evaluation

Les metriques a suivre :

- exactitude `mood`
- exactitude `intensity`
- exactitude `mood + intensity`
- proximite avec les `scores` teacher si exploites
- output final apres passage dans `OST committed mood`
- nombre d'erreurs graves produit

Le critere principal ne doit pas etre uniquement le pourcentage global.

Il faut aussi mesurer :

- erreurs graves sur scenes critiques ;
- transitions ratees ;
- confusion recurrente `tension / mystery / epic` ;
- stabilite du comportement sur plusieurs chapitres consecutifs.

## Criteres de succes v1

Le student v1 sera considere comme reussi si :

- il n'introduit pas d'erreurs graves produit sur le benchmark de reference ;
- il garde une grande proximite avec le teacher sur les cas normaux ;
- il baisse nettement la latence ;
- il baisse nettement la VRAM ;
- il ameliore la cohabitation avec le reste du PC ;
- il s'integre sans rearchitecture lourde.

Objectifs de reference raisonnables :

- qualite percue proche du teacher ;
- latence nettement plus basse ;
- VRAM nettement plus basse ;
- possibilite d'un fallback teacher sur cas ambigus.

## Estimation de gains attendus

Fourchettes de travail pour une v1 reussie :

- latence par page : gain attendu significatif
- VRAM : baisse nette
- cohabitation : meilleure, mais pas parfaite

Important :

- un `2B` reste un VLM ;
- la cohabitation avec des apps normales doit s'ameliorer sensiblement ;
- la cohabitation avec un gros jeu gourmand restera limitee si on garde le meme contrat de calcul.

## Runtime produit v1

La premiere integration runtime ne doit pas chercher a tout remplacer d'un coup.

Plan produit recommande :

- student en premier sur le chemin principal ;
- fallback teacher si confiance basse ou cas ambigu ;
- `OST committed mood` conserve tel quel ;
- pipeline complet teacher reserve aux cas difficiles ou au mode idle.

Ordre de priorite :

- page visible d'abord ;
- contexte proche ensuite ;
- background opportuniste seulement si la machine est disponible.

## Cohabitation avec le PC

Le student Qwen doit ameliorer la cohabitation, mais ne suffira probablement pas a rendre MoodAI "invisible" a cote d'un gros jeu.

Pour la v1, il faut prevoir un mode de fonctionnement explicite :

- `Normal` : student + pipeline habituel
- `Cohabitation` : page visible prioritaire, backlog limite, pas de travail inutile
- `Idle / Premium` : teacher complet autorise quand la machine est libre

Important :

- retarder tout le calcul ne suffit pas ;
- pour vraiment cohabiter, il faut aussi savoir jeter ou ne jamais lancer certaines taches de fond.

## Roadmap proposee

### Phase 0 - Preparation

- [ ] Geler le benchmark de reference actuel
- [ ] Documenter proprement la version teacher retenue
- [ ] Definir le schema de dataset
- [ ] Definir les metriques de succes

### Phase 1 - Export teacher

- [ ] Ecrire un export batch resumable du pipeline actuel
- [ ] Sauvegarder les outputs `teacher brut`, `pipeline final`, `OST committed`
- [ ] Produire un premier corpus `5k a 10k`
- [ ] Verifier la qualite et la diversite du corpus

### Phase 2 - Training pilot

- [ ] Monter un premier training `LoRA/QLoRA` sur `Qwen3-VL-2B-Instruct`
- [ ] Evaluer sur validation separee
- [ ] Evaluer sur `RealTest`
- [ ] Identifier les erreurs graves et les confusions majeures

### Phase 3 - V1 serieuse

- [ ] Etendre le corpus a `20k+`
- [ ] Re-entrainer avec settings valides
- [ ] Ajouter fallback teacher sur cas ambigus
- [ ] Integrer au runtime local

### Phase 4 - Mode cohabitation

- [ ] Ajouter un mode `Cohabitation`
- [ ] Prioriser strictement la page visible
- [ ] Limiter ou jeter le backlog inutile
- [ ] Mesurer l'impact a cote d'autres apps lourdes

### Phase 5 - Evaluation produit finale

- [ ] Verifier absence d'erreurs graves
- [ ] Mesurer la latence reelle en usage app
- [ ] Mesurer VRAM reelle
- [ ] Valider la qualite audio finale apres `OST committed mood`

## Estimation de temps machine

Ordres de grandeur de travail sur RTX 4070 pour ce plan :

- labelling `RealTest` seul : quelques heures
- labelling `5k a 10k` pages : de l'ordre de 1 a 2 jours
- labelling `20k+` pages : plusieurs jours
- entrainement pilot LoRA : de l'ordre de plusieurs heures a environ 1 jour
- entrainement v1 plus large : de l'ordre de 1 a 3 jours selon settings

Le pipeline doit donc etre pense des le depart comme :

- batch ;
- resumable ;
- checkpointable ;
- inspectable.

## Risques

- le student copie aussi les erreurs du teacher ;
- un dataset trop petit donne un faux sentiment de succes ;
- un dataset trop redondant degrade la generalisation ;
- le student peut etre bon en score global mais mauvais sur des scenes critiques ;
- le gain de cohabitation peut etre bon avec des apps normales mais insuffisant avec des jeux lourds ;
- si la confiance student est mal calibree, le fallback teacher sera mal exploite.

## Garde-fous

- garder `RealTest` comme benchmark strict et stable ;
- verifier explicitement les erreurs graves ;
- conserver un fallback teacher ;
- ne pas integrer directement `OST committed mood` dans le modele ;
- ne pas lancer de full fine-tune avant d'avoir epuise les iterations LoRA utiles.

## Decision finale retenue

Pour la phase 1, la direction officielle du projet est :

- student principal : `Qwen3-VL-2B-Instruct`
- methode d'entrainement : `LoRA/QLoRA`
- cible principale : `teacher + pipeline`
- `OST committed mood` conserve comme couche logique hors modele
- `RealTest` reserve principalement a l'evaluation
- corpus auto-labelise cible : `5k a 10k` pour le pilot, puis `20k+` pour une vraie v1

## Notes

Ce plan privilegie un chemin pragmatique :

- minimum de rupture dans l'architecture actuelle ;
- gain rapide sur la lourdeur du runtime ;
- evaluation propre avant toute bifurcation vers un backbone classifier mobile-first.

Si la v1 Qwen 2B ne suffit pas en cohabitation, la piste suivante a ouvrir est :

- un student classifier beaucoup plus leger, potentiellement base sur `MobileNetV4-Conv-M`, avec une logique runtime separee.
