package io.github.malik05051.pikcraft.entity;

import java.util.EnumMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;

import io.github.malik05051.pikcraft.PikminType;
import io.github.malik05051.pikcraft.network.OnionCutscenePayload;
import io.github.malik05051.pikcraft.network.OpenOnionMenuPayload;
import io.github.malik05051.pikcraft.registry.ModAttachments;
import io.github.malik05051.pikcraft.registry.ModCriteria;
import io.github.malik05051.pikcraft.registry.ModEntities;
import io.github.malik05051.pikcraft.registry.ModSounds;
import io.github.malik05051.pikcraft.squad.Squads;
import net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking;
import net.minecraft.core.BlockPos;
import net.minecraft.core.UUIDUtil;
import net.minecraft.core.particles.ParticleTypes;
import net.minecraft.network.chat.Component;
import net.minecraft.network.syncher.EntityDataAccessor;
import net.minecraft.network.syncher.EntityDataSerializers;
import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.util.Mth;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.InteractionResult;
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
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;
import net.minecraft.world.phys.Vec3;
import org.jspecify.annotations.Nullable;

/**
 * An Onion: the Pikmin's home ship. It sleeps until a player touches it, then makes seeds from what
 * the Pikmin bring back and keeps the Pikmin that go inside. At night it flies up out of reach.
 * Right-click makes it follow you (outdoors only); sneak + right-click opens its menu.
 */
public class OnionEntity extends PathfinderMob {
	private static final EntityDataAccessor<Integer> DATA_TYPE = SynchedEntityData.defineId(OnionEntity.class, EntityDataSerializers.INT);
	private static final EntityDataAccessor<Boolean> DATA_DORMANT = SynchedEntityData.defineId(OnionEntity.class, EntityDataSerializers.BOOLEAN);
	private static final EntityDataAccessor<Float> DATA_LIFT = SynchedEntityData.defineId(OnionEntity.class, EntityDataSerializers.FLOAT);
	private static final EntityDataAccessor<Boolean> DATA_FOLLOWING = SynchedEntityData.defineId(OnionEntity.class, EntityDataSerializers.BOOLEAN);
	private static final EntityDataAccessor<Integer> DATA_AWAKEN_TICK = SynchedEntityData.defineId(OnionEntity.class, EntityDataSerializers.INT);

	public static final int FIELD_LIMIT = 100;
	private static final float NIGHT_HEIGHT = 18.0F;

	private final Map<PikminType, Integer> stored = new EnumMap<>(PikminType.class);
	private int pendingSeeds;
	private int seedCooldown;
	private double groundY = Double.NaN;
	private @Nullable UUID followPlayer;
	private boolean warnedCave;

	private float awakeProgress;
	private float awakeProgressO;

	public OnionEntity(EntityType<? extends OnionEntity> type, Level level) {
		super(type, level);
		this.setNoGravity(true);
		this.setPersistenceRequired();
	}

	public static AttributeSupplier.Builder createAttributes() {
		return Mob.createMobAttributes().add(Attributes.MAX_HEALTH, 1000.0).add(Attributes.MOVEMENT_SPEED, 0.25).add(Attributes.KNOCKBACK_RESISTANCE, 1.0);
	}

	@Override
	protected void defineSynchedData(SynchedEntityData.Builder builder) {
		super.defineSynchedData(builder);
		builder.define(DATA_TYPE, PikminType.RED.ordinal());
		builder.define(DATA_DORMANT, true);
		builder.define(DATA_LIFT, 0.0F);
		builder.define(DATA_FOLLOWING, false);
		builder.define(DATA_AWAKEN_TICK, -1000);
	}

	// ------------------------------------------------------------------ state

	public PikminType onionType() {
		return PikminType.byId(this.entityData.get(DATA_TYPE));
	}

	public void setOnionType(PikminType type) {
		this.entityData.set(DATA_TYPE, type.ordinal());
	}

	public boolean isDormant() {
		return this.entityData.get(DATA_DORMANT);
	}

	public boolean isAwake() {
		return !this.isDormant();
	}

	/** How high the Onion has flown for the night, in blocks. */
	public float lift() {
		return this.entityData.get(DATA_LIFT);
	}

	public boolean isFollowing() {
		return this.entityData.get(DATA_FOLLOWING);
	}

	public int awakenTick() {
		return this.entityData.get(DATA_AWAKEN_TICK);
	}

	/** Client side: how far the wake-up animation has got, 0 to 1. */
	public float awakeProgress(float partialTicks) {
		return Mth.lerp(partialTicks, this.awakeProgressO, this.awakeProgress);
	}

