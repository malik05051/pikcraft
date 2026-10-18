package io.github.malik05051.pikcraft.registry;

import java.util.EnumMap;
import java.util.HashMap;
import java.util.Map;

import io.github.malik05051.pikcraft.PikCraft;
import io.github.malik05051.pikcraft.PikminType;
import io.github.malik05051.pikcraft.entity.CorpseEntity;
import io.github.malik05051.pikcraft.entity.CarryableEntity;
import io.github.malik05051.pikcraft.entity.NectarEggEntity;
import io.github.malik05051.pikcraft.entity.NectarEntity;
import io.github.malik05051.pikcraft.entity.OnionEntity;
import io.github.malik05051.pikcraft.entity.PelletEntity;
import io.github.malik05051.pikcraft.entity.PelletPosyEntity;
import io.github.malik05051.pikcraft.entity.PikminEntity;
import io.github.malik05051.pikcraft.entity.SeedEntity;
import net.fabricmc.fabric.api.object.builder.v1.entity.FabricDefaultAttributeRegistry;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.MobCategory;

public final class ModEntities {
	private static final Map<PikminType, EntityType<PikminEntity>> PIKMIN = new EnumMap<>(PikminType.class);
	private static final Map<EntityType<?>, PikminType> PIKMIN_TYPES = new HashMap<>();

	static {
		for (PikminType type : PikminType.values()) {
			float scale = type == PikminType.PURPLE ? 1.3F : type == PikminType.WHITE ? 0.9F : 1.0F;
			EntityType.Builder<PikminEntity> builder = EntityType.Builder.<PikminEntity>of(PikminEntity::new, MobCategory.MISC)
				.sized(0.45F * scale, 0.9F * scale)
				.eyeHeight(0.7F * scale)
				.clientTrackingRange(10)
				.updateInterval(2);
			if (type == PikminType.RED) {
				builder.fireImmune();
			}
			EntityType<PikminEntity> entityType = register(type.id() + "_pikmin", builder);
			PIKMIN.put(type, entityType);
			PIKMIN_TYPES.put(entityType, type);
		}
	}

	public static final EntityType<OnionEntity> ONION = register("onion",
		EntityType.Builder.of(OnionEntity::new, MobCategory.MISC).sized(2.4F, 3.0F).eyeHeight(2.2F).clientTrackingRange(12).fireImmune());
	public static final EntityType<SeedEntity> SEED = register("seed",
		EntityType.Builder.<SeedEntity>of(SeedEntity::new, MobCategory.MISC).sized(0.25F, 0.25F).clientTrackingRange(8).updateInterval(2).noLootTable());
	public static final EntityType<PelletEntity> PELLET = register("pellet",
		EntityType.Builder.of(PelletEntity::new, MobCategory.MISC).sized(0.5F, 0.2F).clientTrackingRange(10).fireImmune());
	public static final EntityType<CorpseEntity> CORPSE = register("corpse",
		EntityType.Builder.of(CorpseEntity::new, MobCategory.MISC).sized(1.0F, 0.5F).clientTrackingRange(10).fireImmune());
	public static final EntityType<PelletPosyEntity> PELLET_POSY = register("pellet_posy",
		EntityType.Builder.of(PelletPosyEntity::new, MobCategory.MISC).sized(0.8F, 1.6F).clientTrackingRange(10));
	public static final EntityType<NectarEggEntity> NECTAR_EGG = register("nectar_egg",
		EntityType.Builder.of(NectarEggEntity::new, MobCategory.MISC).sized(0.7F, 0.9F).clientTrackingRange(10));
	public static final EntityType<NectarEntity> NECTAR = register("nectar",
		EntityType.Builder.<NectarEntity>of(NectarEntity::new, MobCategory.MISC).sized(0.8F, 0.1F).clientTrackingRange(8).noLootTable());

	private static <T extends net.minecraft.world.entity.Entity> EntityType<T> register(String name, EntityType.Builder<T> builder) {
		ResourceKey<EntityType<?>> key = ResourceKey.create(Registries.ENTITY_TYPE, PikCraft.id(name));
		return Registry.register(BuiltInRegistries.ENTITY_TYPE, key, builder.build(key));
	}

	public static EntityType<PikminEntity> pikmin(PikminType type) {
		return PIKMIN.get(type);
	}

	public static PikminType pikminTypeOf(EntityType<?> entityType) {
		return PIKMIN_TYPES.getOrDefault(entityType, PikminType.RED);
	}

	public static void init() {
		for (PikminType type : PikminType.values()) {
			FabricDefaultAttributeRegistry.register(PIKMIN.get(type), PikminEntity.createAttributes(type));
		}
		FabricDefaultAttributeRegistry.register(ONION, OnionEntity.createAttributes());
		FabricDefaultAttributeRegistry.register(PELLET, CarryableEntity.createAttributes());
		FabricDefaultAttributeRegistry.register(CORPSE, CarryableEntity.createAttributes());
		FabricDefaultAttributeRegistry.register(PELLET_POSY, PelletPosyEntity.createAttributes());
		FabricDefaultAttributeRegistry.register(NECTAR_EGG, NectarEggEntity.createAttributes());
	}

	private ModEntities() {
	}
}
