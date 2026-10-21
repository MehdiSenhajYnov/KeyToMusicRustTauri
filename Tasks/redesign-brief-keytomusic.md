# Brief de Redesign Complet de KeyToMusic

Ce document contient deux versions du meme brief :

- une version "prompt IA" a envoyer a une IA qui doit produire un prototype React non fonctionnel
- une version "brief designer" a envoyer a un designer humain

Les deux versions donnent un maximum d'information produit sans imposer le nombre d'ecrans, l'architecture, la navigation, la direction artistique, les composants, ni la mise en page.

---

## Version 1 - Prompt IA

Tu vas redesigner de zero tout le produit KeyToMusic.

Ton livrable ne doit pas etre une image. Je veux un prototype/site en React, non fonctionnel cote backend, avec donnees mockees, qui montre l'ensemble du produit et de ses etats. Tu as une liberte totale sur la direction artistique, l'architecture de l'information, la navigation, le nombre de vues, la facon de regrouper les fonctions, les composants, les animations et la hierarchie UX.

Je ne veux surtout pas que tu recopies la structure actuelle de l'application. Je ne veux pas non plus que tu te limites a "faire joli". Ton travail doit couvrir tout le produit et ne rien oublier. Tu peux reorganiser totalement l'experience, fusionner des zones, en scinder d'autres, changer les priorites visuelles et la logique de circulation, mais aucune capacite importante ne doit disparaitre.

Le produit a redesigner est un outil desktop de soundboard pour lecture de manga. Il permet de preparer des profils audio, d'assigner des sons a des touches clavier globales, de lire plusieurs couches sonores en meme temps, d'importer des sons locaux ou YouTube, de decouvrir automatiquement de nouvelles musiques, et d'utiliser un systeme "Manga Mood AI" qui detecte l'ambiance d'une page de manga et declenche automatiquement des sons tags par humeur. Le produit inclut aussi une petite extension navigateur pour le module Manga Mood.

Le resultat doit donc representer l'ensemble du produit KeyToMusic, pas uniquement la partie "soundboard de base".

### Contraintes de fond

- Le prototype peut etre totalement mocke, sans vraie lecture audio, sans vrai backend, sans vraie IA, sans vraies API.
- En revanche, l'interface doit donner l'impression que tout le produit existe reellement, avec ses cas nominaux, ses cas limites, ses etats vides, ses etats de progression, ses confirmations, ses erreurs et ses surfaces annexes.
- Tu es libre de redefinir totalement la navigation.
- Tu es libre de redefinir totalement la relation entre les zones principales et secondaires.
- Tu es libre de creer des vues, panneaux, modales, overlays, espaces contextuels, assistants, dashboards, bibliotheques, timelines, inspecteurs, etc.
- Tu ne dois pas simplifier le produit en supprimant des fonctionnalites ou en les rendant invisibles.
- Tu ne dois pas te baser sur la structure actuelle pour decider la nouvelle structure.
- Le produit est conceptuellement desktop-first, local-first, et centre sur un usage rapide pendant la lecture d'un manga.
- Le prototype doit integrer un vrai switch de theme clair / sombre dans l'experience proposee.
- Je veux une direction visuelle orientee "liquid glass", a interpreter librement dans ton propre langage visuel.

### Ce qu'est KeyToMusic

KeyToMusic est un outil local qui sert a preparer et piloter une bande-son de lecture :

- l'utilisateur cree plusieurs profils
- chaque profil contient des pistes audio
- chaque piste peut contenir ou jouer des sons assignes a des touches
- une touche peut declencher un ou plusieurs sons
- une meme touche peut aussi declencher plusieurs pistes en parallele
- les sons peuvent venir de fichiers locaux ou de YouTube
- les sons ont un point de depart "momentum"
- les pistes, les sons et les bindings ont des reglages independants
- l'application peut etre pilotee en arriere-plan via des touches globales
- une IA locale peut detecter l'ambiance d'un manga et declencher automatiquement des sons relies a des moods

### Objets metier a representer clairement

- Profil : une configuration complete de lecture. Il contient un nom, des pistes, des sons, des assignations de touches, un historique/disposition propre, des preferences de decouverte, une blacklist de suggestions, etc.
- Piste : une couche audio logique, par exemple OST, ambiance, SFX, voix. Une piste a au moins un nom et un volume, et peut etre en lecture ou non.
- Son : un asset audio individuel avec nom, source, volume, duree, momentum, et parfois un identifiant YouTube resolu.
- Source audio : soit locale, soit YouTube mis en cache localement.
- Binding de touche : une relation entre une touche, une piste et une liste de sons. Un binding a aussi un mode de boucle, un index courant de lecture, un nom optionnel, un mood optionnel et un seuil d'intensite optionnel.
- Moods AI : `epic`, `tension`, `sadness`, `comedy`, `romance`, `horror`, `peaceful`, `mystery`, avec intensite 1 a 3.

