# Recherche exhaustive — Classification de mood narratif en manga
## KeyToMusic : résoudre le problème de la dépendance contextuelle

> **Mise a jour (mars 2026) :** cette revue de litterature reste utile pour les pistes futures, mais elle n'est ni la source de verite benchmark ni la spec produit. Consulter [RESEARCH_SYNTHESIS.md](./RESEARCH_SYNTHESIS.md) pour l'etat benchmark courant et [../../docs/MANGA_MOOD_CURRENT_ARCHITECTURE.md](/home/mehdi/Dev/KeyToMusicRustTauri/docs/MANGA_MOOD_CURRENT_ARCHITECTURE.md) pour le flux actuel.

---

## Synthèse exécutive

Ton problème central — le VLM confond l'intensité visuelle avec la catégorie narrative et ne peut pas détecter les inversions sémantiques (rêve/flashback) — est un problème **ouvert** en recherche. Mais plusieurs pistes convergent vers une solution viable. La piste la plus prometteuse à court terme est un **pipeline hybride VLM + CRF** avec détection explicite de marqueurs narratifs. À moyen terme, le **fine-tuning LoRA de Qwen2.5-VL** sur un dataset annoté manga-mood est probablement ton meilleur investissement. À long terme, l'architecture **MangaLMM** (VLM spécialisé manga fine-tuné sur Qwen2.5-VL) et les travaux de M2M-Gen sur la musique de fond manga ouvrent des pistes directement applicables.

**Classement par rapport effort/impact :**

| Rang | Piste | Effort | Impact estimé | Priorité |
|------|-------|--------|---------------|----------|
| 1 | VLM + CRF linéaire (séquence de moods) | 2-3 semaines | +10-15% strict | **Immédiat** |
| 2 | Détection marqueurs narratifs (flashback/rêve) | 2-4 semaines | +8-12% strict | **Immédiat** |
| 3 | Fine-tuning LoRA Qwen2.5-VL sur mood manga | 3-6 semaines | +15-25% strict | **Court terme** |
| 4 | Pipeline M2M-Gen adapté (scene segmentation + emotion) | 2-3 semaines | +10-15% strict | **Court terme** |
| 5 | OCR + analyse sentiment dialogue | 1-2 semaines | +5-8% strict | **Complément** |
| 6 | BiLSTM/Transformer sur embeddings visuels | 4-6 semaines | +12-18% strict | **Moyen terme** |
| 7 | CLIP contrastif manga-mood | 6-8 semaines | +15-20% strict | **Moyen terme** |
| 8 | Diffusion features comme représentation | 2-4 semaines | +5-10% strict | **Expérimental** |
| 9 | RL avec feedback utilisateur | 4-8 semaines | +5-15% strict | **Long terme** |
| 10 | GNN sur graphe narratif | 8+ semaines | Inconnu | **Recherche** |

---

## Piste 1 — CRF linéaire sur la séquence de moods (Modélisation séquentielle)

### Description

Au lieu de classifier chaque page indépendamment, modéliser la séquence de pages comme un problème de **sequence labeling** avec un Conditional Random Field (CRF). Le CRF prend en entrée les logits/probabilités du VLM pour chaque page et les corrige en tenant compte des transitions entre moods successifs.

### Fondements

Les CRF sont le gold standard pour le sequence labeling dans des domaines comme le NER, le POS tagging, et l'activité humaine. Leur force est de modéliser les **probabilités de transition** entre labels successifs — exactement ce dont tu as besoin. Par exemple, un CRF apprendra que `sadness → sadness` est bien plus probable que `sadness → comedy → sadness` sur 3 pages consécutives. Il apprendra aussi que `emotional_climax` ne peut pas apparaître 5 pages d'affilée — c'est un pic ponctuel.

Les Neural CRF Transducers (Hu et al., 2018, arXiv:1811.01382) combinent deux RNN : un pour les observations (features VLM) et un pour les dépendances entre labels, capturant des dépendances long-range théoriquement infinies. Les travaux sur la Recurrent Temporal Deep Field (RTDF) combinent CRF + DeconvNet + RTRBM pour le labeling vidéo sémantique — exactement ton analogue dans le domaine vidéo.

### Architecture concrète

```
Page_1 → VLM → logits_1 ─┐
Page_2 → VLM → logits_2 ──┤── CRF → mood_1, mood_2, ..., mood_N (séquence corrigée)
Page_3 → VLM → logits_3 ──┤
...                        ┘
```

**Implémentation :**

1. **Feature extraction** : Pour chaque page, extraire un vecteur de features. Le plus simple : demander au VLM un score de confiance pour chaque mood (10 valeurs), ou extraire les logits de la dernière couche. Avec Qwen2.5-VL via llama-server, tu peux forcer la génération de JSON structuré avec scores.

