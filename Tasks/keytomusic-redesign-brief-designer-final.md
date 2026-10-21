# Brief designer - Redesign complet de KeyToMusic

## Demande

Je te demande de repenser completement KeyToMusic.

Je ne veux pas une variation cosmetique de l'existant. Je veux une vraie refonte de fond, en repartant de zero sur l'organisation, la hierarchie, la circulation, la lisibilite, la facon de montrer la complexite et le caractere du produit.

Je te laisse une liberte totale sur :

- le nombre de vues
- la navigation
- l'architecture de l'information
- la facon de regrouper les fonctions
- les composants et patterns d'interface
- le rythme des parcours
- la direction artistique
- les codes graphiques et systemes visuels

Je ne veux pas te dire comment organiser le produit. Je veux te dire ce que le produit doit absolument couvrir, pour que ta proposition puisse etre libre en forme mais exhaustive sur le fond.

Le livrable attendu est un prototype React non fonctionnel, avec donnees mockees. Je veux pouvoir comparer ta vision a celle d'une IA. Je ne veux donc pas seulement des ecrans isoles ou de belles intentions graphiques : je veux un ensemble coherent, credible et complet.

## Direction visuelle souhaitee

Je veux te donner seulement deux directions explicites :

- il doit exister un vrai theme clair et un vrai theme sombre, avec possibilite de bascule
- je veux une sensibilite visuelle "liquid glass", librement interpretee

Cette orientation n'est pas une invitation a faire un effet de mode. Je veux une approche haut de gamme, maitrisee, lisible, avec profondeur, transparence, couches, matiere et relief, mais sans sacrifier l'usage, la clarte ou la densite d'information.

## Nature du produit

KeyToMusic est un outil desktop local pour accompagner la lecture de manga avec une bande-son dynamique.

L'utilisateur y prepare des profils audio, organise des pistes, importe des sons, lie des sons a des touches clavier globales, ajuste leurs points de depart, pilote plusieurs couches sonores en parallele, decouvre de nouvelles musiques et peut activer un systeme d'IA locale qui detecte le mood d'une page de manga pour declencher automatiquement des sons tags.

Le produit comprend aussi une extension navigateur liee a ce systeme de mood detection.

Le redesign doit donc couvrir :

- l'application principale desktop
- le systeme Manga Mood AI
- la popup de l'extension navigateur

## Intention generale

Je veux une proposition qui fasse sentir que le produit est :

- desktop-first
- local-first
- rapide a piloter pendant la lecture
- suffisamment puissant pour gerer plusieurs couches audio et plusieurs systemes en meme temps
- assez clair pour rester comprehensible malgre sa richesse
- assez identitaire pour ne pas ressembler a un outil de productivite banal

Je veux que tu assumes une vraie vision UX. Tu peux etre radical sur l'organisation si cela rend le produit meilleur. La simplification ne doit jamais se faire par suppression de scope.

## Ce que le produit doit absolument couvrir

### 1. Profils

- voir tous les profils
- creer un profil
- selectionner un profil
- renommer un profil
- dupliquer un profil
- supprimer un profil
- gerer le premier lancement sans profil
- gerer le cas d'un profil vide

### 2. Pistes

- plusieurs pistes dans un profil
- creation
- renommage
- suppression
- volume
- etat de lecture
- piste vide
- limite actuelle de 20 pistes par profil

### 3. Sons

- sons locaux
- sons YouTube telecharges et caches localement
- ajout simple
- ajout par lot
- drag and drop
- preview avant validation
- volume individuel
- duree
- momentum
- suppression
- deplacement vers une autre touche

### 4. Momentum et waveform

Chaque son a un point de depart "momentum". Le produit doit permettre de comprendre :

- ce que ce point represente
- la difference entre la valeur actuelle et une suggestion automatique
- la possibilite de comparer, ajuster et appliquer cette suggestion
- la lecture de preview depuis ce point
- la waveform
- la position de lecture quand un son joue

### 5. Bindings et logique clavier

Un binding relie :

- une touche
- une piste
- une liste de sons
- un mode de boucle
- un nom optionnel
- un mood optionnel
- une intensite minimale optionnelle

Le point critique a rendre tres lisible est le suivant :