### Capacites produit a couvrir

#### 1. Gestion des profils

- l'utilisateur peut voir tous ses profils
- creer un profil
- selectionner un profil
- renommer un profil
- dupliquer un profil
- supprimer un profil
- charger un profil existant
- un profil vide doit etre traite comme un vrai etat produit
- le produit doit gerer le cas ou aucun profil n'existe encore

#### 2. Gestion des pistes

- un profil peut contenir plusieurs pistes
- l'utilisateur peut creer une piste
- renommer une piste
- regler le volume d'une piste
- supprimer une piste
- voir quelles pistes jouent actuellement
- une piste peut etre vide ou contenir des bindings
- il existe une limite actuelle de 20 pistes par profil

#### 3. Gestion des sons

- ajout de sons locaux
- ajout de sons via glisser-deposer de fichiers audio
- ajout par lot de plusieurs fichiers
- ajout de sons depuis YouTube
- preview d'un son avant validation
- gestion du volume individuel d'un son
- gestion de sa duree
- gestion de son point de depart "momentum"
- un son peut etre supprime
- un son peut etre deplace vers une autre touche
- un son peut etre relie a une piste via un binding

#### 4. Momentum, waveform et preview

- chaque son a un point de depart "momentum" utilisable pour commencer la lecture plus tard dans le fichier
- l'utilisateur doit pouvoir voir une waveform
- l'outil detecte automatiquement un momentum suggere a partir de l'energie du son
- cette suggestion doit pouvoir etre comprise, comparee et appliquee
- l'utilisateur doit pouvoir modifier manuellement le momentum
- il doit exister une lecture de preview depuis le point de momentum
- la waveform peut aussi montrer la position de lecture quand un son joue

#### 5. Assignation des touches

- l'utilisateur peut assigner des touches a des sons
- l'utilisateur peut assigner une ou plusieurs touches lors d'un ajout multiple
- si plusieurs sons sont ajoutes pour moins de touches que de sons, l'affectation peut cycler
- un binding est defini par un couple touche + piste
- une meme touche peut donc exister sur plusieurs pistes en meme temps
- une meme touche peut declencher plusieurs pistes simultanement
- un binding peut contenir plusieurs sons
- un binding peut avoir un nom personnalise
- l'utilisateur peut changer la touche d'un binding entier
- l'utilisateur peut deplacer un seul son d'un binding vers une autre touche
- l'utilisateur peut supprimer un binding entier
- l'utilisateur peut modifier la piste cible d'un binding
- l'utilisateur peut modifier le mode de boucle d'un binding

#### 6. Modes de boucle

- `off`
- `single`
- `random`
- `sequential`

Le produit doit rendre tres clair qu'un binding multi-son n'est pas la meme chose qu'un binding multi-piste.

#### 7. Detection clavier et raccourcis globaux

- l'application detecte des touches globales, y compris en arriere-plan
- l'utilisateur peut activer ou desactiver la detection clavier
- il existe un raccourci global "stop all"
- il existe un raccourci global pour activer/desactiver l'auto-momentum
- il existe un raccourci global pour activer/desactiver la detection clavier
- il existe un cooldown global entre pressions
- le produit supporte aussi des combinaisons / chords multi-touches
- le produit gere un "momentum modifier" : maintenir une touche de modification pour lancer un son depuis son momentum
- les conflits entre raccourcis globaux, touches assignees et combinaisons doivent etre visibles et comprehensibles
- l'application doit aussi montrer la logique d'aide / recapitulatif des raccourcis
- le produit comporte aussi des interactions clavier de productivite comme undo/redo, focus de recherche, selection globale, navigation dans Discovery et fermeture / annulation des overlays

#### 8. Lecture audio

- volume master global
- volume par piste
- volume par son
- crossfade entre sons d'une meme piste
- lecture simultanee sur plusieurs pistes
- stop d'une piste
- stop de tout
- affichage des sons actuellement en lecture
- affichage de la progression de lecture
- possibilite de seek sur un son en cours
- gestion d'un etat "rien ne joue"
- gestion d'erreurs audio ou de peripherique de sortie

