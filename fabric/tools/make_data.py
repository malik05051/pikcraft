#!/usr/bin/env python3
"""Writes the mod's language files (English and French), its advancements and the whistle recipe.

Run from the fabric/ folder: python3 tools/make_data.py
"""
import json
import os

RES = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src", "main", "resources")
TYPES = ["red", "yellow", "blue", "rock", "winged", "purple", "white"]

TYPE_NAMES = {
    "red": ("Red", "rouge"), "yellow": ("Yellow", "jaune"), "blue": ("Blue", "bleu"), "rock": ("Rock", "roc"),
    "winged": ("Winged", "ailé"), "purple": ("Purple", "violet"), "white": ("White", "blanc"),
}

# id: (parent, icon, frame, criteria events, EN title, EN description, FR title, FR description)
ADVANCEMENTS = [
    ("awaken_onion", "root", "pikcraft:onion_spawn_egg", "task", ["awaken_onion"],
     "Strange Encounter", "Wake up a sleeping Onion", "Rencontre étrange", "Réveiller un Oignon endormi"),
    ("pluck", "awaken_onion", "pikcraft:red_pikmin_spawn_egg", "task", ["pluck"],
     "Pluck!", "Pluck your first Pikmin from the ground", "Cueillette !", "Cueillir ton premier Pikmin"),
    ("whistle", "pluck", "pikcraft:whistle", "task", ["whistle"],
     "Over Here!", "Call Pikmin with the whistle", "Par ici !", "Appeler des Pikmin avec le sifflet"),
    ("throw", "pluck", "pikcraft:yellow_pikmin_spawn_egg", "task", ["throw"],
     "Flying Lessons", "Throw a Pikmin", "Leçon de vol", "Lancer un Pikmin"),
    ("charge", "throw", "minecraft:iron_sword", "task", ["charge"],
     "Charge!", "Send your whole squad charging", "À l'assaut !", "Lancer toute ton escouade à l'assaut"),
    ("deliver_pellet", "pluck", "pikcraft:pellet_posy_spawn_egg", "task", ["deliver_pellet"],
     "Pellet Delivery", "Get a pellet carried back to an Onion", "Livraison de pastille", "Faire rapporter une pastille à un Oignon"),
    ("deliver_corpse", "throw", "minecraft:rotten_flesh", "task", ["deliver_corpse"],
     "Circle of Life", "Have Pikmin carry a defeated enemy home", "Le cycle de la vie", "Faire rapporter un ennemi vaincu par des Pikmin"),
    ("nectar", "pluck", "pikcraft:nectar_egg_spawn_egg", "task", ["nectar"],
     "Sweet Nectar", "A Pikmin drinks nectar and blooms", "Doux nectar", "Un Pikmin boit du nectar et fleurit"),
    ("onion_follow", "awaken_onion", "minecraft:lead", "task", ["onion_follow"],
     "Moving Day", "Get an Onion to follow you", "Déménagement", "Faire suivre un Oignon"),
    ("second_onion", "onion_follow", "pikcraft:onion_spawn_egg", "goal", ["second_onion"],
     "Onion Collector", "Wake a second Onion", "Collectionneur d'Oignons", "Réveiller un deuxième Oignon"),
    ("all_types", "second_onion", "pikcraft:white_pikmin_spawn_egg", "challenge", ["pikmin_" + t for t in TYPES],
     "Rainbow Squad", "Pluck a Pikmin of all seven types", "Escouade arc-en-ciel", "Cueillir un Pikmin de chacun des sept types"),
]