- un binding est unique par couple `touche + piste`
- une meme touche peut donc exister sur plusieurs pistes
- une meme touche peut donc declencher plusieurs couches en parallele
- un binding multi-sons n'est pas la meme chose qu'un declenchement multi-pistes

Il faut aussi couvrir :

- assignation des touches
- assignation multiple lors d'un ajout multiple
- logique de cyclage si plus de sons que de touches
- renommage d'un binding
- changement de la touche d'un binding entier
- deplacement d'un seul son d'un binding vers une autre touche
- suppression d'un binding entier
- changement de piste cible
- changement de mode de boucle

### 6. Modes de boucle

- `off`
- `single`
- `random`
- `sequential`

### 7. Clavier global et raccourcis

Le clavier est au coeur du produit. Il faut rendre comprehensibles :

- la detection clavier globale en arriere-plan
- son activation / desactivation
- le raccourci global stop all
- le raccourci global d'activation / desactivation de l'auto-momentum
- le raccourci global d'activation / desactivation de la detection clavier
- le cooldown global entre pressions
- les chords et combinaisons multi-touches
- le momentum modifier
- les conflits entre raccourcis, bindings et combinaisons
- l'aide / recapitulatif des raccourcis

Le produit comporte aussi des interactions clavier de productivite qu'il faut prendre en compte dans l'experience :

- undo / redo
- focus de recherche
- selection globale dans la zone des assignations
- navigation clavier dans Discovery
- fermeture / annulation des overlays et modales

### 8. Playback

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
- les erreurs audio ou erreurs de peripherique de sortie

### 9. Recherche, filtre, selection, edition groupee

Il faut couvrir la facon dont l'utilisateur retrouve et manipule ses assignations :

- recherche par touche
- recherche par nom de son
- recherche par nom personnalise
- filtrage par piste
- filtrage par mode de boucle
- filtrage par etat de lecture
- filtrage par mood
- compteur de resultats
- effacement rapide des filtres
- selection simple
- multi-selection
- selection par plage
- selection globale
- edition groupee
- suppression groupee

### 10. Ajout local

Le flow d'ajout local ne se limite pas a choisir un fichier. Il implique :

- file picker
- drag and drop
- support de plusieurs fichiers
- previsualisation
- edition du momentum avant validation
- preview
- choix ou creation d'une piste
- choix du volume initial
- choix du mode de boucle
- affectation de touche(s) dans le meme flow

### 11. Ajout YouTube

Le flow YouTube doit aussi etre couvert :

- coller une URL
- rechercher par texte
- voir les resultats
- voir si un resultat est deja telecharge
- preview stream
- ajouter un resultat a la bibliotheque du profil
- gerer le cas ou `yt-dlp` n'est pas installe
- permettre son installation
- montrer les telechargements et leur progression

### 12. Import de playlist YouTube

- detecter une playlist dans une URL
- choisir une seule video ou toute la playlist
- voir les entrees de la playlist
- selection totale, vide ou partielle
- ajout multiple

### 13. Discovery

Le systeme de decouverte musicale est important dans la valeur produit.

Il faut montrer :

- des suggestions generees a partir des sons deja presents
- des seeds venant de YouTube ou de sons locaux resolves vers YouTube
- un classement selon l'occurrence dans plusieurs Mix YouTube croises
- des suggestions qui affichent titre, chaine, duree, seeds, touche proposee, piste proposee, momentum suggere et etat de predownload
- la generation
- l'annulation
- la progression
- le cas ou des sons locaux sont en cours de resolution
- le cas ou aucune suggestion n'est disponible
- le cas ou aucune seed n'est exploitable
- le cas d'erreur
- le preview
- le seek de preview
- le volume de preview dedie
- la modification de la touche, de la piste et du momentum proposes avant ajout
- l'ajout au profil
- le dislike permanent
- la memorisation de la progression par profil
- le pipeline de predownload autour de la suggestion courante

### 14. Blacklist Discovery

- consultation de la blacklist par profil
- metadonnees des videos blacklistees
- retrait d'un element de la blacklist
- etat vide

### 15. Import / export

- export d'un profil au format `.ktm`
- import d'un profil `.ktm`
- import d'un ancien format `.json`
- progression d'export fichier par fichier
- annulation
- avertissement si fermeture pendant export
- succes, erreur, annulation

### 16. Fichiers manquants