#### 9. Recherche et filtre dans les assignations

- l'utilisateur peut rechercher parmi les touches / sons / noms personnalises
- filtrer par piste
- filtrer par mode de boucle
- filtrer par statut lecture / pas en lecture
- filtrer par mood
- voir combien d'elements matchent
- effacer rapidement la recherche et les filtres

#### 10. Selection simple et multiple

- selection d'une seule touche
- multi-selection
- selection par plage
- selection globale
- edition en masse de plusieurs bindings
- suppression en masse
- modification en masse de la piste et du mode de boucle
- etat dedie quand plusieurs touches sont selectionnees

#### 11. Ajout local de sons

- ouverture d'un file picker
- drag and drop de fichiers audio
- support de plusieurs fichiers
- previsualisation des fichiers ajoutes
- edition de leur momentum avant validation
- preview avant import final
- choix ou creation d'une piste au moment de l'ajout
- choix du volume initial
- choix du mode de boucle
- affectation de touche(s) dans le meme flow

#### 12. Ajout YouTube

- coller une URL YouTube
- rechercher YouTube par texte
- voir une liste de resultats
- voir si un resultat est deja telecharge
- preview stream d'un resultat de recherche
- ajout d'un resultat a la bibliotheque du profil
- gestion d'un etat ou `yt-dlp` n'est pas installe et doit etre installe
- affichage d'un vrai etat de telechargement/progression

#### 13. Import de playlist YouTube

- si une URL contient une playlist, le produit peut recuperer la playlist
- l'utilisateur peut choisir de ne prendre qu'une video ou toute la playlist
- l'utilisateur peut voir les entrees de la playlist
- selectionner toutes les entrees, aucune, ou une selection partielle
- ajouter plusieurs videos choisies

#### 14. Discovery musical

Le produit contient un moteur de decouverte de sons.

- les suggestions viennent des sons deja presents dans le profil
- les seeds peuvent venir de sons YouTube ou de sons locaux resolves automatiquement vers YouTube
- les suggestions sont classees selon leur occurrence dans plusieurs Mix YouTube croises
- les suggestions deja possedees, deja vues, dismiss, ou dislikeds ne doivent pas revenir normalement
- les suggestions ont un titre, une chaine, une duree, des seeds d'origine, une proposition d'affectation, et un etat de predownload
- l'utilisateur peut lancer / relancer la decouverte
- annuler une generation
- voir une progression de generation
- voir le cas ou des seeds locaux sont en cours de resolution
- voir le cas ou aucune suggestion n'est disponible
- voir le cas ou aucune seed n'est exploitable
- voir le cas d'erreur
- preview une suggestion
- seek la preview
- regler un volume de preview dedie, distinct du volume master
- modifier avant ajout la touche suggeree
- modifier avant ajout la piste suggeree
- modifier avant ajout le momentum suggere
- ajouter la suggestion au profil
- dislike une suggestion de maniere permanente pour le profil
- la progression de decouverte est memorisee par profil

#### 15. Blacklist Discovery

- le produit doit permettre de consulter les videos blacklistees pour un profil
- voir leurs metadonnees
- les retirer de la blacklist
- etat vide si la liste est vide

#### 16. Import / export des profils

- export d'un profil dans un format portable `.ktm`
- import d'un profil `.ktm`
- import d'un ancien format legacy `.json`
- lors de l'export, il y a une progression de copie fichier par fichier
- l'export peut etre annule
- si l'utilisateur ferme l'application pendant un export, il y a un avertissement
- le produit doit pouvoir montrer les succes, echecs et annulations

#### 17. Gestion des fichiers manquants

- au chargement d'un profil ou pendant la lecture, des sons peuvent manquer sur disque
- si le son etait local, l'utilisateur peut le relocaliser
- si le son venait de YouTube, l'utilisateur peut le retelecharger
- l'utilisateur peut aussi supprimer la reference
- il peut ignorer un cas ou tous les cas restants
- il existe donc une vraie file de gestion des sons manquants

#### 18. Confirmation, feedback et overlays

- confirmations d'actions destructives
- toasts / notifications de succes, erreur, info, warning
- progression d'export
- etats de chargement
- etats skeleton / attente
- cas "fermer / annuler / confirmer"

#### 19. Parametres audio et systeme

