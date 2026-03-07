# Comparatif BL/1

Comparatif entre:

- `baseline_cache` = baseline historique RealTest `BL/1`
- `t4b_wide5_selective` = nouvelle methode `wide-5 + selective repair`

Sources:

- `manga-mood-ai/results/realtest_suite_bl_1.json`
- `manga-mood-ai/results/realtest_suite_bl_1.md`

## Resultats

| Methode | Modele | Decision | Strict | Relaxed | Intensity | Delta strict | Delta relaxed | Delta intensity | Temps moyen |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|
| `baseline_cache` | `Qwen3-VL-4B-Thinking` | `majority` | `47/70` | `60/70` | `21/70` | `+0` | `+0` | `+0` | cache only |
| `t4b_wide5_selective` | `Qwen3-VL-4B-Thinking` | `wide5_selective_repair` | `56/70` | `68/70` | `24/70` | `+9` | `+8` | `+3` | `13.0s/page` |

## Lecture rapide

- gain principal: `+9` en strict
- gain secondaire: `+8` en relaxed
- gain faible mais positif en intensite: `+3`
- la nouvelle methode reste sur le meme backbone (`Qwen3-VL-4B-Thinking`)
- l'amelioration vient de la logique algorithmique, pas d'un changement de modele

## Notes importantes

- Le baseline dans la suite est charge depuis cache, donc son `avg_window_s` apparait a `0.0` dans le tableau brut.
- La nouvelle methode a reprompt `12` pages sur `74` pages de chapitre, pour corriger certains bords de runs.
- Rerun complet apres les changements runtime du 2026-03-07: score stable sur la nouvelle annotation (`56/70`, `68/70`, `24/70`), mais temps moyen un peu plus lent (`13.0s/page` au lieu d'environ `11.1s/page` sur le run precedent).
- Si tu veux relire la sortie agregée complete du run `BL/1`, le bon fichier est `manga-mood-ai/results/realtest_suite_bl_1.md`.
- Les fichiers `realtest_suite_bl_1_first1.*` et `realtest_suite_bl_1_first3.*` sont seulement des smoke tests limites en nombre de pages.
