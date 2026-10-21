# Prompt IA - Redesign complet de KeyToMusic

## Mission

Tu vas redesigner de zero tout le produit KeyToMusic.

Ton livrable ne doit pas etre une image. Je veux un prototype React non fonctionnel cote backend, avec donnees mockees, qui montre l'ensemble du produit, de ses flux, de ses etats, de ses surfaces secondaires et de ses cas limites.

Tu as une liberte totale sur :

- le nombre de vues
- la navigation
- l'architecture de l'information
- la facon de regrouper les fonctions
- la hierarchie UX
- la direction artistique
- les composants
- les animations
- la relation entre les espaces principaux et secondaires

Je ne veux pas une copie de l'application actuelle. Je ne veux pas non plus un simple reskin. Je veux une vraie vision produit, UI et UX, capable de reorganiser radicalement l'experience tout en couvrant absolument tout le scope.

## Direction a respecter

- le prototype doit integrer un vrai switch theme clair / theme sombre
- les deux themes doivent sembler natifs, travailles et complets, pas juste inverses
- je veux une direction visuelle "liquid glass", librement interpretee dans ton propre langage visuel
- cette direction doit rester lisible, sobrement maitrisee et credible dans un produit dense
- il faut du relief, de la profondeur, de la matiere, de la transparence et des couches visuelles, sans tomber dans l'effet gadget

## Philosophie de redesign

- tu peux tout reconfigurer
- tu peux fusionner, separer, redistribuer ou recontextualiser les fonctions
- tu peux redefinir les priorites visuelles et les parcours
- tu peux creer des espaces de type dashboard, atelier, inspecteur, bibliotheque, timeline, overlay, panneau contextuel, vue immersive, assistant, etc.
- tu ne dois supprimer aucune capacite importante
- tu ne dois pas rendre invisibles les fonctions secondaires sous pretexte de simplification
- tu ne dois pas te limiter a "faire joli"

Le produit est desktop-first, local-first, et pense pour un usage rapide pendant la lecture d'un manga. Il doit donner le sentiment d'un outil creatif, nerveux, puissant et fiable, pas d'un dashboard SaaS generique.

## Ce qu'est KeyToMusic

KeyToMusic est un outil desktop local qui sert a preparer et piloter une bande-son de lecture de manga.

L'utilisateur peut :

- creer plusieurs profils
- organiser des pistes audio
- importer des sons locaux ou YouTube
- assigner des sons a des touches clavier globales
- jouer plusieurs couches sonores en parallele
- regler le point de depart "momentum" de chaque son
- decouvrir automatiquement de nouvelles musiques
- utiliser un systeme local "Manga Mood AI" qui detecte l'ambiance d'une page de manga et declenche automatiquement des sons relies a des moods

Le produit inclut aussi une extension navigateur liee au module Manga Mood. Le redesign doit donc couvrir l'application desktop et la popup d'extension.

## Objets metier a rendre clairs

- Profil : configuration complete de lecture. Il contient des pistes, des sons, des bindings, une logique de decouverte, une blacklist Discovery et ses propres etats.
- Piste : couche audio logique, par exemple OST, ambiance, SFX, voix. Elle a au minimum un nom, un volume et un etat de lecture.
- Son : asset audio individuel avec nom, source, volume, duree, momentum et parfois un identifiant YouTube resolu.
- Source audio : fichier local ou son YouTube telecharge et mis en cache localement.
- Binding : relation entre une touche, une piste et une liste de sons. Il a aussi un mode de boucle, un nom optionnel, un mood optionnel et une intensite minimale optionnelle.
- Suggestion Discovery : proposition de nouveau son issue du moteur de decouverte, avec metadonnees, seeds, affectation proposee, momentum suggere et etat de predownload.
- Moods AI : `epic`, `tension`, `sadness`, `comedy`, `romance`, `horror`, `peaceful`, `mystery`, chacun avec intensite 1 a 3.

Point critique a faire comprendre visuellement : un binding est unique par couple `touche + piste`. Donc une meme touche peut exister sur plusieurs pistes a la fois. Un binding multi-sons n'est pas la meme chose qu'un declenchement multi-pistes.

## Scope produit a couvrir

### 1. Profils

- voir tous les profils
- creer un profil
- selectionner un profil
- renommer un profil
- dupliquer un profil
- supprimer un profil
- gerer le premier lancement sans aucun profil
- gerer le profil vide comme un vrai etat produit

### 2. Pistes

- un profil peut contenir plusieurs pistes
- creer une piste
- renommer une piste
- regler le volume d'une piste
- supprimer une piste
- voir quelles pistes jouent
- gerer une piste vide
- limite actuelle : 20 pistes par profil

### 3. Sons

- ajout de sons locaux
- drag and drop de fichiers audio
- ajout par lot de plusieurs fichiers
- ajout de sons depuis YouTube
- preview avant validation
- volume individuel
- duree
- momentum
- suppression d'un son
- deplacement d'un son vers une autre touche

### 4. Momentum, waveform et preview