	@Override
	public void tick() {
		super.tick();
		if (this.level().isClientSide()) {
			this.awakeProgressO = this.awakeProgress;
			this.awakeProgress = this.isDormant() ? 0.0F : Math.min(1.0F, this.awakeProgress + 1.0F / 40.0F);
		}
	}

	public boolean acceptsDeliveries() {
		return this.isAwake() && this.lift() < 0.5F;
	}

	public int storedCount(PikminType type) {
		return this.stored.getOrDefault(type, 0);
	}

	public void store(PikminType type, int count) {
		this.stored.merge(type, count, Integer::sum);
	}

	public Vec3 groundPosition() {
		return new Vec3(this.getX(), Double.isNaN(this.groundY) ? this.getY() : this.groundY, this.getZ());
	}

	private static boolean isNight(Level level) {
		long time = level.getOverworldClockTime() % 24000L;
		return time >= 12800L && time <= 23200L;
	}

	// ------------------------------------------------------------------ players

	@Override
	protected InteractionResult mobInteract(Player player, InteractionHand hand) {
		if (hand != InteractionHand.MAIN_HAND) {
			return InteractionResult.PASS;
		}
		if (this.level().isClientSide()) {
			return InteractionResult.SUCCESS;
		}
		ServerPlayer serverPlayer = (ServerPlayer) player;
		if (this.isDormant()) {
			this.awaken(serverPlayer);
		} else if (this.lift() > 0.5F) {
			player.sendOverlayMessage(Component.translatable("message.pikcraft.onion_night"));
		} else if (player.isShiftKeyDown()) {
			this.openMenu(serverPlayer);
		} else {
			this.toggleFollow(serverPlayer);
		}
		return InteractionResult.SUCCESS;
	}

	public void openMenu(ServerPlayer player) {
		PikminType type = this.onionType();
		ServerPlayNetworking.send(player, new OpenOnionMenuPayload(this.getId(), type.ordinal(), this.storedCount(type),
			Squads.squadCount(player, type), Squads.fieldCount(player)));
	}

	private void toggleFollow(ServerPlayer player) {
		boolean following = !(this.isFollowing() && player.getUUID().equals(this.followPlayer));
		this.followPlayer = following ? player.getUUID() : null;
		this.entityData.set(DATA_FOLLOWING, following);
		this.warnedCave = false;
		player.sendOverlayMessage(Component.translatable(following ? "message.pikcraft.onion_follow" : "message.pikcraft.onion_stay"));
		if (following) {
			ModCriteria.trigger(player, "onion_follow");
		}
	}

	/** Woken by a player's touch: it pushes them back and spits its first seed. */
	public void awaken(ServerPlayer player) {
		if (!this.isDormant()) {
			return;
		}
		ServerLevel level = (ServerLevel) this.level();
		this.entityData.set(DATA_DORMANT, false);
		this.entityData.set(DATA_AWAKEN_TICK, this.tickCount);
		this.playSound(ModSounds.ONION_AWAKEN, 2.0F, 1.0F);
		level.sendParticles(ParticleTypes.END_ROD, this.getX(), this.getY() + 2.0, this.getZ(), 40, 1.0, 1.0, 1.0, 0.08);
		Vec3 push = player.position().subtract(this.position()).multiply(1.0, 0.0, 1.0);
		push = push.lengthSqr() < 1.0E-3 ? new Vec3(1.0, 0.0, 0.0) : push.normalize();
		player.push(push.x * 1.4, 0.45, push.z * 1.4);
		player.needsSync = true;
		this.pendingSeeds += 1;
		this.seedCooldown = 30;
		ServerPlayNetworking.send(player, new OnionCutscenePayload(this.getId(), OnionCutscenePayload.AWAKEN));
		ModCriteria.trigger(player, "awaken_onion");
		int awakened = player.getAttachedOrElse(ModAttachments.ONIONS_AWAKENED, 0) + 1;
		player.setAttached(ModAttachments.ONIONS_AWAKENED, awakened);
		if (awakened >= 2) {
			ModCriteria.trigger(player, "second_onion");
		}
	}

