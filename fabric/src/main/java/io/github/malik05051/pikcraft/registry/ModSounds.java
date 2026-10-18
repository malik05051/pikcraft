package io.github.malik05051.pikcraft.registry;

import io.github.malik05051.pikcraft.PikCraft;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.sounds.SoundEvent;

/** Sounds, all synthesized for the mod (see tools/make_assets.py). */
public final class ModSounds {
	public static final SoundEvent WHISTLE = register("whistle");
	public static final SoundEvent PLUCK = register("pikmin.pluck");
	public static final SoundEvent THROW = register("pikmin.throw");
	public static final SoundEvent HEY = register("pikmin.hey");
	public static final SoundEvent ATTACK = register("pikmin.attack");
	public static final SoundEvent DIE = register("pikmin.die");
	public static final SoundEvent CARRY = register("pikmin.carry");
	public static final SoundEvent DRINK = register("pikmin.drink");
	public static final SoundEvent ONION_AWAKEN = register("onion.awaken");
	public static final SoundEvent ONION_SEED = register("onion.seed");
	public static final SoundEvent ONION_SUCK = register("onion.suck");
	public static final SoundEvent ONION_FLY = register("onion.fly");
	public static final SoundEvent POSY_HIT = register("posy.hit");
	public static final SoundEvent PELLET_DROP = register("pellet.drop");
	public static final SoundEvent EGG_BREAK = register("egg.break");

	private static SoundEvent register(String name) {
		var id = PikCraft.id(name);
		return Registry.register(BuiltInRegistries.SOUND_EVENT, id, SoundEvent.createVariableRangeEvent(id));
	}

	public static void init() {
	}

	private ModSounds() {
	}
}