- chaque son a un point de depart "momentum"
- il faut voir une waveform
- l'outil detecte automatiquement un momentum suggere
- la suggestion doit pouvoir etre comprise, comparee et appliquee
- l'utilisateur doit pouvoir ajuster le momentum manuellement
- il doit exister une preview depuis le point de momentum
- la waveform peut aussi montrer la position de lecture quand un son joue

### 5. Assignation clavier

- assigner des touches a des sons
- assigner une ou plusieurs touches lors d'un ajout multiple
- si plusieurs sons sont ajoutes pour moins de touches que de sons, l'affectation peut cycler
- un binding est defini par un couple touche + piste
- une meme touche peut exister sur plusieurs pistes
- une meme touche peut declencher plusieurs pistes en parallele
- un binding peut contenir plusieurs sons
- un binding peut avoir un nom personnalise
- changer la touche d'un binding entier
- deplacer un seul son d'un binding vers une autre touche
- supprimer un binding entier
- modifier la piste cible d'un binding
- modifier le mode de boucle d'un binding

### 6. Modes de boucle

- `off`
- `single`
- `random`
- `sequential`

### 7. Clavier global et raccourcis

- detection clavier globale, y compris en arriere-plan
- activation ou desactivation de cette detection
- raccourci global "stop all"
- raccourci global pour activer ou desactiver l'auto-momentum
- raccourci global pour activer ou desactiver la detection clavier
- cooldown global entre pressions
- support des chords et combinaisons multi-touches
- "momentum modifier" pour lancer un son depuis son point de momentum
- conflits entre raccourcis globaux, touches assignees et combinaisons
- aide / recapitulatif des raccourcis
- interactions clavier de productivite : undo, redo, focus recherche, selection globale, navigation Discovery, fermeture et annulation d'overlays

### 8. Lecture audio

- volume master global
- volume par piste
- volume par son
- crossfade entre sons d'une meme piste
- lecture simultanee sur plusieurs pistes
- stop d'une piste
- stop de tout
- liste des sons actuellement en lecture
- progression de lecture
- seek sur un son en cours
- etat "rien ne joue"
- erreurs audio ou erreurs de peripherique de sortie

### 9. Recherche, filtre et organisation

- recherche par touche
- recherche par nom de son
- recherche par nom personnalise de binding
- filtrage par piste
- filtrage par mode de boucle
- filtrage par etat de lecture
- filtrage par mood
- affichage du nombre d'elements correspondants
- effacement rapide de la recherche et des filtres

### 10. Selection et edition groupee

- selection simple
- multi-selection
- selection par plage
- selection globale
- edition groupee de plusieurs bindings
- suppression groupee
- modification groupee de la piste
- modification groupee du mode de boucle
- etat dedie quand plusieurs elements sont selectionnes

### 11. Ajout local de sons

- file picker
- drag and drop
- support multi-fichiers
- previsualisation des fichiers ajoutes
- edition du momentum avant validation
- preview avant import final
- choix ou creation d'une piste pendant le flow
- choix du volume initial
- choix du mode de boucle
- affectation de touche(s) dans le meme flow

### 12. Ajout YouTube

- coller une URL YouTube
- rechercher YouTube par texte
- voir une liste de resultats
- voir si un resultat est deja telecharge
- preview stream d'un resultat
- ajout d'un resultat a la bibliotheque du profil
- gestion du cas ou `yt-dlp` n'est pas installe
- possibilite d'installer `yt-dlp`
- affichage d'un vrai etat de telechargement et de progression

### 13. Import de playlist YouTube

- recuperer une playlist si l'URL en contient une
- choisir une seule video ou toute la playlist
- voir les entrees de la playlist
- selectionner tout, rien ou une partie
- ajouter plusieurs videos choisies

### 14. Discovery musical

Le moteur Discovery fait partie du coeur produit.

- les suggestions viennent des sons deja presents dans le profil
- les seeds peuvent venir de sons YouTube ou de sons locaux resolves automatiquement vers YouTube
- les suggestions sont classees selon leur occurrence dans plusieurs Mix YouTube croises
- les suggestions deja possedees, deja vues, dismiss ou dislike ne doivent pas revenir normalement
- chaque suggestion doit montrer titre, chaine, duree, seeds d'origine, touche proposee, piste proposee, momentum suggere et etat de predownload
- lancer ou relancer la decouverte
- annuler une generation
- voir la progression
- voir le cas ou des seeds locaux sont en cours de resolution
- voir le cas ou aucune suggestion n'est disponible
- voir le cas ou aucune seed n'est exploitable
- voir le cas d'erreur
- preview une suggestion
- seek la preview
- regler un volume de preview dedie, distinct du master
- modifier la touche suggeree avant ajout
- modifier la piste suggeree avant ajout
- modifier le momentum suggere avant ajout
- ajouter la suggestion au profil
- dislike permanent pour le profil
- progression de decouverte memorisee par profil
- pipeline de predownload autour de la suggestion courante

### 15. Blacklist Discovery

