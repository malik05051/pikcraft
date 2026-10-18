package io.github.malik05051.pikcraft.entity;

import io.github.malik05051.pikcraft.PikminType;
import io.github.malik05051.pikcraft.registry.ModEntities;
import net.minecraft.core.particles.ParticleTypes;
import net.minecraft.network.syncher.EntityDataAccessor;
import net.minecraft.network.syncher.EntityDataSerializers;
import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.MoverType;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;

/** A seed shot out of an Onion. Where it lands, a Pikmin sprout grows. */
public class SeedEntity extends Entity {
	private static final EntityDataAccessor<Integer> DATA_TYPE = SynchedEntityData.defineId(SeedEntity.class, EntityDataSerializers.INT);

	public SeedEntity(EntityType<? extends SeedEntity> type, Level level) {
		super(type, level);
	}

	@Override
	protected void defineSynchedData(SynchedEntityData.Builder builder) {
		builder.define(DATA_TYPE, PikminType.RED.ordinal());
	}

	public PikminType seedType() {
		return PikminType.byId(this.entityData.get(DATA_TYPE));
	}

	public void setSeedType(PikminType type) {
		this.entityData.set(DATA_TYPE, type.ordinal());
	}

	@Override
	protected double getDefaultGravity() {
		return 0.04;
	}

	@Override
	public void tick() {
		super.tick();
		this.applyGravity();
		this.move(MoverType.SELF, this.getDeltaMovement());
		this.setDeltaMovement(this.getDeltaMovement().scale(0.98));
		if (this.level() instanceof ServerLevel level) {
			if (this.onGround() || this.isInWater() || this.tickCount > 200) {
				this.plant(level);
			}
		}
	}

	private void plant(ServerLevel level) {
		PikminEntity sprout = ModEntities.pikmin(this.seedType()).create(level, EntitySpawnReason.TRIGGERED);
		if (sprout != null) {
			sprout.snapTo(this.getX(), this.getY() - 0.15, this.getZ(), this.random.nextFloat() * 360.0F, 0.0F);
			sprout.bury();
			level.addFreshEntity(sprout);
		}
		level.sendParticles(ParticleTypes.COMPOSTER, this.getX(), this.getY() + 0.1, this.getZ(), 6, 0.15, 0.05, 0.15, 0.0);
		this.discard();
	}

	@Override
	public boolean hurtServer(ServerLevel level, DamageSource source, float damage) {
		return false;
	}

	@Override
	protected void readAdditionalSaveData(ValueInput input) {
		this.setSeedType(PikminType.byId(input.getIntOr("SeedType", 0)));
	}

	@Override
	protected void addAdditionalSaveData(ValueOutput output) {
		output.putInt("SeedType", this.seedType().ordinal());
	}
}
