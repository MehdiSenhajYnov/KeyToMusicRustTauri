# Discovery — Recommandations persistantes au switch de profil & Preloading

> **Statut:** Partial — P0 et P1 implementes, P2 optionnel non fait
> **Type:** Bug Fix — Discovery mode
> **Priorite:** Haute

---

## Probleme

Deux bugs lies au systeme Discovery :

1. **Recommandations du profil precedent visibles apres switch** — Quand on passe d'un profil ou le discovery etait deja fait a un nouveau profil avec des sons locaux, les suggestions de l'ancien profil restent affichees jusqu'a ce que le nouveau discovery finisse et ecrase les resultats.

2. **Preloading incomplet** — L'objectif est zero chargement visible pour l'utilisateur apres le discovery initial. Le preloading (telecharger le son, assigner touche/track/momentum, calculer waveform) ne couvre pas tous les cas necessaires, notamment le preload pour le refresh.

---

## Causes identifiees (par priorite)

### P0 — CRITIQUE : Suggestions stale au changement de profil

#### 1. Le store n'est pas clear immediatement au switch

**Fichier:** `src/components/Discovery/DiscoveryPanel.tsx:165-222`

Quand `profile?.id` change, le `useEffect` (ligne 166) fait :
1. Incremente `discoveryGenRef` (ligne 171)
2. Appelle `commands.getDiscoverySuggestions(profile.id)` (ligne 173-174)
3. Si cache existe → `restoreFromCache()` (ligne 178-184)
4. Si pas de cache mais sons → `startDiscovery()` (ligne 189-210)

**Le probleme :** Entre le moment ou `profile?.id` change et le moment ou la Promise de `getDiscoverySuggestions` se resolve (ou `startDiscovery` emet ses premiers `discovery_partial`), les anciennes `visibleSuggestions` restent dans le store et sont affichees. Il n'y a **aucun `clear()` au debut** de cet effet quand le profil a des sons.

Le `clear()` n'est appele que si `!profile` (ligne 168) ou si le profil n'a ni cache ni sons (ligne 213). Le cas "profil avec sons, pas de cache" ne clear pas avant de lancer `startDiscovery`.

**Impact:** L'utilisateur voit les recommendations de l'ancien profil pendant plusieurs secondes (le temps de la resolution locale + fetch des mix YouTube).

**Fix:**
```typescript
// DiscoveryPanel.tsx ligne ~167, AVANT le chargement du cache
clear(); // Toujours clear au changement de profil
const gen = ++discoveryGenRef.current;
```

#### 2. Le listener `discovery_partial` peut merger dans un state stale

**Fichier:** `src/components/Discovery/DiscoveryPanel.tsx:137-150`

Le listener `discovery_partial` (ligne 138-146) verifie `state.isGenerating` mais ne verifie **pas** le `discoveryGenRef`. Si le discovery de l'ancien profil emet un `discovery_partial` pendant que le nouveau profil se charge, le merge se fait dans le store du nouveau profil.

La protection par `discoveryGenRef` n'existe que dans les callbacks de `startDiscovery().then()` (lignes 192, 201, 209), pas dans le listener d'evenements streaming.

**Impact:** Risque de pollution des suggestions du nouveau profil par celles de l'ancien.

**Fix:** Ajouter un check du `discoveryGenRef` dans le listener `discovery_partial`, ou s'assurer que `cancelDiscovery()` (appele dans le cleanup, ligne 220) empeche effectivement l'emission d'evenements.

---

### P1 — IMPORTANT : Preloading pour le refresh

#### 3. Le calcul du refreshStart utilise `visitedIndex` qui est relatif a `allSuggestions`, pas aux items non-reveles

**Fichier:** `src/hooks/useDiscoveryPredownload.ts:114-151`

Le preload pool pour le refresh (ligne 117) fait :
```typescript
const refreshStart = visitedIndex + 1;
```

Mais le refresh dans `handleGenerate` (DiscoveryPanel.tsx:224-247) fait :
```typescript
const unseenStart = state.revealedCount; // ligne 229
const unseen = state.allSuggestions.slice(unseenStart); // ligne 230
```

Donc le refresh montre `allSuggestions[revealedCount]` et suivants, pas `allSuggestions[visitedIndex + 1]`. Le preload devrait precharger les items a partir de `revealedCount`, pas `visitedIndex + 1`.

**Scenario concret :**
- `allSuggestions` a 30 items, `revealedCount = 10`, `visitedIndex = 3` (user a browse 0-3)
- Le preload pool telecharge `allSuggestions[4]` et `allSuggestions[5]` (visitedIndex+1 et +2)
- Le refresh va montrer `allSuggestions[10]` et suivants (unseenStart = revealedCount = 10)
- Les items 10 et 11 ne sont PAS precharges → l'utilisateur voit des placeholders

