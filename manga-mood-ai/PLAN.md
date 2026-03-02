# Manga Mood AI — Plan de test

## Objectif

Tester et comparer plusieurs modèles de vision IA en local pour déterminer lequel est capable de **classifier le mood/ambiance d'une page de manga ou webtoon** de manière fiable. Ce sous-projet est un banc d'essai isolé, avant intégration dans KeyToMusic.

Le but final (hors scope ici) : coupler KeyToMusic avec une extension navigateur qui scrape les pages manga/webtoon en cours de lecture, les envoie à une IA locale, qui analyse l'ambiance et déclenche automatiquement l'OST adaptée.

---

## Setup machine de référence

- **High-end (dev):** RTX 4070 (12 GB VRAM), 32 GB RAM, Ryzen 5 7600
- **Mid-range (cible):** GPU intégré ou GTX 1650, 8-16 GB RAM
- **Low-end (minimum):** Pas de GPU dédié, 4-8 GB RAM, CPU ancien

---

## Modèles à tester

### 4 approches, du plus léger au plus lourd :

| # | Modèle | Type | Taille | RAM/VRAM | Cible | Vitesse attendue |
|---|--------|------|--------|----------|-------|-----------------|
| 1 | **SigLIP 2 ViT-B** | Embedding + classification | 86 Mo | ~200 MB | Tous PC | <100ms (CPU) |
| 2 | **Moondream 0.5B** | VLM (génératif) | ~400 Mo | ~1 GB | Low-end | ~1-2s (CPU) |
| 3 | **Qwen3-VL-2B** | VLM (génératif) | ~1.5 GB (Q4) | ~2-3 GB | Mid-range | ~2-3s (CPU), <1s (GPU) |
| 4 | **Qwen2.5-VL-7B** | VLM (génératif) | ~4.4 GB (Q4) | ~6 GB | High-end (RTX 4070) | ~1-2s (GPU), ~5-8s (CPU) |

---

## Comment tester

### Prérequis installés
- [x] Python 3.11 + venv (`manga-mood-ai/venv/`)
- [x] Ollama 0.17.4
- [x] PyTorch CUDA 12.1 + transformers + ollama SDK
- [x] Moondream 0.5B (Ollama)
- [ ] Qwen3-VL 2B (Ollama) — en cours de téléchargement
- [ ] Qwen2.5-VL 7B (Ollama) — en cours de téléchargement

### Mettre des images de test
```
manga-mood-ai/test-images/manga/    ← pages manga noir & blanc
manga-mood-ai/test-images/webtoon/  ← pages webtoon couleur
```
Formats acceptés : .jpg, .jpeg, .png, .webp

### Lancer les tests
```bash
cd manga-mood-ai
./venv/Scripts/activate

# Tout tester (les 4 modèles sur toutes les images)
python test_models.py

# Un modèle spécifique
python test_models.py --model siglip
python test_models.py --model moondream
python test_models.py --model qwen2b
python test_models.py --model qwen7b

# Une image spécifique
python test_models.py --image test-images/manga/battle.jpg

# Multi-image context (envoie N pages précédentes comme contexte aux VLMs)
python test_models.py --model qwen7b --context 3

# Prompt custom (pour expérimenter)
python test_models.py --model qwen7b --custom-prompt "Describe the emotions in this scene"
```

### Résultats
- Affichage rich dans le terminal (tableau comparatif)
- Sauvegarde JSON dans `results/benchmark.json`

---

## Ce qu'on évalue

| Critère | Question |
|---------|----------|
| **Accuracy** | Est-ce que le mood détecté correspond à ce qu'on voit ? |
| **Latence** | Combien de temps par image ? Acceptable pour du temps réel ? |
| **Cohérence** | Même image = même résultat à chaque run ? |
| **Manga vs Webtoon** | Aussi bon en N&B qu'en couleur ? |
| **Multi-image** | Le contexte des pages précédentes améliore-t-il le résultat ? |

---

## Hors scope

- Extension navigateur, intégration KeyToMusic, Rust, llama.cpp, fine-tuning
- On juge juste : **"Est-ce qu'un modèle local peut classifier le mood d'une page manga/webtoon ?"**
