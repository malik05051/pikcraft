package io.github.malik05051.pikcraft;

import java.util.Locale;

import net.minecraft.util.RandomSource;

/**
 * The seven Pikmin types (Pikmin 1 to 3): their colours, strengths and how often their Onion is found.
 */
public enum PikminType {
	//    colour      carry  attack  speed  health  throw  onion weight
	RED(0xE8302A, 1, 1.5F, 1.00F, 5.0F, 1.00F, 40),
	YELLOW(0xF2D72C, 1, 1.0F, 1.00F, 5.0F, 1.35F, 17),
	BLUE(0x2F6FE8, 1, 1.0F, 1.00F, 5.0F, 1.00F, 17),
	ROCK(0x6E6A7A, 1, 2.0F, 0.95F, 8.0F, 0.95F, 8),
	WINGED(0xF08CC8, 1, 0.7F, 1.10F, 4.0F, 1.10F, 8),
	PURPLE(0x7A3FB8, 10, 2.5F, 0.80F, 9.0F, 0.80F, 5),
	WHITE(0xF2F2F2, 1, 0.8F, 1.25F, 4.0F, 1.05F, 5);

	public final int color;
	/** How much weight one Pikmin of this type lifts. */
	public final int strength;
	public final float attack;
	public final float speed;
	public final float health;
	public final float throwPower;
	/** Relative chance of an Onion of this colour (red common, purple and white very rare). */
	public final int onionWeight;

	PikminType(int color, int strength, float attack, float speed, float health, float throwPower, int onionWeight) {
		this.color = color;
		this.strength = strength;
		this.attack = attack;
		this.speed = speed;
		this.health = health;
		this.throwPower = throwPower;
		this.onionWeight = onionWeight;
	}

	public String id() {
		return name().toLowerCase(Locale.ROOT);
	}

	/** Rock Pikmin have no leaf, so they never grow buds or flowers. */
	public boolean grows() {
		return this != ROCK;
	}

	public boolean flies() {
		return this == WINGED;
	}

	public static PikminType byId(int ordinal) {
		PikminType[] values = values();
		return values[Math.floorMod(ordinal, values.length)];
	}

	public static PikminType randomOnionType(RandomSource random) {
		int total = 0;
		for (PikminType type : values()) {
			total += type.onionWeight;
		}
		int roll = random.nextInt(total);
		for (PikminType type : values()) {
			roll -= type.onionWeight;
			if (roll < 0) {
				return type;
			}
		}
		return RED;
	}
}