2. **CRF layer** : Utiliser `pytorch-crf` ou `sklearn-crfsuite`. Avec pytorch-crf, c'est un module PyTorch standard qu'on branche sur les emissions du VLM.

3. **Training** : Tu as besoin de séquences annotées. Ton benchmark Blue Lock (31 pages) est un début. Il faudra 5-10 tomes annotés (150-300 pages) pour entraîner un CRF robuste. Les paramètres du CRF sont peu nombreux (matrice de transition 10×10 = 100 paramètres), donc peu de données suffisent.

4. **Features supplémentaires pour le CRF** :
   - Position relative dans le chapitre (début/milieu/fin)
   - Densité de texte sur la page (beaucoup de bulles = dialogue = comedy/peaceful, peu de texte = action/emotional)
   - Changement visuel brutal entre pages consécutives (indice de changement de scène)

### Estimation d'effort
- **Temps** : 2-3 semaines d'implémentation
- **Données** : 5-10 séquences annotées (150-300 pages) minimum
- **Difficulté** : Modérée — les bibliothèques CRF existent

### Impact attendu
+10-15% en accuracy strict. Le CRF éliminera les oscillations aberrantes (comedy→epic_battle→comedy en 3 pages) et lissera les prédictions. Il corrigera partiellement les confusions intensité/catégorie car `emotional_climax` après 5 pages de `sadness` sera re-labelé en `sadness`.

### Risques
- Le CRF ne résoudra pas les inversions sémantiques (rêve/flashback) car il n'a pas accès au contenu visuel lui-même, seulement aux logits. Il lisse mais ne comprend pas.
- Si le VLM se trompe systématiquement sur certaines catégories, le CRF ne corrigera pas — il amplifiera la tendance.
- Le CRF est un modèle **linéaire** sur les transitions. Pour des patterns non-linéaires complexes, il faut passer au BiLSTM (voir Piste 6).

---

## Piste 2 — Détection explicite de marqueurs narratifs (Flashback/Rêve/Monologue)

### Description

Classifier **d'abord** si une page contient des marqueurs narratifs de contexte (flashback, rêve, monologue intérieur, imagination), puis utiliser cette méta-information pour ajuster la classification de mood.

### Fondements

Le manga utilise des **codes visuels conventionnels** pour signaler les flashbacks et rêves :
- **Bordures floues ou arrondies** (vs rectangulaires pour le présent)
- **Effets de halo/particules/étoiles** autour des personnages
- **Tons sépia ou dégradés** (niveaux de gris différents du style normal)
- **Bulles de pensée** (nuageuses) vs bulles de dialogue (ovales)
- **Absence de bordure de panel** (page entière = splash page rêvée)
- **Onomatopées spécifiques** ou absence d'onomatopées

Le dataset KangaiSet (Théodose & Burie, 2023) travaille sur la reconnaissance d'émotions dans les mangas à l'échelle des visages, et note que les "background effects" et "symbols" graphiques sont des indices critiques pour le mood. Le paper sur l'emotion recognition dans les comics (Sharma & Kukreja, 2025) atteint 92.6% en utilisant VGG16 + attention, démontrant que les patterns visuels de comics sont apprenables.

Le dataset **Manga109** (21,142 pages, 109 volumes) fournit des annotations de panels, bulles, et texte. Le **Manga109 segmentation** récent (CVPR 2025) ajoute des masques pixel-level pour les panels, ballons, texte, visages et corps. L'attribut "style of balloons (normal, cloud, spike...)" est annoté dans eBDtheque — c'est exactement l'indice de monologue intérieur vs dialogue.

### Architecture concrète

**Phase 1 — Classifieur binaire "page contextuelle"**

Un classifieur simple (ResNet-18 ou même le VLM lui-même avec un prompt spécialisé) qui prédit :
- `direct` : la page montre ce qui se passe "maintenant"
- `contextual` : la page est un flashback, rêve, imagination, monologue intérieur

Prompt VLM pour la détection :
```
Look at this manga page. Focus on visual narrative markers:
- Panel borders: are they fuzzy, wavy, or non-rectangular? (flashback/dream indicator)
- Are there ethereal effects like sparkles, halos, soft gradients? (dream/imagination)
- Are thought bubbles (cloud-shaped) present vs speech bubbles (oval)?
- Is the page a full splash with no panel borders? (could be dream sequence)
- Are there visual transition effects (dissolve, fade)?

Classify: DIRECT (showing present events) or CONTEXTUAL (flashback/dream/imagination/memory)
Answer with one word only.
```

