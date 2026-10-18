package io.github.malik05051.pikcraft.entity;

import net.minecraft.network.syncher.EntityDataAccessor;
import net.minecraft.network.syncher.EntityDataSerializers;
import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.MoverType;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;

/** A sticky puddle of nectar from a broken egg. Pikmin drink it and bloom into flowers. */
public class NectarEntity extends Entity {
	private static final EntityDataAccessor<Integer> DATA_SIPS = SynchedEntityData.defineId(NectarEntity.class, EntityDataSerializers.INT);
	public static final int MAX_SIPS = 5;
	private static final int LIFETIME = 6000;

	public NectarEntity(EntityType<? extends NectarEntity> type, Level level) {
		super(type, level);
	}

	@Override
	protected void defineSynchedData(SynchedEntityData.Builder builder) {
		builder.define(DATA_SIPS, MAX_SIPS);
	}

	public int sips() {
		return this.entityData.get(DATA_SIPS);
	}

	/** One Pikmin drinks; returns false if it was already empty. */
	public boolean sip() {
		int sips = this.sips();
		if (sips <= 0) {
			return false;
		}
		this.entityData.set(DATA_SIPS, sips - 1);
		if (sips - 1 <= 0) {
			this.discard();
		}
		return true;
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
		this.setDeltaMovement(this.getDeltaMovement().multiply(0.6, 0.98, 0.6));
		if (!this.level().isClientSide() && this.tickCount > LIFETIME) {
			this.discard();
		}
	}

	@Override
	public boolean hurtServer(ServerLevel level, DamageSource source, float damage) {
		return false;
	}

	@Override
	public boolean isPickable() {
		return false;
	}

	@Override
	protected void readAdditionalSaveData(ValueInput input) {
		this.entityData.set(DATA_SIPS, input.getIntOr("Sips", MAX_SIPS));
	}

	@Override
	protected void addAdditionalSaveData(ValueOutput output) {
		output.putInt("Sips", this.sips());
	}
}
