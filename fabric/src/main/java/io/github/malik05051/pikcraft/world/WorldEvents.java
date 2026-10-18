package io.github.malik05051.pikcraft.world;

import java.util.HashMap;
import java.util.Iterator;
import java.util.Map;
import java.util.UUID;

import io.github.malik05051.pikcraft.PikminType;
import io.github.malik05051.pikcraft.entity.CarryableEntity;
import io.github.malik05051.pikcraft.entity.CorpseEntity;
import io.github.malik05051.pikcraft.entity.NectarEggEntity;
import io.github.malik05051.pikcraft.entity.OnionEntity;
import io.github.malik05051.pikcraft.entity.PelletPosyEntity;
import io.github.malik05051.pikcraft.entity.PikminEntity;
import io.github.malik05051.pikcraft.network.OnionCutscenePayload;
import io.github.malik05051.pikcraft.registry.ModAttachments;
import io.github.malik05051.pikcraft.registry.ModEntities;
import io.github.malik05051.pikcraft.squad.Squads;
import net.fabricmc.fabric.api.entity.event.v1.ServerLivingEntityEvents;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerEntityEvents;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import net.fabricmc.fabric.api.networking.v1.ServerPlayConnectionEvents;
import net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking;
import net.minecraft.core.BlockPos;
import net.minecraft.core.particles.ParticleTypes;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.tags.BlockTags;
import net.minecraft.util.Mth;
import net.minecraft.util.RandomSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.Mob;
import net.minecraft.world.entity.ai.goal.target.NearestAttackableTargetGoal;
import net.minecraft.world.entity.boss.enderdragon.EnderDragon;
import net.minecraft.world.entity.boss.wither.WitherBoss;
import net.minecraft.world.entity.monster.Creeper;
import net.minecraft.world.entity.monster.spider.Spider;
import net.minecraft.world.entity.monster.skeleton.AbstractSkeleton;
import net.minecraft.world.entity.monster.zombie.Zombie;
import net.minecraft.world.entity.monster.zombie.ZombifiedPiglin;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.AABB;
import org.jspecify.annotations.Nullable;

/**
 * Everything that happens in the world on its own: the first Onion, Onions, posies and nectar eggs
 * appearing around players, enemies defeated by Pikmin turning into bodies to carry, and monsters
 * hunting Pikmin.
 */
public final class WorldEvents {
	/** Ticks after joining before the first Onion appears, so the world has finished loading. */
	private static final int START_DELAY = 60;
	private static final Map<UUID, Integer> PENDING_START = new HashMap<>();

	public static void init() {
		ServerPlayConnectionEvents.JOIN.register((handler, sender, server) -> {
			if (!handler.player.getAttachedOrElse(ModAttachments.START_ONION, false)) {
				PENDING_START.put(handler.player.getUUID(), START_DELAY);
			}
		});
		ServerTickEvents.END_SERVER_TICK.register(WorldEvents::tick);
		ServerLivingEntityEvents.ALLOW_DEATH.register(WorldEvents::allowDeath);
		ServerEntityEvents.ENTITY_LOAD.register(WorldEvents::onLoad);
	}

	private static void tick(MinecraftServer server) {
		for (Iterator<Map.Entry<UUID, Integer>> it = PENDING_START.entrySet().iterator(); it.hasNext(); ) {
			Map.Entry<UUID, Integer> entry = it.next();
			ServerPlayer player = server.getPlayerList().getPlayer(entry.getKey());
			if (player == null) {
				it.remove();
			} else if (entry.getValue() <= 0) {
				it.remove();
				placeStartOnion(player);
			} else {
				entry.setValue(entry.getValue() - 1);
			}
		}
		int tick = server.getTickCount();
		for (ServerPlayer player : server.getPlayerList().getPlayers()) {
			if (tick % 10 == player.getId() % 10) {
				Squads.syncHud(player);
			}
			if (tick % 200 == player.getId() % 200 && !player.isSpectator()) {
				populate(player);
			}
		}
	}

	// ------------------------------------------------------------------ the first Onion