- volume master
- duree de crossfade
- cooldown clavier
- fenetre de chord detection
- momentum modifier
- peripherique de sortie audio
- aide / documentation des raccourcis
- ouverture du dossier data
- ouverture du dossier de logs
- il existe aussi des reglages avances lies au moteur Mood AI, aujourd'hui tres peu exposes visuellement, comme les seuils d'entree et de sortie du mood, le nombre minimal de pages avant changement et la taille de fenetre d'analyse

#### 20. Linux / Wayland

- sur Linux Wayland, la detection clavier globale peut necessiter une permission systeme supplementaire
- le produit doit pouvoir montrer si l'acces background est pret, manquant, en cours de verification ou activable automatiquement
- il existe un flow d'auto-fix
- il existe un etat "recheck"

#### 21. Manga Mood AI dans l'application principale

Le produit intègre un systeme local d'analyse d'ambiance manga.

- il existe un toggle general pour activer ou desactiver ce systeme
- le runtime IA local et le modele sont des elements installables separement
- ils ont des etats : non installe, en telechargement, pret, demarrage, actif, erreur, arrete
- il existe un port API local configurable
- l'application expose l'etat du runtime et l'etat de l'API extension
- l'utilisateur peut lancer ou arreter le runtime
- l'application peut telecharger le modele avec progression
- les bindings peuvent etre tags avec un mood
- chaque binding peut aussi avoir un niveau d'intensite minimum requis
- quand l'IA detecte un mood engage, tous les bindings correspondants peuvent etre joues automatiquement
- l'application doit aussi montrer la difference entre "dernier mood brut detecte" et "mood actuellement retenu / joue"

#### 22. Extension navigateur Manga Mood

Le produit ne se limite pas a l'app desktop. Il existe aussi une petite extension navigateur, a inclure dans le redesign global.

Cette extension :

- se connecte a l'application locale
- peut activer/desactiver la mood detection
- peut regler le port API local
- montre l'etat de connexion a l'application
- montre le statut du service
- montre la page manga visible
- montre le dernier mood detecte
- montre combien de pages sont deja analysees
- montre "Current Work"
- montre "Current Note"
- montre "Already Analyzed"
- montre "Next Pages"
- montre des evenements recents
- montre un apercu de la page courante
- montre si la page est en cours d'analyse ou deja analysee
- permet d'ouvrir l'image courante
- permet d'activer l'extension sur l'onglet courant si la page n'est pas auto-detectee

Il faut donc penser aussi a cette surface produit et a ses etats :

- extension connectee
- extension non connectee
- API inaccessible
- aucune page manga detectee
- extension inactive sur cet onglet
- page visible analysee
- page visible en cours d'analyse

### Etats et situations a ne surtout pas oublier

- premier lancement sans profil
- profil existe mais aucune piste
- profil avec pistes mais aucune touche assignee
- aucun son en lecture
- plusieurs pistes jouent en meme temps
- changement de profil pendant qu'il y a du son
- export en cours
- export annule
- fermeture fenetre pendant export
- son local manquant
- son YouTube manquant
- ajout YouTube sans `yt-dlp`
- preview YouTube indisponible
- discovery sans seeds
- discovery en cours
- discovery avec resolution de sons locaux
- discovery vide
- discovery avec erreur
- suggestion predownloaded
- suggestion en telechargement
- suggestion en erreur
- extension non connectee a l'app
- model IA non installe
- runtime IA non lance
- API extension non prete
- detection clavier desactivee
- conflit de raccourcis
- permission Linux manquante

### Ce que je veux de toi

- redesigner tout le produit sans aucune contrainte de structure
- apporter une vraie vision UI/UX, pas un simple reskin
- assumer des choix forts de hierarchie, de navigation, de priorisation et de mise en scene
- montrer comment un utilisateur comprend, prepare, pilote et ajuste son univers sonore
- rendre visibles les systemes complexes sans les rendre confus
- produire un prototype React statique mais credible
- integrer un theme clair et un theme sombre avec possibilite de bascule
- explorer une direction visuelle liquid glass sans tomber dans une interface gadget ou illisible

### Ce que je ne veux pas

- une simple copie amelioree de l'interface actuelle
- une reduction du scope
- une disparition des cas limites ou des etats secondaires
- une reponse sous forme de maquettes image uniquement
- une UI generique qui "fait dashboard SaaS"

---

## Version 2 - Brief Designer

Je te demande de repenser completement KeyToMusic.

Je ne veux pas une variation cosmetique de l'existant. Je veux une vraie refonte de fond, en repartant de zero sur l'organisation, la hierarchie, la circulation, la lisibilite, la facon de montrer la complexite et le caractere du produit.

