# PikCraft pour Pikmin 1 (GameCube)

[English](README.md) | **Français**

PikCraft tourne dans le vrai moteur de Pikmin 1 de Nintendo. Le patcher reconstruit le modèle de
chaque niveau en blocs façon Minecraft et écrit une nouvelle image disque, à jouer dans
[Dolphin](https://fr.dolphin-emu.org).

> Testé avec le disque PAL (GPIP01) dans Dolphin : le Site du Crash est jouable, et on peut y
> casser et poser des blocs. Les autres zones sont converties de la même façon, mais n'ont pas
> encore été parcourues.

## Commandes

| Bouton | Action |
| --- | --- |
| Croix ◀ | Casse le bloc du dessus sous le curseur |
| Croix ▶ | Pose un bloc à cet endroit (du type du dernier bloc cassé ; de la terre au départ) |

Tout le reste se joue comme Pikmin. Olimar et les Pikmin descendent dans les trous que vous creusez
et montent sur les blocs que vous posez. Le terrain revient à son état d'origine chaque jour.
Casser et poser nécessite le disque PAL (GPIP01, révision 0) ; avec les autres versions, le patcher
construit le terrain en blocs sans ces actions.

## Ce qu'il fait

- Il sonde la surface de collision de chaque niveau sur une grille de blocs, celle sur laquelle
  marchent Olimar et les Pikmin. Le sol devient herbe, terre et pierre ; la roche devient pierre ;
  le bois devient planches ; la boue reste de la boue. Le fond des mares (marqué comme eau dans les
  collisions) devient du sable, rempli d'eau jusqu'à la rive.
- **Mode complet** (PAL) : l'exécutable du jeu reçoit le code de PikCraft (`mod/`). Il dessine
  lui-même les blocs à chaque image, répond aux collisions de la carte à partir de la carte de
  hauteur des blocs (des pentes entre les centres des blocs : une marche d'un bloc est praticable,
  les falaises plus hautes sont des murs), et gère le cassage et la pose. La carte de hauteur est
  stockée dans chaque modèle de carte, dans un bloc de données supplémentaire.
- **Mode terrain seul** (autres versions) : les faces visibles sont fusionnées directement dans le
  modèle de la carte ; les collisions d'origine sont conservées.
- Les textures sont du pixel art original en 16×16. Aucun élément de Mojang n'est utilisé.
- Les ennemis, les pièces du vaisseau, les trajets de transport et l'éclairage ne changent pas :
  chaque niveau reste faisable.

## Utilisation

Il vous faut votre propre image disque de Pikmin : NTSC-U (GPIE01), NTSC-J (GPIJ01) ou PAL
(GPIP01), en simple `.iso`/`.gcm`. Si la vôtre est en `.rvz` ou `.gcz`, convertissez-la dans
Dolphin : clic droit sur le jeu, **Convertir le fichier…**, puis **ISO**.

```
pikcraft-patcher "Pikmin.iso" "PikCraft.iso"
```

Sans arguments, il vous demande les chemins. Votre image d'origine n'est jamais modifiée.

| Option | Effet |
| --- | --- |
| `--block-size 20` | Taille des blocs en unités de carte ; plus petit = plus détaillé. Une carte qui demanderait trop de sommets utilise automatiquement des blocs plus grands. |
| `--only <texte>` | Ne convertit que les cartes dont le chemin contient ce texte. |
| `--list` | Liste les modèles de cartes trouvés, puis quitte. |
| `--terrain-only` | Ne modifie pas le code du jeu (pas de cassage ni de pose). |
| `--keep-original` | Mode terrain seul, en affichant aussi le terrain d'origine pour comparer. |
| `--lang fr` / `--lang en` | Langue des messages du patcher (par défaut : celle de votre système). |

Pour jouer en français, utilisez le disque PAL et réglez **Config > GameCube > Langue du système**
sur **Français** dans Dolphin, ou changez la langue dans **Options** sur l'écran titre du jeu.

Depuis les sources (Python 3.10+, sans dépendances) : `python -m pikcraft_patcher …` ; tests :
`python -m unittest discover -s tests -t .`

## Mentions légales

Ce dépôt et ses releases ne contiennent aucun code, aucune donnée ni aucun élément de Nintendo. Le
patcher ne fait que lire l'image disque que vous lui fournissez ; utilisez votre propre copie du
jeu. Le patch de code (`pikcraft_patcher/data/*.dolpatch`) ne contient que des différences avec
votre propre exécutable, il est donc inutilisable sans lui ; `mod/build_dolpatch.sh` le reconstruit
à partir de la [décompilation projectPiki](https://github.com/projectPiki/pikmin) et des sources
de `mod/`.

## Fonctionnement interne

- `pikcraft_patcher/gcdisc.py` lit la table des fichiers du disque et écrit une copie modifiée. Il
  ajoute les fichiers remplacés en fin d'image et met à jour leurs entrées.
- `pikcraft_patcher/modfile.py` lit et écrit le format de modèle `.mod`, d'après la
  [décompilation projectPiki](https://github.com/projectPiki/pikmin) (`BaseShape::read`).
- `pikcraft_patcher/voxel.py` transforme les triangles de collision en grille de blocs et en
  maillage de faces fusionnées.
- `pikcraft_patcher/mapbuild.py` greffe les maillages, textures et matériaux des blocs dans le
  modèle de la carte. Chaque matériau de bloc est une copie d'un matériau de la carte (opaque, ou
  translucide pour l'eau) : le moteur de rendu ne voit rien de nouveau. Il respecte aussi le sens
  des triangles de la carte.
- `pikcraft_patcher/dolpatch.py` crée et applique le patch de l'exécutable (différences mot à mot
  façon bsdiff, vérifiées par SHA-1 des deux côtés).
- `mod/PikCraft.h` et `mod/pikcraft.inl` sont le code du jeu : rendu des blocs avec GX, collisions
  tirées de la carte de hauteur derrière `BaseShape::getCollTris`, cassage et pose.
  `mod/decomp-hooks.patch` contient les quelques lignes qui les branchent dans le jeu.
- `pikcraft_patcher/i18n.py` contient les messages du patcher en anglais et en français.
- `pikcraft_patcher/textures.py` contient les textures procédurales des blocs et les encodeurs
  GameCube RGB565/RGB5A3.
