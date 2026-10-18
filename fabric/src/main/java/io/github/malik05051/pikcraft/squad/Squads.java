package io.github.malik05051.pikcraft.squad;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.EnumMap;
import java.util.List;
import java.util.Map;

import io.github.malik05051.pikcraft.PikminType;
import io.github.malik05051.pikcraft.entity.PikminEntity;
import io.github.malik05051.pikcraft.network.EnemyHealthPayload;
import io.github.malik05051.pikcraft.network.HudSyncPayload;
import io.github.malik05051.pikcraft.registry.ModAttachments;
import io.github.malik05051.pikcraft.registry.ModCriteria;
import io.github.malik05051.pikcraft.registry.ModItems;
import io.github.malik05051.pikcraft.registry.ModSounds;
import net.fabricmc.fabric.api.networking.v1.PlayerLookup;
import net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking;
import net.minecraft.core.particles.ParticleTypes;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.sounds.SoundSource;
import net.minecraft.util.Mth;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.entity.projectile.ProjectileUtil;
import net.minecraft.world.level.ClipContext;
import net.minecraft.world.level.Level;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.EntityHitResult;
import net.minecraft.world.phys.HitResult;
import net.minecraft.world.phys.Vec3;

/** The player's side of leading Pikmin: the squad, throwing, the whistle, charging and dismissing. */
public final class Squads {
	public static final double SQUAD_RANGE = 64.0;
	public static final double THROW_RANGE = 14.0;
	public static final double WHISTLE_RANGE = 22.0;
	/** A Pikmin must be this close to the leader to be grabbed and thrown. */
	private static final double GRAB_RANGE = 4.0;

	// ------------------------------------------------------------------ counting

	/** Pikmin following this player, nearest first. */
	public static List<PikminEntity> squad(Player player) {
		List<PikminEntity> squad = player.level().getEntitiesOfClass(PikminEntity.class, player.getBoundingBox().inflate(SQUAD_RANGE),
			pikmin -> pikmin.isAlive() && pikmin.isLedBy(player));
		squad.sort(Comparator.comparingDouble(player::distanceToSqr));
		return squad;
	}

	public static int squadCount(Player player, PikminType type) {
		return (int) squad(player).stream().filter(p -> p.pikminType() == type).count();
	}

	/** All Pikmin out of their Onions around a point, sprouts included (the games' limit of 100). */
	public static int pikminNear(Level level, Vec3 pos, double range) {
		return level.getEntitiesOfClass(PikminEntity.class, new AABB(pos, pos).inflate(range), Entity::isAlive).size();
	}

	public static int fieldCount(Player player) {
		return pikminNear(player.level(), player.position(), 160.0);
	}

	/** Where a follower stands: a loose crowd behind the leader, packed tighter the closer it is. */
	public static Vec3 formationSpot(Player leader, PikminEntity pikmin) {
		// a stable pseudo-random slot on a sunflower spiral, so everyone keeps their place
		int slot = Math.floorMod(pikmin.getId() * 0x9E3779B1, 100);
		double radius = 0.42 * Math.sqrt(slot + 0.5);
		double angle = slot * 2.39996;
		float yaw = leader.getYRot() * Mth.DEG_TO_RAD;
		Vec3 back = new Vec3(Mth.sin(yaw), 0.0, -Mth.cos(yaw));
		double distance = 1.8 + radius;
		return leader.position().add(back.scale(distance)).add(Math.cos(angle) * radius, 0.0, Math.sin(angle) * radius);
	}

	// ------------------------------------------------------------------ aiming