**Phase 2 — Ajustement du mood**

Si `contextual` est détecté, inverser ou ajuster la classification de mood :
- Image de victoire + contextual → `sadness` (le personnage rêve de ce qu'il n'aura pas)
- Image d'action intense + contextual → `tension` ou `sadness` (souvenir traumatique)
- Image paisible + contextual → `emotional_climax` (moment nostalgique)

Table de mapping `(visual_mood, context) → actual_mood` apprise sur tes données annotées.

### Estimation d'effort
- **Temps** : 2-4 semaines
- **Données** : 50-100 pages annotées flashback/rêve suffisent pour le classifieur binaire (c'est un problème plus simple que le mood)
- **Difficulté** : Faible à modérée

### Impact attendu
+8-12% strict, **spécifiquement** sur les cas problématiques de ton benchmark (pages 27-29 de Blue Lock). Ce fix cible directement la confusion la plus dommageable.

### Risques
- Les codes visuels ne sont pas universels entre mangas. Un shonen comme Blue Lock et un seinen comme Vagabond utilisent des conventions différentes.
- Certains mangakas inventent leurs propres codes. Le classifieur doit être re-calibré par manga/mangaka.
- Le binary contextual/direct est réducteur — il y a des gradations (flashback partiel, page de transition).

---

## Piste 3 — Fine-tuning LoRA de Qwen2.5-VL pour la classification de mood manga

### Description

Fine-tuner Qwen2.5-VL (3B ou 7B) avec LoRA/QLoRA spécifiquement pour la tâche de classification de mood manga, en incluant le contexte des pages précédentes.

### Fondements

**MangaLMM** (arXiv:2505.20298, mai 2025) est directement pertinent : c'est un modèle fine-tuné depuis Qwen2.5-VL pour la compréhension de manga, incluant OCR et VQA. Les auteurs montrent que le fine-tuning sur des données manga améliore significativement la compréhension contextuelle par rapport aux modèles généralistes (GPT-4o, Gemini 2.5). Ce paper prouve que ton approche est viable.

Les guides pratiques de fine-tuning Qwen2.5-VL (Roboflow, 2U1/Qwen-VL-Series-Finetune sur GitHub) montrent que :
- **LoRA sur 1,200-2,500 exemples** donne des résultats solides sur des domaines spécifiques
- Le **QLoRA 4-bit** tourne sur une RTX 3060 12GB pour le modèle 2B, et sur une RTX 4090 pour le 7B
- Le training prend ~2-3h pour 3 epochs sur ~1,700 samples avec A100 40GB
- Les learning rates recommandés sont autour de 5e-5, avec la vision tower 10x plus basse
- La perte de capacités générales ("catastrophic forgetting") est minimale si on mélange des données générales avec les données spécialisées

**Point crucial** : Un développeur a rapporté que le fine-tuning de Qwen2.5-VL sur 170k images d'un domaine spécifique donne de **meilleures performances que le 72B** sur ce domaine, sans perte de capacité générale.

### Architecture concrète

**Dataset** :

Format LLaVA-style JSON :
```json
{
  "conversations": [
    {
      "role": "user",
      "content": [
        {"type": "image", "image": "page_027.png"},
        {"type": "image", "image": "page_028.png"},
        {"type": "image", "image": "page_029.png"},
        {"type": "text", "text": "These are 3 consecutive manga pages. Classify the narrative mood of the LAST page. Consider the narrative context from previous pages. A page showing triumph/victory in a dream or imagination of a character who has actually lost should be classified as sadness, not epic_battle.\n\nCategories: epic_battle, tension, sadness, comedy, romance, horror, peaceful, emotional_climax, mystery, chase_action\n\nRespond with JSON: {\"mood\": \"...\", \"reasoning\": \"...\"}"}
    ],
    {
      "role": "assistant",
      "content": "{\"mood\": \"sadness\", \"reasoning\": \"The protagonist is imagining winning the World Cup, but the previous pages establish he has just lost. The triumphant imagery serves to amplify his regret and sadness.\"}"
    }
  ]
}
```

**Points clés du dataset** :
- **Multi-image input** : Qwen2.5-VL supporte nativement le multi-image. Envoyer 2-3 pages consécutives pour chaque exemple.
- **Chain-of-thought** : Inclure le raisonnement dans la réponse du modèle pour qu'il apprenne à articuler la logique narrative.
- **Hard examples** : Sur-représenter les cas de flashback/rêve/ironie dans le dataset (data augmentation sur les cas difficiles).
- **Volume** : 500-1000 examples annotés devrait suffire avec LoRA. 200-300 pages ≈ 5-10 chapitres de manga.

**Training** :
```bash
# Avec le repo 2U1/Qwen-VL-Series-Finetune
python train.py \
  --model_id Qwen/Qwen2.5-VL-7B-Instruct \
  --lora_enable True \
  --num_lora_modules -1 \
  --freeze_vision_tower True \
  --per_device_train_batch_size 1 \
  --gradient_accumulation_steps 8 \
  --learning_rate 2e-5 \
  --num_train_epochs 3 \
  --output_dir ./manga_mood_lora
```

Pour la RTX 3060 12GB : utiliser le modèle 3B avec QLoRA 4-bit.
Pour la RTX 4090 24GB : utiliser le 7B avec LoRA ou le 3B en full.

### Estimation d'effort
- **Temps** : 3-6 semaines (annotation 60%, code 20%, training/eval 20%)
- **Données** : 500-1000 examples annotés (10-20 chapitres de manga)
- **Difficulté** : Modérée à élevée (l'annotation est le goulot d'étranglement)
- **Compute** : RTX 4090 suffit pour le 7B en LoRA

### Impact attendu
+15-25% strict. C'est la piste avec le meilleur potentiel car elle résout les deux problèmes en même temps : le modèle apprend les codes visuels manga ET le raisonnement contextuel.

### Risques
- **Généralisation** : Un modèle fine-tuné sur Blue Lock peut ne pas généraliser à Naruto ou One Piece. Il faut diversifier le dataset d'entraînement.
- **Catastrophic forgetting** : Si le dataset est trop petit ou déséquilibré, le modèle peut perdre ses capacités générales. Mixer avec des données VQA générales.
- **Annotation** : L'annotation de mood est subjective. Il faut des guidelines claires et si possible un inter-annotator agreement.

---

## Piste 4 — Pipeline M2M-Gen adapté

### Description

Adapter le pipeline **M2M-Gen** (Sharma et al., arXiv:2410.09928) — le papier le plus directement pertinent pour ton projet — qui génère de la musique de fond pour manga en utilisant la détection de scènes, la classification d'émotions et les LLM.

### Fondements

**M2M-Gen est littéralement ton projet inversé.** Leurs étapes :
1. **Scene boundary detection** via les dialogues (changements de lieu/temps/personnage)
2. **Emotion classification** sur les visages des personnages (fine-tuning de CLIP)
3. **Scene-level music directive** via GPT-4o (prend en compte le contexte de la scène entière)
4. **Page-level music captions** conditionnées sur la directive et le contexte
5. **Text-to-music generation** via un modèle de musique

Leur pipeline résout le problème de cohérence page-à-page en passant par un niveau d'abstraction "scène" : au lieu de classifier chaque page indépendamment, ils détectent d'abord les frontières de scènes, puis assignent une directive émotionnelle à la scène entière, puis la déclinent page par page.

Leur contribution clé : ils fine-tunent CLIP pour la classification émotionnelle sur les visages manga, et montrent que ça améliore significativement la performance par rapport au CLIP vanilla.

### Architecture concrète

Adapter le pipeline M2M-Gen :

1. **Scene segmentation** : Regrouper les pages en scènes en utilisant le changement visuel (différence d'embeddings entre pages consécutives). Seuil de changement → nouvelle scène.

2. **Per-scene analysis** : Pour chaque scène, le VLM analyse la première et la dernière page + les dialogues pour déterminer l'arc émotionnel.

3. **Scene-level mood** : Un LLM texte (Qwen3 8B) reçoit la description de la scène et attribue un mood dominant.

4. **Page-level refinement** : Le mood de chaque page est une variation du mood de la scène, pas une classification indépendante.

Cela élimine les feedback loops car le contexte vient d'un niveau supérieur (la scène), pas des pages précédentes.

### Estimation d'effort
- **Temps** : 2-3 semaines
- **Données** : Tes 31 pages annotées suffisent pour valider
- **Difficulté** : Modérée

### Impact attendu
+10-15% strict. Principalement par élimination des oscillations et meilleure cohérence intra-scène.

### Risques
- La détection de frontières de scènes n'est pas triviale sans les dialogues (le webtoon peut ne pas les avoir).
- Si une scène contient un flashback au milieu, le mood de la scène sera mal calculé.

---

## Piste 5 — OCR manga + analyse de sentiment du dialogue

### Description

Extraire le texte des bulles de dialogue via OCR, puis utiliser l'analyse de sentiment sur le texte comme signal complémentaire.

### Fondements

Le texte des bulles est un signal riche et sous-exploité. Les dialogues intérieurs (bulles nuageuses) vs dialogues parlés (bulles ovales) ont des patterns lexicaux différents. Un personnage qui pense "Si seulement j'avais..." signale de la tristesse, même si l'image montre du triomphe.

Les travaux :
- **Manga109Dialog** : 132,692 paires speaker-texte annotées, le plus grand dataset de ce type
- **MangaOCR** (de MangaLMM) : benchmark OCR spécialisé manga
- Pour le texte japonais : les modèles OCR manga existent (manga-ocr sur HuggingFace)
- Pour le texte français/anglais (scanlations) : Tesseract ou PaddleOCR fonctionnent bien

### Architecture concrète

```
Page → OCR → texte des bulles → sentiment analysis → mood_text
Page → VLM → mood_visual
(mood_text, mood_visual, features_narratives) → classifieur final → mood
```

L'analyse de sentiment du texte peut être faite par un petit LLM local (Qwen3 0.6B suffit) avec un prompt comme :
```
Given this manga dialogue: "{extracted_text}"
What is the emotional tone? Choose: positive, negative, tense, neutral, romantic, comedic
```

### Estimation d'effort
- **Temps** : 1-2 semaines
- **Données** : Pas de données spécifiques nécessaires
- **Difficulté** : Faible

### Impact attendu
+5-8% strict. Le texte est un signal complémentaire, pas un remplacement. Il aide surtout pour les cas où le visuel est ambigu.

### Risques
- L'OCR sur manga est bruité (texte stylisé, SFX, onomatopées)
- Beaucoup de pages d'action n'ont pas de texte
- Les scanlations ont parfois du texte de qualité variable

---

## Piste 6 — BiLSTM/Transformer sur embeddings visuels séquentiels

### Description

Extraire un embedding visuel de chaque page (via le vision encoder de Qwen2.5-VL ou CLIP), puis entraîner un modèle séquentiel (BiLSTM, Transformer, ou Mamba) sur la séquence d'embeddings pour prédire la séquence de moods.

### Fondements

C'est l'analogue direct du **video sentiment analysis**. Les travaux sur CNN+LSTM pour la reconnaissance d'émotions vidéo (Fan et al. 2016, Vielzeuf et al. 2017) montrent que la combinaison features visuelles + modèle temporel surpasse significativement les approches frame-par-frame. La composante temporelle capture les arcs émotionnels, les montées en tension, et les résolutions — exactement la structure narrative d'un manga.

La récente architecture **eMotions** (arXiv:2508.06902, 2025) utilise des dilated residual blocks avec intégration cross-pyramidale sélective pour l'analyse d'émotions vidéo — transposable à ta séquence de pages.

**Mamba/State Space Models** sont une alternative prometteuse aux Transformers pour les séquences longues. Avec O(n) en complexité au lieu de O(n²), ils sont idéaux pour des séquences de 100+ pages (un tome entier).

### Architecture concrète

```python
# Pseudo-code
class MangaMoodSequenceModel(nn.Module):
    def __init__(self):
        self.visual_encoder = load_clip_or_qwen_vision()  # frozen
        self.proj = nn.Linear(visual_dim, 256)
        self.lstm = nn.LSTM(256, 128, bidirectional=True, num_layers=2)
        self.classifier = nn.Linear(256, 10)  # 10 moods
    
    def forward(self, page_images):  # [seq_len, C, H, W]
        embeddings = [self.visual_encoder(img) for img in page_images]
        embeddings = torch.stack([self.proj(e) for e in embeddings])
        lstm_out, _ = self.lstm(embeddings)
        moods = self.classifier(lstm_out)
        return moods  # [seq_len, 10]
```

**Variante Transformer** :
```python
class MoodTransformer(nn.Module):
    def __init__(self):
        self.visual_encoder = load_clip()  # frozen
        self.proj = nn.Linear(768, 256)
        self.pos_encoding = PositionalEncoding(256, max_len=200)
        self.transformer = nn.TransformerEncoder(
            nn.TransformerEncoderLayer(d_model=256, nhead=8), num_layers=4)
        self.classifier = nn.Linear(256, 10)
```

### Estimation d'effort
- **Temps** : 4-6 semaines
- **Données** : 10-20 tomes annotés (500-1000 pages en séquences)
- **Difficulté** : Élevée (besoin de beaucoup de données séquentielles annotées)

### Impact attendu
+12-18% strict. Le modèle séquentiel capture les arcs narratifs et les dépendances long-range.

### Risques
- Besoin de **beaucoup** de données séquentielles annotées (pas juste des pages isolées)
- L'embedding visuel CLIP peut perdre les codes narratifs manga-spécifiques
- Le BiLSTM peut overfitter sur des patterns spécifiques à certains mangas

---

## Piste 7 — CLIP contrastif manga-mood

### Description

Fine-tuner un modèle CLIP (ou SigLIP) en apprentissage contrastif pour aligner les embeddings d'images manga avec des descriptions de mood.

### Fondements

M2M-Gen fine-tune CLIP pour la classification émotionnelle sur les visages manga et montre une amélioration significative. Le paper "Emotion Embedding Spaces for Matching Music to Stories" explore des espaces d'embedding émotionnels pour le matching text-to-music — directement pertinent pour ton use case.

L'idée : entraîner CLIP à mapper une image manga vers le même espace qu'une description textuelle de mood ("Une page de tristesse où le personnage se remémore ce qu'il a perdu" devrait être proche de l'image de Blue Lock page 28).

### Architecture concrète

1. **Paires (image, description_mood)** :
   - Image manga + "This is an epic battle scene with intense action and dynamic combat"
   - Image manga + "This is a sad page where a character reflects on loss through imagined success"

2. **Fine-tuning contrastif** : Adapter CLIP-ViT-B/32 ou SigLIP avec LoRA pour que les embeddings d'images manga soient proches des descriptions de mood correspondantes.

3. **Inférence** : Pour chaque page, calculer la similarité cosine avec les 10 descriptions de mood, prendre le max.

### Estimation d'effort
- **Temps** : 6-8 semaines
- **Données** : 1000-5000 paires (image, description enrichie)
- **Difficulté** : Élevée

### Impact attendu
+15-20% strict. L'embedding contrastif capture la sémantique profonde, pas juste les features visuelles de surface.

### Risques
- Nécessite des descriptions très précises et variées pour chaque mood
- Le CLIP vanilla est mauvais sur le manga (domain gap important)
- Le training contrastif est sensible aux hyperparamètres

---

## Piste 8 — Diffusion features comme représentation visuelle

### Description

Utiliser les représentations internes d'un modèle de diffusion (Stable Diffusion) comme features visuelles pour la classification, au lieu de CLIP ou du VLM.

### Fondements

Des travaux récents montrent que les diffusion models capturent des informations sémantiques riches dans leurs couches internes :
- **DIFT (Diffusion Features)** : extrait des features de correspondance sémantique depuis des modèles de diffusion pré-entraînés
- **Diffusion Hyperfeatures** (NeurIPS 2023) : agrège les features multi-échelle et multi-timestep en descripteurs par pixel, surpassant DIFT sur les benchmarks de correspondance sémantique
- **Revelio** (ICCV 2025) : montre que les couches `up_ft1` de Stable Diffusion 1.5 à timestep t=25 capturent les informations sémantiques les plus fines, et que ces features sont compétitives avec CLIP pour la classification
- **DDPM-Seg** : utilise les features internes d'un DDPM pour la segmentation sémantique

Le point clé : les diffusion models, entraînés sur des milliards d'images avec des captions textuelles, ont appris à représenter la sémantique visuelle de manière très riche. Les couches intermédiaires du U-Net capturent à la fois les structures de bas niveau (lignes, textures) et les concepts de haut niveau (ambiance, émotion).

### Architecture concrète

```python
from diffusers import StableDiffusionPipeline
import torch

# Extraire les features de diffusion
pipe = StableDiffusionPipeline.from_pretrained("stabilityai/stable-diffusion-2-1")

def extract_diffusion_features(image, timestep=25):
    """Extraire les features de la couche up_ft1 du U-Net"""
    latent = pipe.vae.encode(image).latent_dist.sample()
    noisy_latent = pipe.scheduler.add_noise(latent, torch.randn_like(latent), timestep)
    
    # Forward pass partiel pour extraire les activations intermédiaires
    with torch.no_grad():
        features = pipe.unet(noisy_latent, timestep, encoder_hidden_states=text_embeds)
        # Récupérer up_ft1 via hook
    return features  # [B, C, H, W]

# Puis classifier
classifier = nn.Sequential(
    nn.AdaptiveAvgPool2d(1),
    nn.Flatten(),
    nn.Linear(feature_dim, 256),
    nn.ReLU(),
    nn.Linear(256, 10)
)
```

### Estimation d'effort
- **Temps** : 2-4 semaines
- **Données** : Mêmes données que pour le fine-tuning VLM
- **Difficulté** : Modérée
- **Compute** : SD 2.1 tient sur RTX 3060 en fp16, mais l'extraction est lente (~1-2s par image)

### Impact attendu
+5-10% strict. Les features de diffusion pourraient capturer l'ambiance/mood mieux que CLIP, mais c'est spéculatif.

### Risques
- **Latence** : L'extraction de features de diffusion est coûteuse (forward pass du U-Net). Peut dépasser ta contrainte de 5s/page.
- Les modèles de diffusion sont entraînés sur des photos et de l'art digital, pas spécifiquement du manga en noir et blanc. Le domain gap est inconnu.
- La technique est récente et pas encore validée pour la classification d'émotions.

---

## Piste 9 — Reinforcement Learning avec feedback utilisateur

### Description

Un agent RL qui apprend à ajuster ses prédictions de mood en fonction du feedback implicite de l'utilisateur (skip de musique, changement manuel, durée d'écoute).

### Architecture concrète

- **État** : (embedding visuel de la page actuelle, moods des N pages précédentes, mood actuel prédit)
- **Action** : choisir un mood parmi les 10
- **Récompense** : +1 si l'utilisateur ne skip pas la musique, -1 si skip dans les 5 premières secondes, +2 si l'utilisateur augmente le volume
- **Algo** : PPO ou DQN (le problème est discret et simple)

**Variante plus réaliste** : plutôt que du RL online, collecter du feedback et re-entraîner périodiquement en batch. L'utilisateur annote manuellement les erreurs, et ces corrections servent à fine-tuner le modèle.

### Estimation d'effort
- **Temps** : 4-8 semaines
- **Données** : Feedback continu (nécessite une base d'utilisateurs)
- **Difficulté** : Élevée

### Impact attendu
+5-15% strict, s'améliorant avec le temps. L'avantage est l'adaptation continue et personnalisée.

### Risques
- Cold start : le modèle est mauvais au début, ce qui fait fuir les utilisateurs
- Biais de confirmation : si l'utilisateur ne skip jamais, le modèle n'apprend rien
- Complexité d'infrastructure pour le feedback loop

---

## Piste 10 — GNN sur graphe narratif

### Description

Modéliser les pages comme des nœuds d'un graphe, avec des arêtes de continuité (pages consécutives), de flashback (la page actuelle fait référence à une page passée), et de parallélisme (deux scènes interleaved).

### Fondements

Les GNN sont utilisés pour modéliser des structures narratives complexes dans les séries TV (Balestri & Pescatore, 2025). L'idée est de capturer les dépendances non-linéaires : un flashback crée une arête entre la page actuelle et une page passée, permettant au modèle de "comprendre" que le mood actuel dépend d'un contexte distant.

### Architecture concrète

- **Nœuds** : Une page = un nœud avec features visuelles (CLIP embedding)
- **Arêtes de continuité** : page_i → page_{i+1} (poids 1.0)
- **Arêtes de flashback** : détectées par similarité visuelle avec des pages passées (si une page ressemble visuellement à une page 20 pages plus tôt, c'est probablement un flashback de cette scène)
- **GNN** : GAT (Graph Attention Network) ou GraphSAGE pour propager l'information

### Estimation d'effort
- **Temps** : 8+ semaines
- **Données** : Annotations de structure narrative (flashbacks, scènes parallèles)
- **Difficulté** : Très élevée

### Impact attendu
Inconnu. C'est une piste de recherche, pas une solution éprouvée.

### Risques
- La construction du graphe (détection automatique des flashbacks) est aussi difficile que le problème original
- Overkill pour un problème qui peut probablement être résolu avec des méthodes plus simples

---

## Datasets et ressources clés

### Datasets manga
| Dataset | Contenu | Annotations | Utilité |
|---------|---------|-------------|---------|
| **Manga109** | 21,142 pages, 109 volumes | Panels, bulles, texte, visages, corps (500k+ annotations) | Base pour tout travail manga |
| **Manga109 Segmentation** (CVPR 2025) | Même corpus | Masques pixel-level | Détection marqueurs narratifs |
| **Manga109Dialog** | Même corpus | 132,692 paires speaker-texte | OCR + sentiment |
| **KangaiSet** | Sous-ensemble Manga109 | Émotions faciales (7 classes Ekman) | Fine-tuning emotion |
| **eBDtheque** | 100 images comics | Panels, bulles (avec style : normal/cloud/spike), texte, personnages | Détection type de bulle |
| **CoMix** | Multi-style (manga + US comics) | Multi-task : détection, speaker ID, reading order, dialogue | Benchmark complet |
| **ComicsPAP** | 100k+ samples strips | Pick-a-Panel (compréhension séquentielle) | Test compréhension narrative |

### Papers les plus pertinents

1. **M2M-Gen** (Sharma et al., 2024) — arXiv:2410.09928 — **LE** paper le plus pertinent. Musique de fond pour manga. Pipeline scene segmentation → emotion → music.

2. **MangaLMM** (2025) — arXiv:2505.20298 — Fine-tuning de Qwen2.5-VL pour la compréhension manga. Benchmark MangaVQA.

3. **Re:Verse** (ICCV 2025 Workshop) — VLM manga comprehension benchmark avec annotations dialogue/pensées différenciées.

4. **CoMix** (NeurIPS 2024) — arXiv:2407.03550 — Benchmark multi-task comics. Inclut reading order, speaker ID, dialogue generation.

5. **KangaiSet** (Théodose & Burie, 2023) — Emotion recognition sur les visages manga.

6. **Emotion Classification in Comics** (Sharma & Kukreja, 2025) — VGG16 + attention, 92.6% accuracy.

7. **EmoComicNet** (Dutta et al., 2024) — Pattern Recognition — Multi-task emotion recognition pour comics.

8. **Diffusion Hyperfeatures** (NeurIPS 2023) — Features sémantiques depuis les modèles de diffusion.

9. **Revelio** (ICCV 2025) — Interprétation sémantique des représentations de diffusion.

10. **Neural CRF Transducers** (Hu et al., 2018) — arXiv:1811.01382 — CRF + RNN pour le sequence labeling.

### Outils et repos

- **2U1/Qwen-VL-Series-Finetune** — GitHub — Fine-tuning Qwen2.5-VL avec LoRA
- **manga109api** — Python API pour le dataset Manga109
- **CoMix-dataset** — GitHub (emanuelevivoli) — Code et données CoMix
- **pytorch-crf** — CRF layer pour PyTorch
- **Unsloth** — Fine-tuning VLM optimisé (Qwen2.5-VL 4bit)

---

## Plan d'action recommandé

### Phase 1 — Quick wins (Semaines 1-3)

1. **Implémenter le CRF** sur les outputs existants du VLM (Piste 1). Annoter 5 tomes supplémentaires pour la matrice de transition. Gain attendu : +10-15%.

2. **Ajouter la détection de marqueurs narratifs** (Piste 2). Un prompt VLM dédié avant la classification de mood. Pas d'entraînement nécessaire, juste du prompt engineering.

3. **Extraire le texte OCR** (Piste 5) et l'intégrer comme feature supplémentaire.

### Phase 2 — Fine-tuning (Semaines 3-8)

4. **Constituer un dataset** de 500-1000 pages annotées, multi-manga, avec annotations de mood ET de contexte narratif (flashback/rêve/direct).

5. **Fine-tuner Qwen2.5-VL 7B** avec LoRA (Piste 3) en multi-image (contexte de 2-3 pages précédentes).

6. **Adapter le pipeline M2M-Gen** (Piste 4) pour la détection de scènes et le mood hiérarchique.

### Phase 3 — Modèle séquentiel (Semaines 8-14)

7. **Entraîner un BiLSTM/Transformer** (Piste 6) sur les embeddings du VLM fine-tuné + CRF comme post-processing.

8. **Explorer CLIP contrastif** (Piste 7) si les résultats du fine-tuning VLM plafonnent.

### Phase 4 — Polish et R&D (Semaines 14+)

9. Intégrer le **feedback utilisateur** (Piste 9) pour l'amélioration continue.
10. Explorer les **diffusion features** (Piste 8) et **GNN** (Piste 10) comme recherche.

### Objectif réaliste

Avec les phases 1+2, tu devrais passer de **65% strict / 81% relaxed** à environ **80-85% strict / 92-95% relaxed**. La Phase 3 pourrait amener à **88-92% strict**. Au-delà, les gains sont marginaux et le problème devient celui de l'annotation subjective (deux humains ne sont pas d'accord 100% du temps non plus).

---

## Note sur M2M-Gen — la connexion directe

Je veux insister sur M2M-Gen car c'est **exactement** ton projet. Leur pipeline :

1. Divise le manga en scènes (via les dialogues + changements visuels)
2. Classifie l'émotion par scène (fine-tuning CLIP sur les visages)
3. Utilise GPT-4o pour traduire l'émotion en directive musicale
4. Génère des captions musicales page par page
5. Génère la musique via text-to-music

Ton pipeline KeyToMusic :
1. Capture les pages
2. Classifie le mood par page (VLM)
3. Déclenche l'OST correspondante

Le point crucial : M2M-Gen utilise le **contexte de la scène** pour conditionner les captions page-level. C'est exactement la pièce manquante dans ton pipeline. En intégrant une notion de scène (regroupement de pages) et en conditionnant le mood page-level sur le mood de la scène, tu élimines la majorité des erreurs de contexte.

Ils n'ont pas de code public mais le papier est très détaillé. Et ils utilisent **Manga109** avec les dialogues transcrits — un dataset que tu peux obtenir pour la recherche académique.