**Fix:** Le preload pour le refresh doit utiliser `revealedCount` au lieu de `visitedIndex + 1` :
```typescript
const refreshStart = revealedCount; // pas visitedIndex + 1
```

Et il faut aussi s'abonner a `revealedCount` dans le hook :
```typescript
const revealedCount = useDiscoveryStore((s) => s.revealedCount);
```

#### 4. Le preload pool ne precharge que 2 items — devrait precharger au moins les 2 premiers items qui seront visibles au refresh

**Fichier:** `src/hooks/useDiscoveryPredownload.ts:119`

La boucle `for (let i = 0; i < 2; i++)` est correcte pour 2 items, mais avec le mauvais offset (voir P1-3). Avec le bon offset (`revealedCount`), les 2 items precharges seront ceux affiches en premier au refresh.

---

### P2 — AMELIORATION : Preload complet = zero loading visible

#### 5. Le preloading de la fenetre [x-2, x+3] ne garantit pas que le son courant (x) est pret avant affichage

**Fichier:** `src/hooks/useDiscoveryPredownload.ts:58-112`

Le preload (ligne 62-69) lance les telecharges en parallele pour les indices [x-2, x+3], mais quand l'utilisateur navigue (goToNext), le nouveau `current` peut etre en `idle` ou `downloading`. Le composant affiche alors un placeholder waveform (ligne 796-801 dans DiscoveryPanel.tsx).

**Comportement attendu :** Idealement, la navigation devrait etre instantanee — le son suivant devrait deja avoir son waveform/momentum pret. L'asymetrie [x-2, x+3] avec max 3 concurrent devrait suffire en theorie, mais si le reseau est lent ou si l'utilisateur navigue vite, les preloads n'auront pas le temps de finir.

**Amelioration possible :** Prioriser le telechargement de `x+1` avant `x+2` et `x+3` dans la boucle (l'ordre actuel est deja correct car les indices sont tries). Le vrai gain serait d'augmenter la concurrence de 3 a 4 ou 5 pour les navigations rapides.

---

## Plan d'implementation

### Phase 1 : Fix du clear au switch de profil (P0)

- [x] **1.1** Dans `DiscoveryPanel.tsx` (effet ligne 166), appeler `clear()` immediatement apres le check `!profile`, avant de charger le cache ou lancer le discovery
- [x] **1.2** Verifier que le listener `discovery_partial` (ligne 137-150) ne merge pas de donnees d'un discovery precedent apres un clear — ajouter un check `discoveryGenRef` ou utiliser un compteur local
- [x] **1.3** S'assurer que `cancelDiscovery()` dans le cleanup (ligne 218-221) est effectif cote backend — verifier que le `AtomicBool` cancel dans `src-tauri/src/commands.rs:1135` empeche bien les `discovery_partial` events posterieurs

### Phase 2 : Fix du preload refresh (P1)

- [x] **2.1** Dans `useDiscoveryPredownload.ts`, changer `refreshStart` de `visitedIndex + 1` a `revealedCount` (ligne 117)
- [x] **2.2** Ajouter `revealedCount` comme dependance du hook (selectionneur Zustand + dependency array du useEffect ligne 151)
- [ ] **2.3** Tester : profil avec 30+ suggestions, naviguer 3-4, revenir au 1er, refresh → les 2 premiers nouveaux sons doivent s'afficher sans placeholder

### Phase 3 : Amelioration du preload (P2, optionnel)

- [ ] **3.1** Evaluer si la concurrence de 3 est suffisante ou si 4-5 serait mieux pour les navigations rapides
- [ ] **3.2** Considerer un indicateur subtil (micro-spinner dans le waveform) si le preload n'est pas encore pret plutot qu'un placeholder vide

---

## Fichiers concernes

| Fichier | Modifications |
|---------|--------------|
| `src/components/Discovery/DiscoveryPanel.tsx` | Ajouter `clear()` au debut de l'effet profile switch (ligne ~167). Ajouter check gen dans listener `discovery_partial` |
| `src/hooks/useDiscoveryPredownload.ts` | Changer `refreshStart` pour utiliser `revealedCount` (ligne 117). Ajouter `revealedCount` au selectionneur et aux deps |
| `src/stores/discoveryStore.ts` | Aucune modification necessaire (le `clear()` existe deja, ligne 293-308) |
| `src-tauri/src/commands.rs` | Verifier que `cancel_discovery` (ligne 1135) empeche les events posterieurs |
