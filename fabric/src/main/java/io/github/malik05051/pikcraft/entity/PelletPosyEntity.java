package io.github.malik05051.pikcraft.entity;

import io.github.malik05051.pikcraft.PikminType;
import io.github.malik05051.pikcraft.registry.ModEntities;
import io.github.malik05051.pikcraft.registry.ModSounds;
import net.minecraft.core.particles.ParticleTypes;
import net.minecraft.network.syncher.EntityDataAccessor;
import net.minecraft.network.syncher.EntityDataSerializers;
import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.damagesource.DamageTypes;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.Mob;
import net.minecraft.world.entity.PathfinderMob;
import net.minecraft.world.entity.ai.attributes.AttributeSupplier;
import net.minecraft.world.entity.ai.attributes.Attributes;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;
import net.minecraft.world.phys.Vec3;

/**
 * A Pellet Posy: a flower holding a pellet in its middle. Hitting the middle squarely knocks the
 * pellet off at once; otherwise it takes a few hits. The flower grows a new pellet after a while.
 */
public class PelletPosyEntity extends PathfinderMob {
	private static final EntityDataAccessor<Integer> DATA_VALUE = SynchedEntityData.defineId(PelletPosyEntity.class, EntityDataSerializers.INT);
	private static final EntityDataAccessor<Integer> DATA_COLOUR = SynchedEntityData.defineId(PelletPosyEntity.class, EntityDataSerializers.INT);
	private static final EntityDataAccessor<Integer> DATA_HIT_TICK = SynchedEntityData.defineId(PelletPosyEntity.class, EntityDataSerializers.INT);
	private static final int REGROW_TICKS = 12000;
	private static final float TOUGHNESS = 8.0F;

	private float damageTaken;
	private int regrow;

	public PelletPosyEntity(EntityType<? extends PelletPosyEntity> type, Level level) {
		super(type, level);
	}

	public static AttributeSupplier.Builder createAttributes() {
		return Mob.createMobAttributes().add(Attributes.MAX_HEALTH, 20.0).add(Attributes.MOVEMENT_SPEED, 0.0).add(Attributes.KNOCKBACK_RESISTANCE, 1.0);
	}

	@Override
	protected void defineSynchedData(SynchedEntityData.Builder builder) {
		super.defineSynchedData(builder);
		builder.define(DATA_VALUE, 1);
		builder.define(DATA_COLOUR, PikminType.RED.ordinal());
		builder.define(DATA_HIT_TICK, -100);
	}

	/** Pellet number on the flower, or 0 while it grows a new one. */
	public int pelletValue() {
		return this.entityData.get(DATA_VALUE);
	}

	public PikminType pelletColour() {
		return PikminType.byId(this.entityData.get(DATA_COLOUR));
	}

	public int hitTick() {
		return this.entityData.get(DATA_HIT_TICK);
	}

	public void growPellet() {
		int roll = this.random.nextInt(100);
		int value = roll < 60 ? 1 : roll < 90 ? 5 : roll < 98 ? 10 : 20;
		PikminType[] colours = {PikminType.RED, PikminType.YELLOW, PikminType.BLUE, PikminType.RED, PikminType.YELLOW, PikminType.BLUE,
			PikminType.ROCK, PikminType.WINGED, PikminType.PURPLE, PikminType.WHITE};
		this.entityData.set(DATA_VALUE, value);
		this.entityData.set(DATA_COLOUR, colours[this.random.nextInt(colours.length)].ordinal());
		this.damageTaken = 0.0F;
	}

	@Override
	public boolean hurtServer(ServerLevel level, DamageSource source, float damage) {
		if (source.is(DamageTypes.GENERIC_KILL) || source.is(DamageTypes.FELL_OUT_OF_WORLD)) {
			return super.hurtServer(level, source, damage);
		}
		if (this.pelletValue() <= 0) {
			return false;
		}
		this.entityData.set(DATA_HIT_TICK, this.tickCount);
		this.playSound(ModSounds.POSY_HIT, 0.8F, 0.9F + this.random.nextFloat() * 0.2F);
		this.damageTaken += damage;
		boolean squareHit = source.getEntity() instanceof Player player && this.aimedAtMiddle(player);
		if (squareHit || this.damageTaken >= TOUGHNESS * this.pelletValue() / 2.0F + 2.0F) {
			this.dropPellet(level);
		}
		return true;
	}

	/** Did the player's swing point at the pellet in the middle of the flower? */
	private boolean aimedAtMiddle(Player player) {
		Vec3 eye = player.getEyePosition();
		Vec3 look = player.getLookAngle();
		Vec3 middle = new Vec3(this.getX(), this.getY() + this.getBbHeight() * 0.82, this.getZ());
		Vec3 toMiddle = middle.subtract(eye);
		double along = toMiddle.dot(look);
		if (along <= 0.0) {
			return false;
		}
		Vec3 closest = eye.add(look.scale(along));
		return closest.distanceTo(middle) < 0.22;
	}

	private void dropPellet(ServerLevel level) {
		PelletEntity pellet = ModEntities.PELLET.create(level, EntitySpawnReason.TRIGGERED);
		if (pellet != null) {
			pellet.setPellet(this.pelletValue(), this.pelletColour());
			pellet.snapTo(this.getX(), this.getY() + this.getBbHeight() * 0.8, this.getZ(), this.random.nextFloat() * 360.0F, 0.0F);
			pellet.setDeltaMovement((this.random.nextDouble() - 0.5) * 0.2, 0.25, (this.random.nextDouble() - 0.5) * 0.2);
			level.addFreshEntity(pellet);
		}
		this.playSound(ModSounds.PELLET_DROP, 1.0F, 1.0F);
		level.sendParticles(ParticleTypes.CHERRY_LEAVES, this.getX(), this.getY() + this.getBbHeight() * 0.8, this.getZ(), 10, 0.3, 0.2, 0.3, 0.0);
		this.entityData.set(DATA_VALUE, 0);
		this.regrow = REGROW_TICKS;
	}

	@Override
	protected void customServerAiStep(ServerLevel level) {
		super.customServerAiStep(level);
		if (this.pelletValue() <= 0 && --this.regrow <= 0) {
			this.growPellet();
		}
	}

	@Override
	public boolean isPushable() {
		return false;
	}

	@Override
	public boolean removeWhenFarAway(double distanceToClosestPlayer) {
		return false;
	}

	@Override
	public boolean causeFallDamage(double fallDistance, float damageModifier, DamageSource damageSource) {
		return false;
	}

	@Override
	protected void addAdditionalSaveData(ValueOutput output) {
		super.addAdditionalSaveData(output);
		output.putInt("PelletValue", this.pelletValue());
		output.putInt("PelletColour", this.pelletColour().ordinal());
		output.putInt("Regrow", this.regrow);
	}

	@Override
	protected void readAdditionalSaveData(ValueInput input) {
		super.readAdditionalSaveData(input);
		this.entityData.set(DATA_VALUE, input.getIntOr("PelletValue", 1));
		this.entityData.set(DATA_COLOUR, input.getIntOr("PelletColour", 0));
		this.regrow = input.getIntOr("Regrow", 0);
	}
}