	/** The point the player is looking at, at most {@code range} away. */
	public static Vec3 aimPoint(ServerPlayer player, double range) {
		Vec3 eye = player.getEyePosition();
		Vec3 end = eye.add(player.getLookAngle().scale(range));
		HitResult block = player.level().clip(new ClipContext(eye, end, ClipContext.Block.COLLIDER, ClipContext.Fluid.NONE, player));
		if (block.getType() != HitResult.Type.MISS) {
			end = block.getLocation();
		}
		EntityHitResult entity = ProjectileUtil.getEntityHitResult(player, eye, end, player.getBoundingBox().expandTowards(end.subtract(eye)).inflate(1.0),
			e -> e instanceof LivingEntity && !(e instanceof Player) && !(e instanceof PikminEntity) && e.isPickable(), range * range);
		if (entity != null) {
			return entity.getLocation();
		}
		if (block.getType() == HitResult.Type.MISS) {
			// looking at the sky: aim at the ground below the end of the reach
			HitResult down = player.level().clip(new ClipContext(end, end.add(0.0, -range, 0.0), ClipContext.Block.COLLIDER, ClipContext.Fluid.ANY, player));
			return down.getLocation();
		}
		return end;
	}

	// ------------------------------------------------------------------ commands

	public static void throwPikmin(ServerPlayer player) {
		List<PikminEntity> squad = squad(player);
		if (squad.isEmpty()) {
			return;
		}
		PikminType selected = selectedType(player, squad);
		PikminEntity thrown = squad.stream().filter(p -> p.pikminType() == selected && p.distanceTo(player) < GRAB_RANGE).findFirst().orElse(null);
		if (thrown == null) {
			// nobody close enough: call the nearest one over instead
			squad.stream().filter(p -> p.pikminType() == selected).findFirst().ifPresent(p -> p.follow(player));
			return;
		}
		Vec3 from = player.position().add(0.0, player.getBbHeight() * 0.75, 0.0).add(player.getLookAngle().multiply(0.6, 0.0, 0.6));
		Vec3 to = aimPoint(player, THROW_RANGE * thrown.pikminType().throwPower);
		thrown.throwAt(player, from, to);
		player.swing(net.minecraft.world.InteractionHand.MAIN_HAND, net.minecraft.world.item.component.SwingAnimation.DEFAULT, true);
		ModCriteria.trigger(player, "throw");
	}

	public static boolean hasWhistle(ServerPlayer player) {
		return player.getInventory().contains(stack -> stack.is(ModItems.WHISTLE));
	}

	/** Every Pikmin in the circle (and every idle one in earshot of a long blow) joins the squad. */
	public static void whistle(ServerPlayer player, float radius) {
		ServerLevel level = player.level();
		Vec3 center = aimPoint(player, WHISTLE_RANGE);
		level.playSound(null, player.getX(), player.getY(), player.getZ(), ModSounds.WHISTLE, SoundSource.PLAYERS, 1.0F, 1.6F - radius * 0.08F);
		level.sendParticles(ParticleTypes.NOTE, center.x, center.y + 0.3, center.z, 4, radius * 0.5, 0.1, radius * 0.5, 0.0);
		AABB box = new AABB(center, center).inflate(radius, 3.0, radius);
		int called = 0;
		for (PikminEntity pikmin : level.getEntitiesOfClass(PikminEntity.class, box, PikminEntity::isAlive)) {
			if (pikmin.distanceToSqr(center.x, pikmin.getY(), center.z) > radius * radius) {
				continue;
			}
			PikminEntity.State state = pikmin.state();
			if (state == PikminEntity.State.BURIED || state == PikminEntity.State.ENTER_ONION || pikmin.isLedBy(player)) {
				continue;
			}
			pikmin.follow(player);
			called++;
		}
		if (called > 0) {
			ModCriteria.trigger(player, "whistle");
		}
	}

	public static void charge(ServerPlayer player) {
		List<PikminEntity> squad = squad(player);
		if (squad.isEmpty()) {
			return;
		}
		Vec3 target = aimPoint(player, 12.0);
		for (PikminEntity pikmin : squad) {
			double spread = Math.min(3.0, Math.sqrt(squad.size()) * 0.35);
			Vec3 spot = target.add(pikmin.getRandom().nextGaussian() * spread, 0.0, pikmin.getRandom().nextGaussian() * spread);
			pikmin.charge(spot);
		}
		player.level().playSound(null, player.getX(), player.getY(), player.getZ(), ModSounds.HEY, SoundSource.PLAYERS, 1.0F, 1.3F);
		ModCriteria.trigger(player, "charge");
	}