	/** Take Pikmin out of the Onion into the player's squad. */
	public void withdraw(ServerPlayer player, int count) {
		PikminType type = this.onionType();
		int room = Math.max(0, FIELD_LIMIT - Squads.fieldCount(player));
		int amount = Math.min(Math.min(count, this.storedCount(type)), room);
		if (amount <= 0) {
			return;
		}
		this.stored.put(type, this.storedCount(type) - amount);
		ServerLevel level = (ServerLevel) this.level();
		for (int i = 0; i < amount; i++) {
			PikminEntity pikmin = ModEntities.pikmin(type).create(level, EntitySpawnReason.MOB_SUMMONED);
			if (pikmin == null) {
				continue;
			}
			double angle = this.random.nextDouble() * Mth.TWO_PI;
			Vec3 ground = this.groundPosition();
			pikmin.snapTo(ground.x + Math.cos(angle) * 1.3, ground.y + 0.2, ground.z + Math.sin(angle) * 1.3, this.random.nextFloat() * 360.0F, 0.0F);
			pikmin.setMaturity(PikminEntity.LEAF);
			level.addFreshEntity(pikmin);
			pikmin.follow(player);
		}
		this.playSound(ModSounds.ONION_SEED, 1.0F, 1.2F);
	}

	/** Send the player's squad Pikmin of this colour into the Onion. */
	public void deposit(ServerPlayer player, int count) {
		int sent = 0;
		for (PikminEntity pikmin : Squads.squad(player)) {
			if (sent >= count) {
				break;
			}
			if (pikmin.pikminType() == this.onionType()) {
				pikmin.goIntoOnion(this);
				sent++;
			}
		}
	}

	// ------------------------------------------------------------------ seeds

	/** An object carried home: the Onion takes it in and makes seeds. */
	public void deliver(CarryableEntity carried) {
		ServerLevel level = (ServerLevel) this.level();
		int seeds = carried.seedsFor(this.onionType());
		this.pendingSeeds += seeds;
		this.playSound(ModSounds.ONION_SUCK, 1.5F, 1.0F);
		level.sendParticles(ParticleTypes.END_ROD, carried.getX(), carried.getY() + 0.5, carried.getZ(), 15, 0.3, 1.5, 0.3, 0.05);
		String event = carried instanceof CorpseEntity ? "deliver_corpse" : "deliver_pellet";
		for (ServerPlayer player : level.getEntitiesOfClass(ServerPlayer.class, this.getBoundingBox().inflate(24.0))) {
			ModCriteria.trigger(player, event);
		}
		carried.onDelivered();
		carried.ejectPassengers();
		carried.stopRiding();
		carried.discard();
	}

	private void spitSeed(ServerLevel level) {
		PikminType type = this.onionType();
		if (Squads.pikminNear(level, this.position(), 160.0) >= FIELD_LIMIT) {
			this.store(type, 1);
			return;
		}
		SeedEntity seed = ModEntities.SEED.create(level, EntitySpawnReason.TRIGGERED);
		if (seed == null) {
			return;
		}
		double angle = this.random.nextDouble() * Mth.TWO_PI;
		double speed = 0.18 + this.random.nextDouble() * 0.12;
		seed.setSeedType(type);
		seed.snapTo(this.getX(), this.getY() + this.getBbHeight(), this.getZ(), 0.0F, 0.0F);
		seed.setDeltaMovement(Math.cos(angle) * speed, 0.55 + this.random.nextDouble() * 0.15, Math.sin(angle) * speed);
		level.addFreshEntity(seed);
		this.playSound(ModSounds.ONION_SEED, 1.0F, 0.9F + this.random.nextFloat() * 0.3F);
	}

	// ------------------------------------------------------------------ every tick

	@Override
	protected void customServerAiStep(ServerLevel level) {
		super.customServerAiStep(level);
		this.setDeltaMovement(Vec3.ZERO);
		if (Double.isNaN(this.groundY)) {
			this.groundY = this.getY();
		}
		if (this.isDormant()) {
			// a touch wakes it up
			for (ServerPlayer player : level.getEntitiesOfClass(ServerPlayer.class, this.getBoundingBox().inflate(0.4), p -> !p.isSpectator())) {
				this.awaken(player);
				break;
			}
			return;
		}

		boolean night = isNight(level);
		float lift = this.lift();
		if (night && lift == 0.0F) {
			this.takeInIdlePikmin(level);
			this.playSound(ModSounds.ONION_FLY, 2.0F, 1.0F);
			this.entityData.set(DATA_FOLLOWING, false);
			this.followPlayer = null;
		}
		float targetLift = night ? NIGHT_HEIGHT : 0.0F;
		if (lift != targetLift) {
			lift = lift < targetLift ? Math.min(targetLift, lift + 0.12F) : Math.max(targetLift, lift - 0.12F);
			this.entityData.set(DATA_LIFT, lift);
			if (this.tickCount % 3 == 0) {
				level.sendParticles(ParticleTypes.CLOUD, this.getX(), this.getY(), this.getZ(), 2, 0.6, 0.1, 0.6, 0.01);
			}
		}

		if (this.isFollowing() && lift == 0.0F) {
			this.followLeader(level);
		}
		double y = this.groundY + lift;
		if (Math.abs(this.getY() - y) > 1.0E-3) {
			this.setPos(this.getX(), y, this.getZ());
		}

		if (this.pendingSeeds > 0 && lift == 0.0F && --this.seedCooldown <= 0) {
			this.seedCooldown = 5;
			this.pendingSeeds--;
			this.spitSeed(level);
		}
	}