EN = {
    "itemGroup.pikcraft": "PikCraft",
    "item.pikcraft.whistle": "Whistle",
    "item.pikcraft.onion_spawn_egg": "Onion Spawn Egg",
    "item.pikcraft.pellet_posy_spawn_egg": "Pellet Posy Spawn Egg",
    "item.pikcraft.nectar_egg_spawn_egg": "Nectar Egg Spawn Egg",
    "entity.pikcraft.onion": "Onion",
    "entity.pikcraft.seed": "Pikmin Seed",
    "entity.pikcraft.pellet": "Pellet",
    "entity.pikcraft.corpse": "Defeated Enemy",
    "entity.pikcraft.pellet_posy": "Pellet Posy",
    "entity.pikcraft.nectar_egg": "Nectar Egg",
    "entity.pikcraft.nectar": "Nectar",
    "key.category.pikcraft.pikmin": "Pikmin",
    "key.pikcraft.throw": "Throw Pikmin",
    "key.pikcraft.whistle": "Whistle (hold)",
    "key.pikcraft.charge": "Charge",
    "key.pikcraft.dismiss": "Dismiss",
    "key.pikcraft.next_type": "Next Pikmin type (Shift: previous)",
    "message.pikcraft.onion_follow": "The Onion follows you",
    "message.pikcraft.onion_stay": "The Onion stays here",
    "message.pikcraft.onion_night": "The Onion has flown up for the night",
    "message.pikcraft.onion_no_cave": "The Onion can't go into caves",
    "screen.pikcraft.onion": "Onion",
    "screen.pikcraft.onion.title": "%s Onion",
    "screen.pikcraft.onion.in_onion": "Onion",
    "screen.pikcraft.onion.squad": "Squad",
    "screen.pikcraft.onion.field": "In the field: %s / %s",
    "screen.pikcraft.onion.confirm": "OK",
    "subtitles.pikcraft.whistle": "Whistle blows",
    "subtitles.pikcraft.pikmin.pluck": "Pikmin plucked",
    "subtitles.pikcraft.pikmin.throw": "Pikmin thrown",
    "subtitles.pikcraft.pikmin.hey": "Pikmin calls",
    "subtitles.pikcraft.pikmin.attack": "Pikmin attacks",
    "subtitles.pikcraft.pikmin.die": "Pikmin dies",
    "subtitles.pikcraft.pikmin.carry": "Pikmin carry",
    "subtitles.pikcraft.pikmin.drink": "Pikmin drinks",
    "subtitles.pikcraft.onion.awaken": "Onion wakes",
    "subtitles.pikcraft.onion.seed": "Onion spits a seed",
    "subtitles.pikcraft.onion.suck": "Onion takes something in",
    "subtitles.pikcraft.onion.fly": "Onion flies",
    "subtitles.pikcraft.posy.hit": "Pellet Posy shakes",
    "subtitles.pikcraft.pellet.drop": "Pellet drops",
    "subtitles.pikcraft.egg.break": "Nectar Egg breaks",
    "advancements.pikcraft.root.title": "PikCraft",
    "advancements.pikcraft.root.description": "A tiny explorer on a world of blocks",
}

FR = {
    "itemGroup.pikcraft": "PikCraft",
    "item.pikcraft.whistle": "Sifflet",
    "item.pikcraft.onion_spawn_egg": "Œuf d'apparition d'Oignon",
    "item.pikcraft.pellet_posy_spawn_egg": "Œuf d'apparition de fleur à pastille",
    "item.pikcraft.nectar_egg_spawn_egg": "Œuf d'apparition d'œuf de nectar",
    "entity.pikcraft.onion": "Oignon",
    "entity.pikcraft.seed": "Graine de Pikmin",
    "entity.pikcraft.pellet": "Pastille",
    "entity.pikcraft.corpse": "Ennemi vaincu",
    "entity.pikcraft.pellet_posy": "Fleur à pastille",
    "entity.pikcraft.nectar_egg": "Œuf de nectar",
    "entity.pikcraft.nectar": "Nectar",
    "key.category.pikcraft.pikmin": "Pikmin",
    "key.pikcraft.throw": "Lancer un Pikmin",
    "key.pikcraft.whistle": "Siffler (maintenir)",
    "key.pikcraft.charge": "Charger",
    "key.pikcraft.dismiss": "Disperser",
    "key.pikcraft.next_type": "Type de Pikmin suivant (Maj : précédent)",
    "message.pikcraft.onion_follow": "L'Oignon te suit",
    "message.pikcraft.onion_stay": "L'Oignon reste ici",
    "message.pikcraft.onion_night": "L'Oignon s'est envolé pour la nuit",
    "message.pikcraft.onion_no_cave": "L'Oignon ne peut pas aller dans les grottes",
    "screen.pikcraft.onion": "Oignon",
    "screen.pikcraft.onion.title": "Oignon %s",
    "screen.pikcraft.onion.in_onion": "Oignon",
    "screen.pikcraft.onion.squad": "Escouade",
    "screen.pikcraft.onion.field": "Sur le terrain : %s / %s",
    "screen.pikcraft.onion.confirm": "OK",
    "subtitles.pikcraft.whistle": "Coup de sifflet",
    "subtitles.pikcraft.pikmin.pluck": "Pikmin cueilli",
    "subtitles.pikcraft.pikmin.throw": "Pikmin lancé",
    "subtitles.pikcraft.pikmin.hey": "Un Pikmin appelle",
    "subtitles.pikcraft.pikmin.attack": "Un Pikmin attaque",
    "subtitles.pikcraft.pikmin.die": "Un Pikmin meurt",
    "subtitles.pikcraft.pikmin.carry": "Des Pikmin transportent",
    "subtitles.pikcraft.pikmin.drink": "Un Pikmin boit",
    "subtitles.pikcraft.onion.awaken": "Un Oignon se réveille",
    "subtitles.pikcraft.onion.seed": "Un Oignon crache une graine",
    "subtitles.pikcraft.onion.suck": "Un Oignon aspire quelque chose",
    "subtitles.pikcraft.onion.fly": "Un Oignon s'envole",
    "subtitles.pikcraft.posy.hit": "Une fleur à pastille tremble",
    "subtitles.pikcraft.pellet.drop": "Une pastille tombe",
    "subtitles.pikcraft.egg.break": "Un œuf de nectar se casse",
    "advancements.pikcraft.root.title": "PikCraft",
    "advancements.pikcraft.root.description": "Un petit explorateur sur un monde de blocs",
}


