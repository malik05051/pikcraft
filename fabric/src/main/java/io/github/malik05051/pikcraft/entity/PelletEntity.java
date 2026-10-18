package io.github.malik05051.pikcraft.entity;

import io.github.malik05051.pikcraft.PikminType;
import net.minecraft.network.syncher.EntityDataAccessor;
import net.minecraft.network.syncher.EntityDataSerializers;
import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.world.entity.EntityDimensions;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.Pose;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;

/** A pellet from a Pellet Posy: 1, 5, 10 or 20, in a Pikmin colour. */
public class PelletEntity extends CarryableEntity {
	private static final EntityDataAccessor<Integer> DATA_VALUE = SynchedEntityData.defineId(PelletEntity.class, EntityDataSerializers.INT);
	private static final EntityDataAccessor<Integer> DATA_COLOUR = SynchedEntityData.defineId(PelletEntity.class, EntityDataSerializers.INT);

	public PelletEntity(EntityType<? extends PelletEntity> type, Level level) {
		super(type, level);
	}

	@Override
	protected void defineSynchedData(SynchedEntityData.Builder builder) {
		super.defineSynchedData(builder);
		builder.define(DATA_VALUE, 1);
		builder.define(DATA_COLOUR, PikminType.RED.ordinal());
	}

	public int value() {
		return this.entityData.get(DATA_VALUE);
	}

	public PikminType colour() {
		return PikminType.byId(this.entityData.get(DATA_COLOUR));
	}

	public void setPellet(int value, PikminType colour) {
		this.entityData.set(DATA_VALUE, value);
		this.entityData.set(DATA_COLOUR, colour.ordinal());
		this.refreshDimensions();
	}

	@Override
	public void onSyncedDataUpdated(EntityDataAccessor<?> accessor) {
		super.onSyncedDataUpdated(accessor);
		if (DATA_VALUE.equals(accessor)) {
			this.refreshDimensions();
		}
	}

	/** Pellets grow with their number. */
	public float size() {
		return switch (this.value()) {
			case 5 -> 0.75F;
			case 10 -> 1.0F;
			case 20 -> 1.3F;
			default -> 0.5F;
		};
	}

	@Override
	public EntityDimensions getDefaultDimensions(Pose pose) {
		return EntityDimensions.scalable(this.size(), this.size() * 0.35F);
	}

	@Override
	public int weight() {
		return this.value();
	}

	@Override
	public int seedsFor(PikminType onionType) {
		boolean matching = onionType == this.colour();
		return switch (this.value()) {
			case 5 -> matching ? 5 : 3;
			case 10 -> matching ? 10 : 5;
			case 20 -> matching ? 20 : 10;
			default -> matching ? 2 : 1;
		};
	}

	@Override
	protected void addAdditionalSaveData(ValueOutput output) {
		super.addAdditionalSaveData(output);
		output.putInt("Value", this.value());
		output.putInt("Colour", this.colour().ordinal());
	}

	@Override
	protected void readAdditionalSaveData(ValueInput input) {
		super.readAdditionalSaveData(input);
		this.setPellet(input.getIntOr("Value", 1), PikminType.byId(input.getIntOr("Colour", 0)));
	}
}
