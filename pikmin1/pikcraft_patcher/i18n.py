"""Patcher messages in English and French.

The language comes from `--lang`, otherwise from the system (the LC_ALL / LC_MESSAGES / LANG /
LANGUAGE variables, or the Windows display language), and defaults to English.
"""

from __future__ import annotations

import locale
import os
import sys

LANGUAGES = ("en", "fr")

MESSAGES: dict[str, dict[str, str]] = {
    "en": {
        "description": "Turns your own Pikmin (GameCube) disc image into PikCraft.",
        "help_source": "your Pikmin disc image (.iso / .gcm)",
        "help_output": "where to write PikCraft (a new .iso)",
        "help_block_size": "block size in map units (default %(default)s)",
        "help_keep_original": "draw the original terrain under the blocks (terrain-only mode)",
        "help_terrain_only": "don't patch the game code: blocks can't be broken or placed",
        "help_only": "only convert maps whose path contains this text",
        "help_list": "list the map models found and exit",
        "help_lang": "language of the patcher's messages (default: the system language)",
        "help_version": "show the version and exit",
        "help_help": "show this help and exit",
        "version": "%(prog)s {version} (block breaking: {versions})",
        "none": "none",
        "prompt_source": "Path to your Pikmin disc image: ",
        "prompt_output": "Where to save PikCraft (.iso): ",
        "error": "error: {message}",
        "not_pikmin": "{game_id} is not Pikmin (expected one of {expected})",
        "no_maps": "no map models found on this disc",
        "mode_full": "Mode: full (break blocks with D-pad left, place with D-pad right)",
        "mode_terrain": "Mode: terrain only (block breaking needs the PAL disc, GPIP01 revision 0)",
        "skipped": "  {path}: skipped ({reason})",
        "map_done": "  {path}: {columns} columns, {faces}block {block}, {before} -> {after} KiB ({seconds}s)",
        "faces": "{count} faces, ",
        "nothing_converted": "no maps could be converted",
        "done": "Done: {output}  (open it in Dolphin)",
        "not_gamecube": (
            "{path} is not a GameCube disc image. Use a plain .iso/.gcm "
            "(in Dolphin: right-click the game > Convert File > ISO)."
        ),
        "not_on_disc": "{path} is not on the disc",
        "file_not_found": "{path} doesn't exist",
        "refuse_overwrite": "refusing to overwrite the original disc image",
        "not_a_patch": "not a PikCraft executable patch",
        "wrong_version": "this patch is for a different version of the game",
        "corrupt_patch": "corrupt patch",
        "bad_checksum": "patched executable failed its checksum",
        # argparse's own strings
        "usage: ": "usage: ",
        "positional arguments": "positional arguments",
        "options": "options",
        "the following arguments are required: %s": "the following arguments are required: %s",
        "unrecognized arguments: %s": "unrecognized arguments: %s",
        "invalid choice: %(value)r (choose from %(choices)s)": "invalid choice: %(value)r (choose from %(choices)s)",
        "%(prog)s: error: %(message)s\n": "%(prog)s: error: %(message)s\n",
    },
    "fr": {
        "description": "Transforme votre propre image disque de Pikmin (GameCube) en PikCraft.",
        "help_source": "votre image disque de Pikmin (.iso / .gcm)",
        "help_output": "où enregistrer PikCraft (un nouveau .iso)",
        "help_block_size": "taille des blocs en unités de carte (par défaut %(default)s)",
        "help_keep_original": "affiche le terrain d'origine sous les blocs (mode terrain seul)",
        "help_terrain_only": "ne modifie pas le code du jeu : impossible de casser ou poser des blocs",
        "help_only": "ne convertit que les cartes dont le chemin contient ce texte",
        "help_list": "liste les modèles de cartes trouvés, puis quitte",
        "help_lang": "langue des messages du patcher (par défaut : celle du système)",
        "help_version": "affiche la version, puis quitte",
        "help_help": "affiche cette aide, puis quitte",
        "version": "%(prog)s {version} (cassage de blocs : {versions})",
        "none": "aucune",
        "prompt_source": "Chemin de votre image disque de Pikmin : ",
        "prompt_output": "Où enregistrer PikCraft (.iso) : ",
        "error": "erreur : {message}",
        "not_pikmin": "{game_id} n'est pas Pikmin (attendu : {expected})",
        "no_maps": "aucun modèle de carte trouvé sur ce disque",
        "mode_full": "Mode : complet (croix ◀ pour casser un bloc, croix ▶ pour en poser un)",
        "mode_terrain": "Mode : terrain seul (le cassage de blocs nécessite le disque PAL, GPIP01 révision 0)",
        "skipped": "  {path} : ignorée ({reason})",
        "map_done": "  {path} : {columns} colonnes, {faces}bloc de {block}, {before} -> {after} Kio ({seconds} s)",
        "faces": "{count} faces, ",
        "nothing_converted": "aucune carte n'a pu être convertie",
        "done": "Terminé : {output}  (ouvrez-le dans Dolphin)",
        "not_gamecube": (
            "{path} n'est pas une image disque GameCube. Utilisez un simple .iso/.gcm "
            "(dans Dolphin : clic droit sur le jeu > Convertir le fichier > ISO)."
        ),
        "not_on_disc": "{path} n'est pas sur le disque",
        "file_not_found": "{path} n'existe pas",
        "refuse_overwrite": "refus d'écraser l'image disque d'origine",
        "not_a_patch": "ce n'est pas un patch d'exécutable PikCraft",
        "wrong_version": "ce patch est prévu pour une autre version du jeu",
        "corrupt_patch": "patch corrompu",
        "bad_checksum": "l'exécutable modifié ne correspond pas à sa somme de contrôle",
        "usage: ": "utilisation : ",
        "positional arguments": "arguments",
        "options": "options",
        "the following arguments are required: %s": "arguments obligatoires manquants : %s",
        "unrecognized arguments: %s": "arguments non reconnus : %s",
        "invalid choice: %(value)r (choose from %(choices)s)": "choix invalide : %(value)r (choix possibles : %(choices)s)",
        "%(prog)s: error: %(message)s\n": "%(prog)s : erreur : %(message)s\n",
    },
}

_language = "en"


def detect() -> str:
    """The system's language, if it is one the patcher speaks."""
    for var in ("LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"):
        value = os.environ.get(var, "").split(":")[0]
        if value and value not in ("C", "POSIX") and not value.startswith("C."):
            return "fr" if value.lower().startswith("fr") else "en"
    if sys.platform == "win32":
        try:
            import ctypes

            # LANGID: the low 10 bits are the primary language; 0x0C is French
            return "fr" if ctypes.windll.kernel32.GetUserDefaultUILanguage() & 0x3FF == 0x0C else "en"
        except (AttributeError, OSError):
            pass
    try:
        name = locale.getlocale()[0] or ""
    except ValueError:
        name = ""
    return "fr" if name.lower().startswith("fr") else "en"


def set_language(lang: str) -> None:
    global _language
    _language = lang if lang in LANGUAGES else "en"


def language() -> str:
    return _language


def tr(key: str, **kwargs) -> str:
    text = MESSAGES[_language].get(key, MESSAGES["en"].get(key, key))
    return text.format(**kwargs) if kwargs else text


def number(value: float, digits: int = 1) -> str:
    """A number as written in the current language (12.5 / 12,5)."""
    text = f"{value:.{digits}f}" if digits else f"{value:g}"
    return text.replace(".", ",") if _language == "fr" else text


def gettext(message: str) -> str:
    """Stand-in for argparse's gettext: translates the strings argparse prints itself."""
    return MESSAGES[_language].get(message, message)