def write(path, data):
    full = os.path.join(RES, path)
    os.makedirs(os.path.dirname(full), exist_ok=True)
    with open(full, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
        f.write("\n")


def main():
    for t in TYPES:
        en, fr = TYPE_NAMES[t]
        EN["entity.pikcraft.%s_pikmin" % t] = "%s Pikmin" % en
        FR["entity.pikcraft.%s_pikmin" % t] = "Pikmin %s" % fr
        EN["item.pikcraft.%s_pikmin_spawn_egg" % t] = "%s Pikmin Spawn Egg" % en
        FR["item.pikcraft.%s_pikmin_spawn_egg" % t] = "Œuf d'apparition de Pikmin %s" % fr
        EN["pikmin.pikcraft." + t] = en
        FR["pikmin.pikcraft." + t] = fr

    write("data/pikcraft/advancement/root.json", {
        "criteria": {"tick": {"trigger": "minecraft:tick"}},
        "display": {
            "icon": {"id": "pikcraft:whistle"},
            "title": {"translate": "advancements.pikcraft.root.title"},
            "description": {"translate": "advancements.pikcraft.root.description"},
            "background": "minecraft:gui/advancements/backgrounds/husbandry",
            "show_toast": False,
            "announce_to_chat": False,
        },
        "requirements": [["tick"]],
    })
    for (name, parent, icon, frame, events, en_title, en_desc, fr_title, fr_desc) in ADVANCEMENTS:
        criteria = {e: {"trigger": "pikcraft:event", "conditions": {"event": e}} for e in events}
        write("data/pikcraft/advancement/%s.json" % name, {
            "parent": "pikcraft:" + parent,
            "criteria": criteria,
            "display": {
                "icon": {"id": icon},
                "title": {"translate": "advancements.pikcraft.%s.title" % name},
                "description": {"translate": "advancements.pikcraft.%s.description" % name},
                "frame": frame,
            },
            "requirements": [[e] for e in events],
        })
        EN["advancements.pikcraft.%s.title" % name] = en_title
        EN["advancements.pikcraft.%s.description" % name] = en_desc
        FR["advancements.pikcraft.%s.title" % name] = fr_title
        FR["advancements.pikcraft.%s.description" % name] = fr_desc

    # iron in the middle row's sides and below, copper in the centre
    write("data/pikcraft/recipe/whistle.json", {
        "type": "minecraft:crafting_shaped",
        "category": "equipment",
        "key": {"I": "minecraft:iron_ingot", "C": "minecraft:copper_ingot"},
        "pattern": ["ICI", " I "],
        "result": {"id": "pikcraft:whistle"},
    })
    write("data/pikcraft/advancement/recipes/whistle.json", {
        "parent": "minecraft:recipes/root",
        "criteria": {
            "has_copper": {"trigger": "minecraft:inventory_changed", "conditions": {"items": [{"items": "minecraft:copper_ingot"}]}},
            "has_the_recipe": {"trigger": "minecraft:recipe_unlocked", "conditions": {"recipes": "pikcraft:whistle"}},
        },
        "requirements": [["has_copper", "has_the_recipe"]],
        "rewards": {"recipes": ["pikcraft:whistle"]},
    })
    write("assets/pikcraft/lang/en_us.json", dict(sorted(EN.items())))
    write("assets/pikcraft/lang/fr_fr.json", dict(sorted(FR.items())))
    missing = set(EN) ^ set(FR)
    assert not missing, missing


if __name__ == "__main__":
    main()