- detection de sons manquants au chargement ou pendant la lecture
- relocalisation d'un fichier local
- retelechargement d'un son YouTube
- suppression de la reference
- ignorer un cas
- ignorer tous les cas restants
- vraie file de traitement

### 17. Feedback, confirmations et etats transitoires

- confirmations d'actions destructives
- notifications succes / erreur / info / warning
- progression d'export
- chargements
- skeletons
- overlays et modales avec cas fermer / annuler / confirmer

### 18. Parametres audio et systeme

- volume master
- duree de crossfade
- cooldown clavier
- fenetre de detection des chords
- momentum modifier
- peripherique de sortie audio
- aide / documentation des raccourcis
- ouverture du dossier data
- ouverture du dossier logs

Il existe aussi des reglages plus avances lies au moteur Mood AI, meme s'ils sont aujourd'hui peu exposes :

- seuil d'entree dans un nouveau mood
- seuil de sortie
- nombre minimal de pages avant changement
- taille de fenetre de lissage / d'analyse

### 19. Linux / Wayland

Sur Linux Wayland, la detection clavier globale peut necessiter une permission supplementaire. Il faut montrer :

- l'etat pret
- l'etat manquant
- l'etat en verification
- l'etat activable automatiquement
- un flow d'auto-fix
- un etat de recheck

### 20. Manga Mood AI dans l'application principale

Le systeme Manga Mood AI fait partie du produit principal.

Il faut couvrir :

- le toggle general
- le runtime local installable
- le modele installable
- les etats non installe, telechargement, pret, demarrage, actif, erreur, arrete
- le port API local configurable
- l'etat du runtime
- l'etat de l'API extension
- le lancement / arret du runtime
- le telechargement du modele avec progression
- les bindings tags avec un mood
- l'intensite minimale optionnelle
- la difference entre le dernier mood brut detecte et le mood actuellement retenu / joue

Les 8 moods sont :

- epic
- tension
- sadness
- comedy
- romance
- horror
- peaceful
- mystery

Avec intensite 1 a 3.

### 21. Popup de l'extension navigateur Manga Mood

Le redesign doit aussi prendre en compte la popup d'extension.

Cette popup doit pouvoir montrer :

- la connexion a l'application
- l'activation / desactivation du module
- le port API
- le statut du service
- la page visible
- le dernier mood
- le nombre de pages deja analysees
- `Current Work`
- `Current Note`
- `Already Analyzed`
- `Next Pages`
- des evenements recents
- un apercu de la page courante
- l'action d'ouverture de l'image courante
- l'etat "page en cours d'analyse"
- l'etat "page deja analysee"
- l'etat "pas de page detectee"
- l'etat "extension inactive sur cet onglet"
- l'action d'activation manuelle sur la page courante

## Etats a couvrir explicitement

- aucun profil
- profil vide
- aucune piste
- aucune assignation
- aucune lecture en cours
- plusieurs pistes jouent en meme temps
- changement de profil pendant lecture
- import ou export en cours
- export annule
- fermeture pendant export
- fichier manquant
- preview indisponible
- YouTube indisponible / outil manquant
- Discovery sans seeds
- Discovery vide
- Discovery en cours
- Discovery avec resolution de sons locaux
- Discovery en erreur
- suggestion predownloaded
- suggestion en telechargement
- suggestion en erreur
- blacklist vide
- runtime IA non installe
- modele IA non installe
- runtime IA en demarrage
- runtime IA non lance
- API extension indisponible
- detection clavier desactivee
- conflit de raccourcis
- permission Linux/Wayland manquante
- extension non connectee
- aucune page manga detectee

## Ce que je veux dans la proposition

- une vraie vision UX, pas un relooking
- une comprehension exhaustive du produit
- une capacite a rendre visible la complexite sans la rendre confuse
- une forte coherence entre coeur produit, systemes secondaires et extension
- un travail visuel ambitieux et maitrise
- une vraie qualite en theme clair comme en theme sombre
- une interpretation liquid glass haut de gamme, lisible et profonde

## Ce que je ne veux pas

- une copie plus jolie de l'existant
- une reduction de scope
- des fonctions importantes oubliees
- des ecrans statiques sans logique produit
- une interface "dashboard SaaS"
- un effet liquid glass purement decoratif qui degrade l'usage

Je veux une proposition qui montre que tu comprends tout le produit, tout en assumant une vision UI/UX personnelle et forte.