Je te laisse une liberte totale sur la forme. Ne te limite pas a la structure actuelle. Ne te sens pas oblige de conserver les zones, la navigation ou les relations visuelles actuelles. Tu peux reinventer totalement la facon dont le produit se parcourt et se comprend.

En revanche, je veux une comprehension exhaustive du produit. Rien d'important ne doit etre oublie. S'il y a une fonction complexe, un cas vide, un etat de progression, une erreur, une surface secondaire, un mode avance ou une contrainte systeme, il faut que ton travail l'integre quelque part dans l'experience.

Le livrable attendu est un prototype React non fonctionnel, avec donnees mockees. Je veux pouvoir comparer ta vision a celle d'une IA. Je ne veux donc pas seulement des visuels isoles : il faut que l'ensemble du produit existe de facon coherente dans le prototype.

Je veux aussi deux directions explicites dans le resultat :

- un vrai theme clair et un vrai theme sombre, avec possibilite de bascule
- une sensibilite visuelle "liquid glass", librement interpretee, mais suffisamment maitrisee pour rester lisible et elegante

### Nature du produit

KeyToMusic est un outil desktop local pour accompagner la lecture de manga avec une bande-son dynamique.

L'utilisateur prepare des profils. Dans chaque profil, il organise des pistes, importe des sons, relie des sons a des touches clavier globales, ajuste les points de depart des sons, gere la lecture simultanee de plusieurs couches, decouvre de nouvelles musiques, et peut activer un systeme d'IA locale qui detecte le mood d'une page de manga pour declencher automatiquement des sons tags.

Le produit comprend aussi une extension navigateur liee a ce systeme de mood detection.

### Elements fonctionnels incontournables

- profils multiples : creation, selection, renommage, duplication, suppression
- pistes multiples : creation, suppression, renommage, volume, lecture en cours
- limite actuelle de 20 pistes par profil
- sons locaux et sons YouTube
- import par lot, drag and drop, preview, configuration avant ajout
- gestion du momentum de chaque son, avec waveform et suggestion automatique
- assignation des touches
- possibilite pour une meme touche de declencher plusieurs pistes en parallele
- bindings multi-sons avec modes de boucle
- edition detaillee d'un binding
- edition groupée de plusieurs bindings
- historique undo / redo
- recherche et filtrage dans les assignations
- lecture audio globale, master volume, per-track volume, per-sound volume, seek, stop, stop all
- raccourcis globaux configurables
- chords et combinaisons clavier
- aide sur les raccourcis
- detection clavier globale avec contrainte Linux/Wayland a gerer
- import/export de profils
- import legacy
- gestion des fichiers manquants
- systeme de feedback, confirmations, progression, erreurs
- moteur Discovery complet
- blacklist permanente des suggestions Discovery
- systeme Manga Mood AI complet
- popup d'extension navigateur relie a ce systeme

### Details produit a prendre en compte

#### Profils, pistes, sons et bindings

Un profil est une configuration complete. Il contient :

- des pistes audio
- des sons
- des bindings de touches
- une blacklist Discovery

Une piste est une couche logique, par exemple OST, ambiance, SFX ou voix.

Un son peut etre :

- local
- YouTube mais telecharge et mis en cache localement

Un binding relie :

- une touche
- une piste
- une liste de sons
- un mode de boucle
- un nom optionnel
- un mood optionnel
- une intensite minimale optionnelle

Le point important : un binding est unique par couple touche + piste. Cela veut dire qu'une meme touche peut exister sur plusieurs pistes a la fois et declencher plusieurs couches en parallele.

#### Clavier global et raccourcis

Le produit repose fortement sur le clavier. Il faut donc rendre comprehensibles :

- l'activation ou desactivation de la detection clavier globale
- le raccourci global stop all
- le raccourci global d'activation/desactivation de l'auto-momentum
- le raccourci global d'activation/desactivation de la detection clavier
- le cooldown global entre pressions
- les chords / combinaisons multi-touches
- le momentum modifier
- les conflits entre raccourcis, bindings et combinaisons
- l'aide / recapitulatif des raccourcis

#### Playback

Le produit doit rendre tangible :

- le volume master
- le volume de chaque piste
- le volume de chaque son
- la liste des sons en cours de lecture
- la progression de lecture
- le seek
- le stop d'une piste
- le stop global
- le crossfade
- l'etat "rien ne joue"

#### Momentum et waveform

