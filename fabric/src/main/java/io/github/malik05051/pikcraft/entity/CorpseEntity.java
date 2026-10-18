package io.github.malik05051.pikcraft.entity;

import java.util.Optional;

import io.github.malik05051.pikcraft.PikminType;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.network.syncher.EntityDataAccessor;
import net.minecraft.network.syncher.EntityDataSerializers;
import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.resources.Identifier;
import net.minecraft.util.Mth;
import net.minecraft.world.entity.EntityDimensions;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.Pose;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;

/**
 * The limp body of a creature the Pikmin defeated. It is drawn with the creature's own model, lying on
 * its side, and the Pikmin carry it home to make seeds.
 */
public class CorpseEntity extends CarryableEntity {
	private static final EntityDataAccessor<String> DATA_BODY = SynchedEntityData.defineId(CorpseEntity.class, EntityDataSerializers.STRING);
	private static final EntityDataAccessor<Integer> DATA_WEIGHT = SynchedEntityData.defineId(CorpseEntity.class, EntityDataSerializers.INT);
	private static final EntityDataAccessor<Float> DATA_BODY_YAW = SynchedEntityData.defineId(CorpseEntity.class, EntityDataSerializers.FLOAT);

	public CorpseEntity(EntityType<? extends CorpseEntity> type, Level level) {
		super(type, level);
	}

	@Override
	protected void defineSynchedData(SynchedEntityData.Builder builder) {
		super.defineSynchedData(builder);
		builder.define(DATA_BODY, "minecraft:zombie");
		builder.define(DATA_WEIGHT, 2);
		builder.define(DATA_BODY_YAW, 0.0F);
	}

	public void copyFrom(LivingEntity dead) {
		this.entityData.set(DATA_BODY, BuiltInRegistries.ENTITY_TYPE.getKey(dead.getType()).toString());
		this.entityData.set(DATA_WEIGHT, Mth.clamp(Mth.ceil(dead.getMaxHealth() / 5.0F), 1, 40));
		this.entityData.set(DATA_BODY_YAW, dead.getYRot());
		this.setYRot(dead.getYRot());
		this.refreshDimensions();
	}

	public Optional<EntityType<?>> bodyType() {
		Identifier id = Identifier.tryParse(this.entityData.get(DATA_BODY));
		return id == null ? Optional.empty() : BuiltInRegistries.ENTITY_TYPE.getOptional(id);
	}

	public float bodyYaw() {
		return this.entityData.get(DATA_BODY_YAW);
	}

	@Override
	public void onSyncedDataUpdated(EntityDataAccessor<?> accessor) {
		super.onSyncedDataUpdated(accessor);
		if (DATA_BODY.equals(accessor)) {
			this.refreshDimensions();
		}
	}

	/** Lying down: as long as the creature was tall, and flat. */
	@Override
	public EntityDimensions getDefaultDimensions(Pose pose) {
		return this.bodyType().map(type -> {
			EntityDimensions standing = type.getDimensions();
			float length = Math.max(standing.width(), standing.height());
			return EntityDimensions.scalable(Mth.clamp(length, 0.4F, 4.0F), Mth.clamp(standing.width(), 0.25F, 2.0F));
		}).orElse(EntityDimensions.scalable(1.0F, 0.5F));
	}

	@Override
	public int weight() {
		return this.entityData.get(DATA_WEIGHT);
	}

	@Override
	public int seedsFor(PikminType onionType) {
		return this.weight() * 2;
	}

	@Override
	protected void addAdditionalSaveData(ValueOutput output) {
		super.addAdditionalSaveData(output);
		output.putString("Body", this.entityData.get(DATA_BODY));
		output.putInt("Weight", this.weight());
		output.putFloat("BodyYaw", this.bodyYaw());
	}

	@Override
	protected void readAdditionalSaveData(ValueInput input) {
		super.readAdditionalSaveData(input);
		this.entityData.set(DATA_BODY, input.getStringOr("Body", "minecraft:zombie"));
		this.entityData.set(DATA_WEIGHT, input.getIntOr("Weight", 2));
		this.entityData.set(DATA_BODY_YAW, input.getFloatOr("BodyYaw", 0.0F));
		this.refreshDimensions();
	}
}
