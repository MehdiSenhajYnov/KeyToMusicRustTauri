# Comparatif BL/1

Comparatif entre:

- `t4b_wide5_selective` = precedente meilleure methode BL/1 `wide-5 + selective repair`
- `t4b_wide5_selective_hold` = meme methode BL/1 + **action bridge hold** zero-cost

Sources:

- `manga-mood-ai/results/realtest_suite_t4b_wide5_selective.json`
- `manga-mood-ai/results/realtest_suite_t4b_wide5_selective_hold.json`
- `manga-mood-ai/results/realtest_suite_bl_1.json`

## Resultats

| Methode | Strict | Relaxed | Intensity | Delta strict | Delta relaxed | Delta intensity | Temps moyen | Second pass |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `t4b_wide5_selective` | `56/70` | `68/70` | `24/70` | `+0` | `+0` | `+0` | `11.8s/page` | `12` |
| `t4b_wide5_selective_hold` | `58/70` | `68/70` | `24/70` | `+2` | `+0` | `+0` | `11.9s/page` | `12` |

## Hypothese testee

La precedente meilleure methode BL/1 coupait parfois l'OST `epic` trop tot quand un court run `tension` etait en fait un **pont narratif** entre deux runs `epic`.

## Changement implemente

- pas de nouveau modele
- pas de nouveau reprompt
- pas de modification du backbone `wide-5`
- ajout d'une regle sequentielle zero-cost:
  - si un run `tension` de longueur `4-6` est encadre par `epic` avant et `epic` apres
  - avec un run `epic` court avant (`<=3`) et un rebound `epic` confirme apres (`>=4`)
  - alors on garde `epic` sur les `2` premieres pages du run `tension`

## Lecture rapide

- le gain est **strictement algorithmique**
- le budget de calcul ne bouge pas: toujours `12` reprompts
- le gain complet vient de **2 pages** corrigees: `13` et `14`
- c'est une vraie nouvelle reference BL/1 selon le gate courant: `58/70 strict`, soit `+2` sur l'incumbent