	/** Walk after the player, but only out in the open: Onions don't fit in caves. */
	private void followLeader(ServerLevel level) {
		Player player = this.followPlayer == null ? null : level.getPlayerByUUID(this.followPlayer);
		if (player == null || player.distanceToSqr(this) > 96 * 96) {
			this.entityData.set(DATA_FOLLOWING, false);
			this.followPlayer = null;
			return;
		}
		Vec3 toPlayer = player.position().subtract(this.position()).multiply(1.0, 0.0, 1.0);
		double distance = toPlayer.length();
		if (distance < 4.0) {
			return;
		}
		Vec3 step = toPlayer.scale(Math.min(0.22, distance - 4.0) / distance);
		double nx = this.getX() + step.x;
		double nz = this.getZ() + step.z;
		BlockPos column = BlockPos.containing(nx, this.groundY, nz);
		int ground = level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES, column.getX(), column.getZ());
		BlockPos surface = new BlockPos(column.getX(), ground, column.getZ());
		boolean outdoors = level.canSeeSky(surface) && Math.abs(ground - this.groundY) <= 2.0
			&& Math.abs(player.getY() - ground) < 6.0;
		if (!outdoors) {
			if (!this.warnedCave && player instanceof ServerPlayer serverPlayer) {
				serverPlayer.sendOverlayMessage(Component.translatable("message.pikcraft.onion_no_cave"));
				this.warnedCave = true;
			}
			return;
		}
		this.warnedCave = false;
		this.groundY = Mth.lerp(0.25, this.groundY, ground);
		this.setPos(nx, this.groundY, nz);
		this.setYRot((float) (Mth.atan2(step.z, step.x) * Mth.RAD_TO_DEG) - 90.0F);
	}

	/** At dusk the Pikmin left idle near their Onion go inside for the night. */
	private void takeInIdlePikmin(ServerLevel level) {
		List<PikminEntity> idle = level.getEntitiesOfClass(PikminEntity.class, this.getBoundingBox().inflate(48.0),
			p -> p.pikminType() == this.onionType() && p.state() != PikminEntity.State.FOLLOW && p.state() != PikminEntity.State.BURIED);
		for (PikminEntity pikmin : idle) {
			this.store(pikmin.pikminType(), 1);
			level.sendParticles(ParticleTypes.END_ROD, pikmin.getX(), pikmin.getY() + 0.3, pikmin.getZ(), 3, 0.1, 0.4, 0.1, 0.02);
			pikmin.discard();
		}
	}

	// ------------------------------------------------------------------ an Onion can't be hurt or moved

	@Override
	public boolean isInvulnerableTo(ServerLevel level, DamageSource source) {
		return !source.is(DamageTypes.GENERIC_KILL);
	}

	@Override
	public boolean isPushable() {
		return false;
	}

	@Override
	public boolean canBeLeashed() {
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

	// ------------------------------------------------------------------ saving

	@Override
	protected void addAdditionalSaveData(ValueOutput output) {
		super.addAdditionalSaveData(output);
		output.putInt("OnionType", this.onionType().ordinal());
		output.putBoolean("Dormant", this.isDormant());
		output.putDouble("GroundY", Double.isNaN(this.groundY) ? this.getY() : this.groundY);
		output.putInt("PendingSeeds", this.pendingSeeds);
		for (PikminType type : PikminType.values()) {
			output.putInt("Stored_" + type.id(), this.storedCount(type));
		}
		if (this.followPlayer != null) {
			output.store("FollowPlayer", UUIDUtil.CODEC, this.followPlayer);
		}
	}

	@Override
	protected void readAdditionalSaveData(ValueInput input) {
		super.readAdditionalSaveData(input);
		this.setOnionType(PikminType.byId(input.getIntOr("OnionType", 0)));
		this.entityData.set(DATA_DORMANT, input.getBooleanOr("Dormant", true));
		this.groundY = input.getDoubleOr("GroundY", this.getY());
		this.pendingSeeds = input.getIntOr("PendingSeeds", 0);
		for (PikminType type : PikminType.values()) {
			this.stored.put(type, input.getIntOr("Stored_" + type.id(), 0));
		}
		this.followPlayer = input.read("FollowPlayer", UUIDUtil.CODEC).orElse(null);
		this.entityData.set(DATA_FOLLOWING, this.followPlayer != null);
	}
}
