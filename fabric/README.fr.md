# PikCraft pour Minecraft (mod Fabric)

[English](README.md) | **Français**

Des Pikmin dans Minecraft Java Edition 26.3 (Fabric). Trouve un Oignon endormi, cueille des Pikmin,
guide-les au sifflet, lance-les sur les monstres et fais-leur rapporter pastilles et ennemis vaincus.

![Une escouade des sept types de Pikmin](../docs/fabric-squad.jpg)

## Installation

1. Installe [Fabric Loader](https://fabricmc.net/use/) 0.19.5 ou plus récent pour Minecraft 26.3, et
   [Fabric API](https://modrinth.com/mod/fabric-api).
2. Mets `pikcraft-<version>.jar` dans ton dossier `mods`.

Le multijoueur marche sur un serveur normal : installe le mod sur le serveur et chez chaque joueur.
Il marche aussi avec les mondes hébergés d'[Essential](https://essential.gg), qui utilisent ce même
multijoueur normal.

## Contenu

- **Sept types de Pikmin :** rouge (résiste au feu, fort), jaune (lancé plus haut, insensible à la
  foudre), bleu (nage, ne se noie jamais), roc (frappe fort, casse le verre et la glace, ne peut pas
  être écrasé), ailé (vole), violet (lourd : porte 10, étourdit ce sur quoi il tombe) et blanc
  (rapide, empoisonne qui le mange). Stades feuille, bourgeon et fleur ; le nectar les fait fleurir.
- **Ton premier Oignon :** un Oignon endormi apparaît à 5 à 15 blocs de toi à ta première connexion,
  avec une petite scène de caméra. Sa couleur est aléatoire : rouge courant, jaune et bleu moins,
  roc et ailé rares, violet et blanc très rares. Touche-le pour le réveiller : il te repousse et
  crache ta première graine. Cueille la pousse avec clic droit.
- **Oignons :** clic droit sur un Oignon pour qu'il te suive (dehors seulement : il ne va pas dans
  les grottes) ; encore clic droit pour qu'il reste. Accroupi + clic droit ouvre le menu de l'Oignon
  pour faire sortir ou rentrer des Pikmin (100 Pikmin sur le terrain au maximum). La nuit, les
  Oignons s'envolent avec leurs Pikmin et n'en produisent plus. D'autres Oignons dorment dans de
  très grandes grottes, et très rarement à la surface.
- **Le sifflet :** 3 lingots de fer et 1 lingot de cuivre (fer, cuivre, fer sur la ligne du milieu,
  fer sous le cuivre). Maintiens « utiliser » pour siffler : le cercle grandit et tous les Pikmin
  dedans te rejoignent.
- **Fleurs à pastille :** frappe la fleur pile au milieu pour faire tomber sa pastille (les Pikmin
  peuvent aussi). Les pastilles vont à un Oignon : la bonne couleur donne plus de graines.
- **Œufs de nectar :** casse-en un et le nectar reste au sol ; les Pikmin le boivent tout seuls.
- **Transport :** pastilles et ennemis se portent, jamais dans l'inventaire. Clic droit main vide
  pour en porter un toi-même.
- **Ennemis :** un monstre que tu tues lâche son butin normalement. Tué par des Pikmin, il tombe et
  son corps est rapporté à un Oignon contre des graines. Pendant le combat, la vie de l'ennemi
  s'affiche en pourcentage. Zombies, squelettes, araignées et creepers chassent aussi les Pikmin.
- **Écrans façon Pikmin :** une roue de vie à la place des cœurs, un compteur d'escouade et un
  soleil pour l'heure, un menu d'Oignon façon Pikmin 3 et un inventaire façon Pikmin 4. Des
  progrès pour le réveil d'un Oignon, la cueillette, les lancers, les livraisons et plus.

## Commandes

| Touche | Action |
| --- | --- |
| R (ou attaquer avec le sifflet en main) | Lancer le Pikmin choisi là où tu regardes |
| V (maintenir, il faut un sifflet) ou utiliser le sifflet | Siffler |
| C | Charger : toute l'escouade fonce là où tu regardes |
| X | Disperser : l'escouade se range par couleur |
| G (Maj+G : précédent) | Choisir le type de Pikmin suivant |
| Clic droit sur une pousse | La cueillir |

Les touches se changent dans **Options > Commandes > Touches > Pikmin**.

## Compiler

Il faut Java 25. `./gradlew build` produit `build/libs/pikcraft-<version>.jar` et lance le test
serveur. `./gradlew runClientGameTest` joue le mod dans un vrai client et enregistre des captures
dans `build/run/clientGameTest/screenshots`.

Textures et sons sont dessinés et synthétisés par `tools/make_assets.py` ; langues, progrès et
recette viennent de `tools/make_data.py`. Aucun fichier de Nintendo ou de Mojang n'est utilisé.
