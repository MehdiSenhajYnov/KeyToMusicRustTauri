# Phase 8.2 - Combined Key Shortcuts & UI Refactor

## Recap des discussions (2026-01-25)

Ce document capture toutes les décisions et détails techniques discutés pour éviter toute perte d'information.

---

## 1. Problèmes identifiés

### 1.1 Alignement icônes ProfileSelector
- **Problème** : Les icônes duplication (⎘) et suppression (x) n'étaient pas alignées
- **Solution** : Remplacé par des SVG uniformes avec dimensions fixes (w-5 h-5, icônes w-3.5 h-3.5)
- **Status** : ✅ Corrigé

### 1.2 Numpad + Shift ne fonctionne pas pour le momentum
- **Problème** : Shift+Numpad4 ne déclenche pas le momentum
- **Cause** : Comportement hardware/OS standard. Quand NumLock est ON et Shift est pressé, le système envoie la touche alternative (ArrowLeft, End, etc.) au lieu de "Shift+Numpad4"
- **Status** : ⚠️ Limitation hardware, pas un bug de l'app
- **Solution possible** : Permettre à l'utilisateur de choisir un autre modificateur (Alt, Ctrl) pour le momentum

### 1.3 Combined Key Shortcuts pas visible sur le frontend
- **Problème** : Le backend envoie les codes combinés (Ctrl+KeyA) mais l'UI ne permet pas de les assigner ni de les afficher
- **Status** : 🔄 À implémenter (UI refactor nécessaire)

---

## 2. Nouvelle architecture proposée

### 2.1 Système de capture de touches unifié

Actuellement, deux systèmes différents :
- **Global shortcuts (Settings)** : Capture au clic avec support modifiers ✓
- **Assignment de sons (AddSoundModal)** : Input texte "aze", pas de modifiers ✗

**Proposition** : Unifier avec le même pattern de capture partout.

### 2.2 Refonte UI AddSoundModal - Key Assignment

**Avant (actuel)** :
```
Keys: [aze________]  ← tape tout d'un coup
```

**Après (proposé)** :
```
Keys:
┌────────────────────────────────────────┐
│ 1. [Click to capture]  →  [Ctrl+A] [×] │
│ 2. [Click to capture]  →  [Z]      [×] │
│ 3. [+ Add key]                         │
└────────────────────────────────────────┘

Sounds (5):                  Assigned to:
├─ epic_battle.mp3           → Ctrl+A (key 1)
├─ calm_ambient.mp3          → Z (key 2)
├─ victory_theme.mp3         → Ctrl+A (cycle)
├─ tension.mp3               → Z (cycle)
└─ finale.mp3                → Ctrl+A (cycle)
```

**Comportement** :
- Par défaut : 1 slot de key vide
- Chaque slot = bouton de capture (comme dans Settings)
- Cliquer → mode capture → appuyer sur touche(s) → enregistré
- Support complet des modifiers : Ctrl+A, Shift+F1, Alt+Z
- Bouton "+" apparaît si `nombre de keys < nombre de sons`
- Bouton "×" pour supprimer un slot
- Preview du cycling en temps réel en dessous
- Si 1 seule key pour N sons → tous assignés à cette key

### 2.3 Format de stockage des key codes

**Format string avec modifiers** (déjà implémenté backend) :
```
"Ctrl+KeyA"
"Shift+F1"
"Ctrl+Shift+KeyZ"
"KeyA"  (sans modifier)
```

**Ordre des modifiers** (consistant backend/frontend) :
```
Ctrl > Shift > Alt > Key
```

Exemples :
- `Ctrl+Shift+KeyA` ✓
- `Shift+Ctrl+KeyA` ✗ (mauvais ordre)

---

## 3. Multi-key chords (A+Z simultanés) - Future v2

### 3.1 Concept
Permettre des combinaisons de touches non-modifier pressées ensemble, comme un accord de piano :
- `KeyA+KeyZ` (A et Z pressés en même temps)
- `Ctrl+KeyA+KeyZ` (Ctrl + A + Z)

### 3.2 Avantage : Explosion combinatoire
| Type | Combinaisons (~50 touches) |
|------|----------------------------|
| 1 touche | 50 |
| 2 touches | 1,225 |
| 3 touches | 19,600 |
| + Modifiers (×8) | ×8 pour chaque |

### 3.3 Défi technique : Détection "simultanée"
Même si l'utilisateur appuie "en même temps", les événements arrivent séquentiellement :
```
t=0ms    KeyA down
t=3ms    KeyZ down    ← "simultané" mais séquentiel
```

**Solution : Fenêtre de détection**
```
KeyA pressé → timer 30ms démarre
KeyZ pressé (dans 30ms) → timer reset
... 30ms sans nouvelle touche ...
→ Cherche binding pour "KeyA+KeyZ" (trié alphabétiquement)
```