Chaque son a un point de depart "momentum". Le produit dispose d'une waveform et d'une detection automatique d'un momentum suggere. Il faut pouvoir :

- comprendre ce que represente ce point
- comparer la valeur actuelle et la valeur suggeree
- previsualiser depuis ce point
- ajuster la valeur manuellement

#### Ajout de sons

L'ajout de sons n'est pas juste un import brut. Le flow reunit plusieurs decisions :

- source locale ou YouTube
- un ou plusieurs fichiers
- recherche YouTube par texte
- ajout direct par URL
- playlist YouTube avec selection partielle ou totale
- preview
- volume initial
- mode de boucle
- piste cible
- creation eventuelle d'une nouvelle piste
- touche ou touches cibles
- logique de cyclage si plusieurs sons pour moins de touches
- edition du momentum avant validation

Il faut aussi tenir compte des interactions clavier transverses du produit, notamment :

- undo / redo
- focus de recherche
- selection globale dans la zone des assignations
- navigation au clavier dans Discovery
- fermeture / annulation via le clavier

#### Recherche et organisation

Il faut couvrir la facon dont l'utilisateur retrouve et manipule ses assignations :

- recherche texte
- filtrage par piste
- filtrage par mode de boucle
- filtrage par etat de lecture
- filtrage par mood
- selection simple
- multi-selection
- edition groupée

#### Discovery

Le systeme de decouverte musicale est important.

Il faut montrer :

- la generation de suggestions a partir des sons deja presents
- le cas ou des sons locaux doivent d'abord etre resolus
- la progression
- l'annulation
- les suggestions avec leur metadata
- le preview
- le volume de preview dedie
- les predownloads
- le momentum suggere
- la touche et la piste proposees
- la possibilite de modifier ces propositions avant ajout
- l'ajout au profil
- le dislike permanent
- la gestion de la blacklist dans les parametres

#### Manga Mood AI

L'IA de mood fait partie du produit, pas d'un plugin cache.

Elle implique :

- un toggle general
- un runtime local installable
- un modele installable
- un etat de telechargement
- un etat de service
- un port local configurable
- une relation entre l'application et l'extension navigateur
- des bindings tags avec un mood parmi 8 categories
- un seuil d'intensite optionnel
- un etat montrant le dernier mood detecte
- un etat montrant le mood actuellement retenu / joue

Le moteur comporte aussi des reglages plus avances, meme s'ils ne sont pas vraiment mis en scene aujourd'hui :

- seuil d'entree dans un nouveau mood
- seuil de sortie
- nombre minimal de pages avant changement de mood
- taille de fenetre de lissage / aggregation

Les 8 moods sont :

- epic
- tension
- sadness
- comedy
- romance
- horror
- peaceful
- mystery

Avec une intensite 1 a 3.

#### Extension navigateur

Le redesign doit aussi prendre en compte la popup de l'extension Manga Mood.

Cette popup doit pouvoir montrer :

- la connexion a l'application
- l'activation/desactivation du module
- le port API
- le statut du service
- la page visible
- le dernier mood
- le nombre de pages deja analysees
- le travail en cours
- une note explicative sur l'etat courant
- les pages deja analysees
- les pages suivantes a traiter
- des evenements recents
- un apercu de la page courante
- l'action d'ouverture de l'image courante
- l'etat "pas de page detectee"
- l'etat "extension inactive sur cet onglet"
- l'action d'activation manuelle sur la page courante

### Etats a couvrir explicitement

- aucun profil
- profil vide
- aucune piste
- aucune assignation
- aucune lecture en cours
- import ou export en cours
- export annule
- erreur de fichier manquant
- erreur de preview
- YouTube indisponible / outil manquant
- discovery vide
- discovery en cours
- discovery en erreur
- blacklist vide
- runtime IA non installe
- modele IA non installe
- runtime IA en demarrage
- API extension indisponible
- permission Linux/Wayland manquante
- extension non connectee
- aucune page manga detectee

### Intention de design

Tu peux etre radical. Tu peux rendre le produit plus clair, plus fort, plus vivant, plus elegant, plus simple a lire et plus interessant a utiliser. Mais la simplification ne doit jamais se faire par suppression de scope.

Je veux une proposition qui montre que tu comprends tout le produit, tout en assumant une vision UI/UX personnelle.

Je veux egalement que le resultat assume un travail de matiere et de profondeur dans un esprit liquid glass, tout en restant robuste en theme clair comme en theme sombre.