	/** Split by colour into little groups in front of the player, like the games' dismiss. */
	public static void dismiss(ServerPlayer player) {
		List<PikminEntity> squad = squad(player);
		if (squad.isEmpty()) {
			return;
		}
		Map<PikminType, List<PikminEntity>> groups = new EnumMap<>(PikminType.class);
		for (PikminEntity pikmin : squad) {
			groups.computeIfAbsent(pikmin.pikminType(), t -> new ArrayList<>()).add(pikmin);
		}
		float yaw = player.getYRot() * Mth.DEG_TO_RAD;
		Vec3 forward = new Vec3(-Mth.sin(yaw), 0.0, Mth.cos(yaw));
		Vec3 side = new Vec3(forward.z, 0.0, -forward.x);
		int index = 0;
		for (List<PikminEntity> group : groups.values()) {
			double offset = (index - (groups.size() - 1) / 2.0) * 2.6;
			Vec3 groupCenter = player.position().add(forward.scale(2.5)).add(side.scale(offset));
			for (int i = 0; i < group.size(); i++) {
				double r = 0.45 * Math.sqrt(i + 0.5);
				double a = i * 2.39996;
				group.get(i).dismiss(groupCenter.add(Math.cos(a) * r, 0.0, Math.sin(a) * r));
			}
			index++;
		}
		player.level().playSound(null, player.getX(), player.getY(), player.getZ(), ModSounds.WHISTLE, SoundSource.PLAYERS, 0.6F, 0.8F);
	}

	// ------------------------------------------------------------------ selected type

	/** The selected type, moved on to one that is in the squad if there are none of it left. */
	public static PikminType selectedType(ServerPlayer player, List<PikminEntity> squad) {
		PikminType selected = PikminType.byId(player.getAttachedOrElse(ModAttachments.SELECTED_TYPE, 0));
		if (squad.stream().noneMatch(p -> p.pikminType() == selected) && !squad.isEmpty()) {
			PikminType nearest = squad.getFirst().pikminType();
			player.setAttached(ModAttachments.SELECTED_TYPE, nearest.ordinal());
			return nearest;
		}
		return selected;
	}

	public static void selectNext(ServerPlayer player, int step) {
		List<PikminEntity> squad = squad(player);
		int current = player.getAttachedOrElse(ModAttachments.SELECTED_TYPE, 0);
		int direction = step < 0 ? -1 : 1;
		for (int i = 1; i <= PikminType.values().length; i++) {
			PikminType candidate = PikminType.byId(current + i * direction);
			if (squad.stream().anyMatch(p -> p.pikminType() == candidate)) {
				player.setAttached(ModAttachments.SELECTED_TYPE, candidate.ordinal());
				break;
			}
		}
		syncHud(player);
	}

	// ------------------------------------------------------------------ to the clients

	public static void syncHud(ServerPlayer player) {
		List<PikminEntity> squad = squad(player);
		List<Integer> counts = new ArrayList<>();
		for (PikminType type : PikminType.values()) {
			counts.add((int) squad.stream().filter(p -> p.pikminType() == type).count());
		}
		PikminType selected = squad.isEmpty() ? PikminType.byId(player.getAttachedOrElse(ModAttachments.SELECTED_TYPE, 0)) : selectedType(player, squad);
		ServerPlayNetworking.send(player, new HudSyncPayload(counts, selected.ordinal(), fieldCount(player)));
	}

	/** Players watching an enemy Pikmin are fighting see its remaining health. */
	public static void reportEnemyHealth(ServerLevel level, LivingEntity enemy) {
		if (enemy instanceof io.github.malik05051.pikcraft.entity.PelletPosyEntity) {
			return; // posies don't lose health: they drop their pellet
		}
		EnemyHealthPayload payload = new EnemyHealthPayload(enemy.getId(), Mth.clamp(enemy.getHealth() / enemy.getMaxHealth(), 0.0F, 1.0F));
		for (ServerPlayer player : PlayerLookup.tracking(enemy)) {
			ServerPlayNetworking.send(player, payload);
		}
	}

	private Squads() {
	}
}