### 3.4 Résolution de conflits
Si bindings existent pour `A`, `A+Z`, et `A+Z+M` :
- User appuie A seul → attendre 30ms → trigger "A"
- User appuie A+Z rapide → attendre 30ms → trigger "A+Z" (pas de double-trigger)
- User appuie A+Z+M → trigger "A+Z+M"

### 3.5 Trade-off latence
| Approche | Latence | Multi-key |
|----------|---------|-----------|
| Actuel (immédiat) | 0ms | ❌ |
| Avec fenêtre | +30-50ms | ✓ |

**Optimisation possible** : N'appliquer le délai que si des multi-key combos existent dans le profil pour cette touche.

### 3.6 Décision
- **Phase 1** : Implémenter Modifier + 1 touche (Ctrl+A, Shift+F1)
- **Phase 2** : Ajouter multi-key chords (A+Z) plus tard si besoin

---

## 4. Modificateur momentum configurable - Discussion

### 4.1 Idée
Permettre à l'utilisateur de choisir quel modificateur déclenche le momentum :
- Shift (défaut actuel)
- Alt
- Ctrl
- None (momentum toujours désactivé, utiliser Auto-Momentum toggle)

### 4.2 Avantages
- Résout le problème Numpad+Shift
- Plus de flexibilité pour l'utilisateur

### 4.3 Implémentation envisagée
- Nouveau champ dans `config.json` : `momentumModifier: "Shift" | "Alt" | "Ctrl" | "None"`
- Dropdown dans Settings
- Backend : vérifier le modifier configuré au lieu de hardcoder Shift
- Frontend : idem

### 4.4 Status
⏸️ En discussion - à décider si on implémente ou pas.

---

## 5. Plan d'implémentation

### Phase 1 : UI Refactor AddSoundModal (Priorité haute)
1. [ ] Créer composant `KeyCaptureSlot` réutilisable
2. [ ] Remplacer input texte par liste de slots dans AddSoundModal
3. [ ] Implémenter logique de capture avec modifiers
4. [ ] Afficher preview du cycling
5. [ ] Tester avec Ctrl+A, Shift+F1, etc.

### Phase 2 : Affichage KeyGrid
1. [ ] Mettre à jour `keyCodeToDisplay()` pour afficher "Ctrl+A" au lieu de "KeyA"
2. [ ] Gérer les noms plus longs dans la grille (truncate ou wrap)

### Phase 3 : Backend adjustments
1. [ ] Vérifier que le backend envoie correctement les combined codes (déjà fait)
2. [ ] S'assurer du fallback : si "Ctrl+A" n'existe pas mais "A" existe, déclencher "A" avec momentum si Shift était dans le combo

### Phase 4 (Optionnel) : Momentum modifier configurable
1. [ ] Ajouter champ config
2. [ ] UI dropdown dans Settings
3. [ ] Mise à jour backend/frontend

### Phase 5 (Future) : Multi-key chords
1. [ ] Implémenter fenêtre de détection 30ms
2. [ ] Tri alphabétique des touches pour consistance
3. [ ] Résolution de conflits
4. [ ] UI pour capturer multi-key

---

## 6. Fichiers à modifier

### Frontend
- `src/components/Sounds/AddSoundModal.tsx` - Refonte majeure
- `src/components/Keys/KeyCaptureSlot.tsx` - Nouveau composant
- `src/components/Keys/KeyGrid.tsx` - Affichage combined keys
- `src/utils/keyMapping.ts` - Fonctions helper (déjà partiellement fait)
- `src/hooks/useKeyDetection.ts` - Déjà fait, vérifier fallback

### Backend (si nécessaire)
- `src-tauri/src/keys/detector.rs` - Déjà fait pour combined codes
- Potentiellement ajouter config momentum modifier

### Documentation
- `CLAUDE.md`
- `KeyToMusic_Technical_Specification.md`
- `TASKS.md`

---

## 7. Questions ouvertes

1. **Momentum avec combined keys** : Si user assigne `Ctrl+A`, est-ce que `Ctrl+Shift+A` déclenche le momentum ? Ou faut-il un binding explicite ?
   - Proposition : Oui, ajouter Shift à un binding existant déclenche momentum

2. **Migration profils existants** : Les profils avec `KeyA` continuent de fonctionner, Shift+A = momentum
   - Proposition : Oui, backward compatible

3. **Limite de modifiers** : Autoriser Ctrl+Shift+Alt+Key ou limiter à 2 modifiers ?
   - Proposition : Pas de limite artificielle, laisser le user décider

4. **Affichage KeyGrid** : Les noms "Ctrl+Shift+A" sont plus longs. Truncate ? Smaller font ? Wrap ?
   - À tester visuellement

---

## 8. Références

- Fichier actuel AddSoundModal : `src/components/Sounds/AddSoundModal.tsx`
- Système de capture Settings : `src/components/Settings/SettingsModal.tsx`
- Backend detector : `src-tauri/src/keys/detector.rs`
- Key mapping utils : `src/utils/keyMapping.ts`