	/** A sleeping Onion 5 to 15 blocks from a player who has never had one, with the discovery scene. */
	public static @Nullable OnionEntity placeStartOnion(ServerPlayer player) {
		ServerLevel level = player.level();
		if (level.dimension() != Level.OVERWORLD) {
			return null; // try again next time they join
		}
		RandomSource random = player.getRandom();
		for (int attempt = 0; attempt < 24; attempt++) {
			double angle = random.nextDouble() * Mth.TWO_PI;
			double distance = 5.0 + random.nextDouble() * 10.0;
			BlockPos pos = surface(level, Mth.floor(player.getX() + Math.cos(angle) * distance), Mth.floor(player.getZ() + Math.sin(angle) * distance));
			if (pos == null || Math.abs(pos.getY() - player.getY()) > 8) {
				continue;
			}
			OnionEntity onion = spawnOnion(level, pos, PikminType.randomOnionType(random));
			if (onion != null) {
				player.setAttached(ModAttachments.START_ONION, true);
				ServerPlayNetworking.send(player, new OnionCutscenePayload(onion.getId(), OnionCutscenePayload.DISCOVER));
				return onion;
			}
		}
		return null;
	}

	public static @Nullable OnionEntity spawnOnion(ServerLevel level, BlockPos pos, PikminType type) {
		OnionEntity onion = ModEntities.ONION.create(level, EntitySpawnReason.EVENT);
		if (onion == null) {
			return null;
		}
		onion.setOnionType(type);
		onion.snapTo(pos.getX() + 0.5, pos.getY(), pos.getZ() + 0.5, level.getRandom().nextFloat() * 360.0F, 0.0F);
		if (!level.noCollision(onion)) {
			return null;
		}
		level.addFreshEntity(onion);
		return onion;
	}

	/** The first free block above the ground (no leaves, no water), or null. */
	private static @Nullable BlockPos surface(ServerLevel level, int x, int z) {
		BlockPos top = level.getHeightmapPos(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES, new BlockPos(x, 0, z));
		if (!level.getFluidState(top.below()).isEmpty() || !level.getFluidState(top).isEmpty()) {
			return null;
		}
		return level.getBlockState(top.below()).isSolid() ? top : null;
	}

	// ------------------------------------------------------------------ the world fills up

	/** Now and then: pellet posies and nectar eggs on the surface, and very rarely another Onion. */
	private static void populate(ServerPlayer player) {
		ServerLevel level = player.level();
		if (level.dimension() != Level.OVERWORLD) {
			return;
		}
		RandomSource random = player.getRandom();
		AABB around = player.getBoundingBox().inflate(48.0);
		if (level.getEntitiesOfClass(PelletPosyEntity.class, around, Entity::isAlive).size() < 3 && random.nextInt(3) == 0) {
			BlockPos pos = surfaceNear(level, player, random);
			if (pos != null && level.getBlockState(pos.below()).is(BlockTags.DIRT)) {
				spawn(level, ModEntities.PELLET_POSY.create(level, EntitySpawnReason.NATURAL), pos);
			}
		}
		if (level.getEntitiesOfClass(NectarEggEntity.class, around, Entity::isAlive).size() < 2 && random.nextInt(4) == 0) {
			BlockPos pos = surfaceNear(level, player, random);
			if (pos != null) {
				spawn(level, ModEntities.NECTAR_EGG.create(level, EntitySpawnReason.NATURAL), pos);
			}
		}
		if (!level.getEntitiesOfClass(OnionEntity.class, player.getBoundingBox().inflate(160.0), Entity::isAlive).isEmpty()) {
			return; // one Onion of any kind around is enough
		}
		if (player.getY() < level.getSeaLevel() - 8 && random.nextInt(6) == 0) {
			BlockPos cave = bigCaveNear(level, player, random);
			if (cave != null) {
				spawnOnion(level, cave, PikminType.randomOnionType(random));
			}
		} else if (random.nextInt(150) == 0) {
			BlockPos pos = surfaceNear(level, player, random);
			if (pos != null) {
				spawnOnion(level, pos, PikminType.randomOnionType(random));
			}
		}
	}