- consulter les videos blacklistees pour un profil
- voir leurs metadonnees
- retirer un element de la blacklist
- gerer l'etat vide

### 16. Import / export

- export d'un profil au format `.ktm`
- import d'un profil `.ktm`
- import d'un ancien format legacy `.json`
- progression d'export fichier par fichier
- annulation d'export
- avertissement si la fenetre se ferme pendant l'export
- affichage clair des succes, echecs et annulations

### 17. Gestion des fichiers manquants

- au chargement d'un profil ou pendant la lecture, des sons peuvent manquer
- si le son etait local, l'utilisateur peut le relocaliser
- si le son venait de YouTube, l'utilisateur peut le retelecharger
- l'utilisateur peut supprimer la reference
- il peut ignorer un cas
- il peut ignorer tous les cas restants
- il existe donc une vraie file de gestion des sons manquants

### 18. Feedback, confirmations et overlays

- confirmations d'actions destructives
- toasts / notifications succes, erreur, info, warning
- progression d'export
- chargements
- skeletons
- cas fermer / annuler / confirmer

### 19. Parametres audio et systeme

- volume master
- duree du crossfade
- cooldown clavier
- fenetre de detection des chords
- momentum modifier
- peripherique de sortie audio
- aide / documentation des raccourcis
- ouverture du dossier data
- ouverture du dossier logs
- reglages avances du moteur Mood AI : seuil d'entree, seuil de sortie, nombre minimal de pages avant changement, taille de fenetre d'analyse / de lissage

### 20. Linux / Wayland

- sur Linux Wayland, la detection clavier globale peut necessiter une permission systeme supplementaire
- montrer si l'acces background est pret, manquant, en verification ou activable automatiquement
- il existe un flow d'auto-fix
- il existe un etat de recheck

### 21. Manga Mood AI dans l'application principale

Le systeme Manga Mood AI fait partie du produit, pas d'un module cache.

- toggle general d'activation
- runtime IA local installable separement
- modele installable separement
- etats : non installe, en telechargement, pret, demarrage, actif, erreur, arrete
- port API local configurable
- l'application expose l'etat du runtime et l'etat de l'API extension
- l'utilisateur peut lancer ou arreter le runtime
- l'application peut telecharger le modele avec progression
- les bindings peuvent etre tags avec un mood
- chaque binding peut avoir une intensite minimale requise
- quand l'IA detecte un mood engage, les bindings correspondants peuvent etre joues automatiquement
- l'application doit montrer la difference entre le dernier mood brut detecte et le mood actuellement retenu / joue

### 22. Popup de l'extension navigateur Manga Mood

Le redesign doit aussi couvrir la popup d'extension.

Cette extension :

- se connecte a l'application locale
- peut activer ou desactiver la mood detection
- peut regler le port API local
- montre l'etat de connexion
- montre le statut du service
- montre la page manga visible
- montre le dernier mood detecte
- montre combien de pages sont deja analysees
- montre `Current Work`
- montre `Current Note`
- montre `Already Analyzed`
- montre `Next Pages`
- montre des evenements recents
- montre un apercu de la page courante
- montre si la page est en cours d'analyse ou deja analysee
- permet d'ouvrir l'image courante
- permet d'activer l'extension sur l'onglet courant si la page n'est pas auto-detectee

## Etats et situations a ne surtout pas oublier

- premier lancement sans profil
- profil vide
- profil avec pistes mais aucune assignation
- aucune piste
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
- Discovery sans seeds
- Discovery en cours
- Discovery avec resolution de sons locaux
- Discovery vide
- Discovery avec erreur
- suggestion predownloaded
- suggestion en telechargement
- suggestion en erreur
- blacklist vide
- extension non connectee a l'app
- API extension non prete
- modele IA non installe
- runtime IA non lance
- runtime IA en demarrage
- detection clavier desactivee
- conflit de raccourcis
- permission Linux manquante
- aucune page manga detectee
- extension inactive sur l'onglet courant

## Niveau d'ambition attendu

Je veux une proposition qui donne le sentiment que tu comprends toute la complexite du produit, mais que tu sais la rendre fluide, desirable et lisible.

Je veux voir :

- une vraie vision d'ensemble
- une vraie hierarchie produit
- une vraie mise en scene des systemes complexes
- une vraie personnalite visuelle
- une vraie coherence entre les surfaces principales, secondaires et systemes
- une vraie qualite de theme clair et de theme sombre
- une vraie exploration "liquid glass" mature et haut de gamme

## Ce que je ne veux pas

- une copie de l'application actuelle
- un simple reskin
- une reduction du scope
- des fonctions importantes cachees ou oubliees
- une UI generique de dashboard SaaS
- une reponse uniquement sous forme d'images
- une interface spectaculaire mais peu lisible
- un liquid glass superficiel qui casse l'utilisabilite

## Format de livrable

Je veux un prototype React statique mais credible, avec donnees mockees, montrant tout le produit et ses etats importants.

Tu es libre sur la structure. Tu n'es pas libre de perdre du scope.