	private static void spawn(ServerLevel level, @Nullable Mob mob, BlockPos pos) {
		if (mob == null) {
			return;
		}
		mob.snapTo(pos.getX() + 0.5, pos.getY(), pos.getZ() + 0.5, level.getRandom().nextFloat() * 360.0F, 0.0F);
		if (level.noCollision(mob)) {
			level.addFreshEntity(mob);
		}
	}

	/** A surface spot 24 to 48 blocks away, out of the player's sight line so things don't pop in. */
	private static @Nullable BlockPos surfaceNear(ServerLevel level, ServerPlayer player, RandomSource random) {
		double angle = random.nextDouble() * Mth.TWO_PI;
		double distance = 24.0 + random.nextDouble() * 24.0;
		int x = Mth.floor(player.getX() + Math.cos(angle) * distance);
		int z = Mth.floor(player.getZ() + Math.sin(angle) * distance);
		if (!level.hasChunkAt(new BlockPos(x, 0, z))) {
			return null;
		}
		return surface(level, x, z);
	}

	/** The floor of a very large cave room near the player: lots of open air all around and above. */
	private static @Nullable BlockPos bigCaveNear(ServerLevel level, ServerPlayer player, RandomSource random) {
		for (int attempt = 0; attempt < 8; attempt++) {
			BlockPos pos = player.blockPosition().offset(random.nextInt(49) - 24, random.nextInt(17) - 8, random.nextInt(49) - 24);
			if (!level.hasChunkAt(pos) || level.canSeeSky(pos)) {
				continue;
			}
			// drop to the floor
			int floor = 0;
			while (floor < 16 && level.getBlockState(pos.below()).isAir()) {
				pos = pos.below();
				floor++;
			}
			if (!level.getBlockState(pos.below()).isSolid() || !level.getBlockState(pos).isAir()) {
				continue;
			}
			int air = 0;
			for (BlockPos p : BlockPos.betweenClosed(pos.offset(-6, 0, -6), pos.offset(6, 6, 6))) {
				if (level.getBlockState(p).isAir()) {
					air++;
				}
			}
			// 13 x 7 x 13 = 1183 blocks: a big room is mostly air
			if (air > 900) {
				return pos.immutable();
			}
		}
		return null;
	}

	// ------------------------------------------------------------------ fights

	/** Mobs defeated by Pikmin don't drop anything: they leave a body for the Pikmin to carry home. */
	private static boolean allowDeath(LivingEntity entity, net.minecraft.world.damagesource.DamageSource source, float amount) {
		if (!(source.getEntity() instanceof PikminEntity) || !(entity instanceof Mob) || !(entity.level() instanceof ServerLevel level)) {
			return true;
		}
		if (entity instanceof PikminEntity || entity instanceof OnionEntity || entity instanceof CarryableEntity || entity instanceof PelletPosyEntity
			|| entity instanceof NectarEggEntity || entity instanceof EnderDragon || entity instanceof WitherBoss) {
			return true;
		}
		CorpseEntity corpse = ModEntities.CORPSE.create(level, EntitySpawnReason.TRIGGERED);
		if (corpse == null) {
			return true;
		}
		corpse.copyFrom(entity);
		corpse.snapTo(entity.getX(), entity.getY(), entity.getZ(), entity.getYRot(), 0.0F);
		level.addFreshEntity(corpse);
		level.sendParticles(ParticleTypes.POOF, entity.getX(), entity.getY() + entity.getBbHeight() * 0.5, entity.getZ(), 10, 0.3, 0.3, 0.3, 0.02);
		entity.discard();
		return false;
	}

	/** Monsters that hunt players hunt Pikmin too. */
	private static void onLoad(Entity entity, ServerLevel level) {
		if (entity instanceof Zombie && !(entity instanceof ZombifiedPiglin) || entity instanceof AbstractSkeleton || entity instanceof Spider
			|| entity instanceof Creeper) {
			Mob mob = (Mob) entity;
			mob.targetSelector.addGoal(3, new NearestAttackableTargetGoal<>(mob, PikminEntity.class, true,
				(target, l) -> target instanceof PikminEntity pikmin && pikmin.state() != PikminEntity.State.BURIED));
		}
	}

	private WorldEvents() {
	}
}
