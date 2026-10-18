package io.github.malik05051.pikcraft.entity;

import java.util.Comparator;
import java.util.List;
import java.util.Optional;
import java.util.UUID;

import io.github.malik05051.pikcraft.PikminType;
import io.github.malik05051.pikcraft.registry.ModCriteria;
import io.github.malik05051.pikcraft.registry.ModEntities;
import io.github.malik05051.pikcraft.registry.ModSounds;
import io.github.malik05051.pikcraft.squad.Squads;
import net.minecraft.core.BlockPos;
import net.minecraft.core.UUIDUtil;
import net.minecraft.core.particles.ParticleTypes;
import net.minecraft.network.syncher.EntityDataAccessor;
import net.minecraft.network.syncher.EntityDataSerializers;
import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.tags.BlockTags;
import net.minecraft.tags.DamageTypeTags;
import net.minecraft.util.Mth;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.damagesource.DamageTypes;
import net.minecraft.world.effect.MobEffectInstance;
import net.minecraft.world.effect.MobEffects;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.Mob;
import net.minecraft.world.entity.PathfinderMob;
import net.minecraft.world.entity.ai.attributes.AttributeSupplier;
import net.minecraft.world.entity.ai.attributes.Attributes;
import net.minecraft.world.entity.ai.control.FlyingMoveControl;
import net.minecraft.world.entity.ai.navigation.FlyingPathNavigation;
import net.minecraft.world.entity.ai.navigation.GroundPathNavigation;
import net.minecraft.world.entity.ai.navigation.PathNavigation;
import net.minecraft.world.entity.animal.golem.AbstractGolem;
import net.minecraft.world.entity.npc.villager.AbstractVillager;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;
import org.jspecify.annotations.Nullable;

/**
 * A Pikmin. One class for all seven types; the type comes from the entity type.
 *
 * Behaviour is a small state machine run on the server: buried sprouts wait to be plucked, then
 * Pikmin follow their leader, get thrown, attack, carry things home, drink nectar and go back into
 * their Onion at night.
 */
public class PikminEntity extends PathfinderMob {
	public static final int LEAF = 0;
	public static final int BUD = 1;
	public static final int FLOWER = 2;

	public enum State {
		BURIED, IDLE, FOLLOW, THROWN, ATTACK, CARRY, DRINK, CHARGE, ENTER_ONION;

		public static State byId(int id) {
			State[] values = values();
			return values[Math.floorMod(id, values.length)];
		}
	}

	private static final EntityDataAccessor<Byte> DATA_MATURITY = SynchedEntityData.defineId(PikminEntity.class, EntityDataSerializers.BYTE);
	private static final EntityDataAccessor<Byte> DATA_STATE = SynchedEntityData.defineId(PikminEntity.class, EntityDataSerializers.BYTE);
	private static final EntityDataAccessor<Boolean> DATA_DROWNING = SynchedEntityData.defineId(PikminEntity.class, EntityDataSerializers.BOOLEAN);
	private static final EntityDataAccessor<Integer> DATA_ACTION_TICK = SynchedEntityData.defineId(PikminEntity.class, EntityDataSerializers.INT);

	private static final int GROW_TICKS = 2400;
	private static final int DROWN_TICKS = 100;

	private final PikminType type;
	private @Nullable UUID leader;
	private @Nullable Entity target;
	private Vec3 latchOffset = Vec3.ZERO;
	private boolean latched;
	private @Nullable Vec3 destination;
	private int stateTicks;
	private int attackCooldown;
	private int buriedTicks;
	private int drownTicks;
	private int thinkCooldown;

	public PikminEntity(EntityType<? extends PikminEntity> entityType, Level level) {
		super(entityType, level);
		this.type = ModEntities.pikminTypeOf(entityType);
		if (this.type.flies()) {
			this.moveControl = new FlyingMoveControl<>(this, 20, true);
			this.setNoGravity(true);
		}
	}

	public static AttributeSupplier.Builder createAttributes(PikminType type) {
		return Mob.createMobAttributes()
			.add(Attributes.MAX_HEALTH, type.health)
			.add(Attributes.MOVEMENT_SPEED, 0.32 * type.speed)
			.add(Attributes.FLYING_SPEED, 0.45 * type.speed)
			.add(Attributes.ATTACK_DAMAGE, type.attack)
			.add(Attributes.FOLLOW_RANGE, 48.0)
			.add(Attributes.STEP_HEIGHT, 1.0);
	}

	@Override
	protected PathNavigation createNavigation(Level level) {
		if (ModEntities.pikminTypeOf(this.getType()).flies()) {
			FlyingPathNavigation navigation = new FlyingPathNavigation(this, level);
			navigation.setCanOpenDoors(false);
			navigation.setCanFloat(true);
			return navigation;
		}
		GroundPathNavigation navigation = new GroundPathNavigation(this, level);
		navigation.setCanFloat(true);
		return navigation;
	}

	@Override
	protected void defineSynchedData(SynchedEntityData.Builder builder) {
		super.defineSynchedData(builder);
		builder.define(DATA_MATURITY, (byte) LEAF);
		builder.define(DATA_STATE, (byte) State.IDLE.ordinal());
		builder.define(DATA_DROWNING, false);
		builder.define(DATA_ACTION_TICK, 0);
	}

	// ------------------------------------------------------------------ accessors

	public PikminType pikminType() {
		return this.type;
	}

	public int maturity() {
		return this.entityData.get(DATA_MATURITY);
	}

	public void setMaturity(int maturity) {
		this.entityData.set(DATA_MATURITY, (byte) Mth.clamp(this.type.grows() ? maturity : LEAF, LEAF, FLOWER));
	}

	public State state() {
		return State.byId(this.entityData.get(DATA_STATE));
	}

	public boolean isDrowning() {
		return this.entityData.get(DATA_DROWNING);
	}

	/** Tick count of the last attack or pluck, for animations. */
	public int actionTick() {
		return this.entityData.get(DATA_ACTION_TICK);
	}

	public @Nullable UUID leader() {
		return this.leader;
	}

	public boolean isLedBy(Player player) {
		return this.state() == State.FOLLOW && player.getUUID().equals(this.leader);
	}

	public @Nullable Entity target() {
		return this.target;
	}

	public boolean isLatched() {
		return this.latched;
	}

	public void setState(State state) {
		if (this.state() == State.CARRY && state != State.CARRY && this.target instanceof CarryableEntity carryable) {
			carryable.removeCarrier(this);
		}
		this.entityData.set(DATA_STATE, (byte) state.ordinal());
		this.stateTicks = 0;
		this.latched = false;
		if (state != State.ATTACK && state != State.CARRY && state != State.DRINK && state != State.ENTER_ONION) {
			this.target = null;
		}
		this.setNoGravity(this.type.flies() || state == State.BURIED);
		if (state == State.BURIED) {
			this.setDeltaMovement(Vec3.ZERO);
		}
		this.getNavigation().stop();
	}

	private void markAction() {
		this.entityData.set(DATA_ACTION_TICK, this.tickCount);
	}

	private float speedFactor() {
		return switch (this.maturity()) {
			case BUD -> 1.15F;
			case FLOWER -> 1.3F;
			default -> 1.0F;
		};
	}

	// ------------------------------------------------------------------ commands from the leader

	public void bury() {
		this.setState(State.BURIED);
		this.buriedTicks = 0;
	}

	public void follow(Player player) {
		boolean joined = this.state() != State.FOLLOW;
		this.leader = player.getUUID();
		this.setState(State.FOLLOW);
		if (joined && this.random.nextInt(3) == 0) {
			this.playSound(ModSounds.HEY, 0.5F, 0.9F + this.random.nextFloat() * 0.3F);
		}
	}

	public void dismiss(Vec3 spot) {
		this.setState(State.IDLE);
		this.destination = spot;
	}

	/** Thrown by the leader towards a point: a lob that lands there. */
	public void throwAt(Player player, Vec3 from, Vec3 to) {
		this.leader = player.getUUID();
		this.setState(State.THROWN);
		this.setPos(from.x, from.y, from.z);
		Vec3 delta = to.subtract(from);
		double horizontal = Math.sqrt(delta.x * delta.x + delta.z * delta.z);
		double ticks = Mth.clamp(horizontal / 0.6, 10.0, 26.0) / this.type.throwPower;
		double gravity = 0.08;
		// mobs lose 2% speed per tick in the air; aim a little long to make up for it
		double drag = 1.08 + horizontal * 0.004;
		this.setDeltaMovement(delta.x / ticks * drag, (delta.y + 0.5 * gravity * ticks * ticks) / ticks, delta.z / ticks * drag);
		this.setNoGravity(false);
		this.markAction();
		this.playSound(ModSounds.THROW, 0.7F, 1.0F + this.random.nextFloat() * 0.2F);
	}

	public void charge(Vec3 spot) {
		this.setState(State.CHARGE);
		this.destination = spot;
	}

	public void goIntoOnion(OnionEntity onion) {
		this.setState(State.ENTER_ONION);
		this.target = onion;
	}

	/** Plucked from the ground by a player. */
	public void pluck(Player player) {
		this.setDeltaMovement(0.0, 0.45, 0.0);
		this.setPos(this.getX(), this.getY() + 0.4, this.getZ());
		this.follow(player);
		this.markAction();
		this.playSound(ModSounds.PLUCK, 1.0F, 0.9F + this.random.nextFloat() * 0.2F);
		if (player instanceof ServerPlayer serverPlayer) {
			ModCriteria.trigger(serverPlayer, "pluck");
			ModCriteria.trigger(serverPlayer, "pikmin_" + this.type.id());
		}
	}

	// ------------------------------------------------------------------ interaction

	@Override
	protected InteractionResult mobInteract(Player player, InteractionHand hand) {
		if (this.state() == State.BURIED) {
			if (!this.level().isClientSide()) {
				this.pluck(player);
			}
			return InteractionResult.SUCCESS;
		}
		return super.mobInteract(player, hand);
	}

	@Override
	public boolean isPushable() {
		return this.state() != State.BURIED && this.state() != State.CARRY && !this.latched;
	}

	@Override
	protected void doPush(Entity entity) {
		if (!(entity instanceof PikminEntity) || this.random.nextInt(4) == 0) {
			super.doPush(entity);
		}
	}

	@Override
	public boolean isInvulnerableTo(ServerLevel level, DamageSource source) {
		if (this.state() == State.BURIED && !source.is(DamageTypes.GENERIC_KILL)) {
			return true;
		}
		if (source.getEntity() instanceof Player || source.getEntity() instanceof PikminEntity) {
			return true; // no friendly fire
		}
		if (this.type == PikminType.YELLOW && source.is(DamageTypeTags.IS_LIGHTNING)) {
			return true;
		}
		if (this.type == PikminType.ROCK && (source.is(DamageTypes.FALLING_BLOCK) || source.is(DamageTypes.FALLING_ANVIL)
			|| source.is(DamageTypes.FALLING_STALACTITE) || source.is(DamageTypes.CRAMMING))) {
			return true;
		}
		return super.isInvulnerableTo(level, source);
	}

	@Override
	public boolean canBeAffected(MobEffectInstance effect) {
		if (this.type == PikminType.WHITE && (effect.is(MobEffects.POISON) || effect.is(MobEffects.WITHER))) {
			return false;
		}
		return super.canBeAffected(effect);
	}

	@Override
	public boolean canBreatheUnderwater() {
		return this.type == PikminType.BLUE;
	}

	@Override
	public boolean causeFallDamage(double fallDistance, float damageModifier, DamageSource damageSource) {
		return false;
	}

	@Override
	public void die(DamageSource source) {
		super.die(source);
		if (this.level() instanceof ServerLevel level) {
			level.sendParticles(ParticleTypes.SOUL, this.getX(), this.getY() + 0.5, this.getZ(), 6, 0.15, 0.3, 0.15, 0.03);
			this.playSound(ModSounds.DIE, 0.8F, 1.0F + this.random.nextFloat() * 0.2F);
			// White Pikmin are poisonous to whatever eats them
			if (this.type == PikminType.WHITE && source.getEntity() instanceof LivingEntity attacker && !(attacker instanceof Player)) {
				attacker.addEffect(new MobEffectInstance(MobEffects.POISON, 200, 2), this);
			}
		}
	}

	@Override
	public boolean removeWhenFarAway(double distanceToClosestPlayer) {
		return false;
	}

	// ------------------------------------------------------------------ the brain

	@Override
	public void tick() {
		super.tick();
		if (this.state() == State.BURIED) {
			this.setDeltaMovement(Vec3.ZERO);
		}
	}

	@Override
	protected void customServerAiStep(ServerLevel level) {
		super.customServerAiStep(level);
		this.stateTicks++;
		if (this.attackCooldown > 0) {
			this.attackCooldown--;
		}
		this.handleWater(level);

		switch (this.state()) {
			case BURIED -> this.tickBuried();
			case IDLE -> this.tickIdle(level);
			case FOLLOW -> this.tickFollow(level);
			case THROWN -> this.tickThrown(level);
			case CHARGE -> this.tickCharge(level);
			case ATTACK -> this.tickAttack(level);
			case CARRY -> this.tickCarry();
			case DRINK -> this.tickDrink(level);
			case ENTER_ONION -> this.tickEnterOnion(level);
		}
	}

	private void handleWater(ServerLevel level) {
		boolean drowning = this.isInWater() && this.type != PikminType.BLUE && !this.type.flies();
		if (drowning) {
			this.drownTicks++;
			if (this.random.nextInt(4) == 0) {
				this.setDeltaMovement(this.getDeltaMovement().add(0.0, 0.08, 0.0)); // flail at the surface
			}
			if (this.drownTicks > DROWN_TICKS) {
				this.hurtServer(level, this.damageSources().drown(), 100.0F);
			}
		} else {
			this.drownTicks = Math.max(0, this.drownTicks - 2);
		}
		if (this.entityData.get(DATA_DROWNING) != drowning) {
			this.entityData.set(DATA_DROWNING, drowning);
		}
	}

	private void tickBuried() {
		this.buriedTicks++;
		if (this.type.grows() && this.buriedTicks % GROW_TICKS == 0 && this.maturity() < FLOWER) {
			this.setMaturity(this.maturity() + 1);
		}
	}

	private void tickIdle(ServerLevel level) {
		if (this.destination != null) {
			if (this.distanceToSqr(this.destination) > 1.0 && this.stateTicks < 200) {
				this.moveTo(this.destination, 1.0);
				return;
			}
			this.destination = null;
		}
		if (--this.thinkCooldown > 0) {
			return;
		}
		this.thinkCooldown = 10 + this.random.nextInt(10);
		// fight back against enemies that come close
		LivingEntity enemy = this.nearestEnemy(level, 2.5);
		if (enemy != null) {
			this.startAttack(enemy, false);
			return;
		}
		if (this.findWork(level, 5.0)) {
			return;
		}
		if (this.random.nextInt(8) == 0) {
			this.getNavigation().moveTo(
				this.getX() + this.random.nextGaussian() * 1.5, this.getY(), this.getZ() + this.random.nextGaussian() * 1.5, 0.5);
		}
	}

	private void tickFollow(ServerLevel level) {
		Player leader = this.leaderPlayer(level);
		if (leader == null || leader.isSpectator() || leader.distanceToSqr(this) > 64 * 64 || leader.level() != this.level()) {
			this.setState(State.IDLE);
			return;
		}
		Vec3 spot = Squads.formationSpot(leader, this);
		double distance = this.distanceToSqr(spot);
		if (distance > 30 * 30) {
			this.teleportTo(spot.x, spot.y, spot.z); // left far behind: catch up
		} else if (distance > 1.2) {
			this.moveTo(spot, distance > 36 ? 1.6 : 1.2);
		} else {
			this.getNavigation().stop();
			this.getLookControl().setLookAt(leader, 30.0F, 30.0F);
		}
	}

	private void tickThrown(ServerLevel level) {
		// latch onto anything hit in flight
		if (this.stateTicks > 2) {
			for (Entity entity : level.getEntities(this, this.getBoundingBox().inflate(0.3), this::isAttackable)) {
				this.onImpact(level, entity);
				this.startAttack((LivingEntity) entity, true);
				return;
			}
		}
		boolean landed = this.stateTicks > 3 && (this.onGround() || this.isInWater() || this.horizontalCollision);
		if (landed || this.stateTicks > 80) {
			this.onImpact(level, null);
			this.setNoGravity(this.type.flies());
			this.resolveLanding(level);
		}
	}

	private void tickCharge(ServerLevel level) {
		if (this.destination == null || this.stateTicks > 200) {
			this.setState(State.IDLE);
			return;
		}
		LivingEntity enemy = this.nearestEnemy(level, 1.8);
		if (enemy != null) {
			this.startAttack(enemy, false);
			return;
		}
		if (this.distanceToSqr(this.destination) < 2.5) {
			this.resolveLanding(level);
		} else {
			this.moveTo(this.destination, 1.7);
		}
	}

	private void tickAttack(ServerLevel level) {
		Entity target = this.target;
		if (target == null || !target.isAlive() || target.level() != this.level()) {
			this.afterFight(level);
			return;
		}
		if (this.latched) {
			// hold on: ride along with the target
			float yaw = target.getYRot() * Mth.DEG_TO_RAD;
			Vec3 offset = this.latchOffset.yRot(-yaw);
			this.setPos(target.getX() + offset.x, target.getY() + offset.y, target.getZ() + offset.z);
			this.setDeltaMovement(Vec3.ZERO);
			this.getLookControl().setLookAt(target);
			if (this.random.nextInt(400) == 0 || target.isInWater() && this.type != PikminType.BLUE) {
				this.latched = false; // shaken off
			}
		} else {
			double reach = 0.7 + target.getBbWidth() * 0.6;
			if (this.distanceToSqr(target) > reach * reach + 0.3) {
				this.getNavigation().moveTo(target, 1.4 * this.speedFactor());
				if (this.stateTicks > 400) {
					this.setState(State.IDLE);
				}
				return;
			}
			this.getNavigation().stop();
			this.getLookControl().setLookAt(target);
		}
		if (this.attackCooldown == 0) {
			this.attackCooldown = 14;
			this.markAction();
			float damage = (float) this.getAttributeValue(Attributes.ATTACK_DAMAGE);
			if (target.hurtServer(level, this.damageSources().mobAttack(this), damage)) {
				this.playSound(ModSounds.ATTACK, 0.35F, 1.1F + this.random.nextFloat() * 0.3F);
				if (target instanceof LivingEntity living) {
					Squads.reportEnemyHealth(level, living);
				}
			}
		}
	}

	private void afterFight(ServerLevel level) {
		// the enemy just became a corpse: carry it home
		if (this.findWork(level, 4.0)) {
			return;
		}
		Player leader = this.leaderPlayer(level);
		this.setState(State.IDLE);
		if (leader != null && leader.distanceToSqr(this) < 6 * 6) {
			this.follow(leader);
		}
	}

	private void tickCarry() {
		if (!(this.target instanceof CarryableEntity carryable) || !carryable.isAlive() || !carryable.hasCarrier(this)) {
			this.setState(State.IDLE);
			return;
		}
		Vec3 spot = carryable.carrierSpot(this);
		if (this.distanceToSqr(spot) > 4.0) {
			this.moveTo(spot, 1.4);
		} else {
			this.getNavigation().stop();
			this.setPos(Mth.lerp(0.5, this.getX(), spot.x), this.getY(), Mth.lerp(0.5, this.getZ(), spot.z));
			this.getLookControl().setLookAt(carryable);
		}
	}

	private void tickDrink(ServerLevel level) {
		if (!(this.target instanceof NectarEntity nectar) || !nectar.isAlive()) {
			this.setState(State.IDLE);
			return;
		}
		if (this.distanceToSqr(nectar) > 1.2) {
			this.moveTo(nectar.position(), 1.2);
			this.stateTicks = 0;
			return;
		}
		this.getNavigation().stop();
		this.getLookControl().setLookAt(nectar);
		if (this.stateTicks == 1) {
			this.markAction();
		}
		if (this.stateTicks > 40) {
			if (nectar.sip()) {
				this.setMaturity(FLOWER);
				this.playSound(ModSounds.DRINK, 0.8F, 1.2F);
				level.sendParticles(ParticleTypes.HAPPY_VILLAGER, this.getX(), this.getY() + 0.8, this.getZ(), 5, 0.2, 0.2, 0.2, 0.0);
				Player leader = this.leaderPlayer(level);
				if (leader instanceof ServerPlayer serverPlayer) {
					ModCriteria.trigger(serverPlayer, "nectar");
				}
			}
			this.setState(State.IDLE);
		}
	}

	private void tickEnterOnion(ServerLevel level) {
		if (!(this.target instanceof OnionEntity onion) || !onion.isAlive() || this.stateTicks > 600) {
			this.setState(State.IDLE);
			return;
		}
		double dx = onion.getX() - this.getX();
		double dz = onion.getZ() - this.getZ();
		if (dx * dx + dz * dz < 2.5 || this.stateTicks > 300) {
			onion.store(this.type, 1);
			level.sendParticles(ParticleTypes.END_ROD, this.getX(), this.getY() + 0.3, this.getZ(), 3, 0.1, 0.4, 0.1, 0.02);
			this.discard();
		} else {
			this.moveTo(onion.position(), 1.3);
		}
	}

	// ------------------------------------------------------------------ helpers

	private void moveTo(Vec3 spot, double speed) {
		double y = this.type.flies() ? spot.y + 1.2 : spot.y;
		if (!this.getNavigation().moveTo(spot.x, y, spot.z, speed * this.speedFactor())) {
			// no path: walk straight at it
			this.getMoveControl().setWantedPosition(spot.x, y, spot.z, speed * this.speedFactor());
		}
	}

	private @Nullable Player leaderPlayer(ServerLevel level) {
		return this.leader == null ? null : level.getPlayerByUUID(this.leader);
	}

	public boolean isAttackable(Entity entity) {
		if (!(entity instanceof LivingEntity living) || !living.isAlive() || entity instanceof Player) {
			return false;
		}
		if (entity instanceof PikminEntity || entity instanceof OnionEntity || entity instanceof CarryableEntity) {
			return false;
		}
		return !(entity instanceof AbstractVillager) && !(entity instanceof AbstractGolem) && !(entity.isInvulnerable());
	}

	private @Nullable LivingEntity nearestEnemy(ServerLevel level, double range) {
		AABB box = this.getBoundingBox().inflate(range);
		List<Mob> enemies = level.getEntitiesOfClass(Mob.class, box,
			mob -> this.isAttackable(mob) && (mob instanceof net.minecraft.world.entity.monster.Enemy || mob instanceof PelletPosyEntity
				|| mob instanceof NectarEggEntity));
		return enemies.stream().min(Comparator.comparingDouble(this::distanceToSqr)).orElse(null);
	}

	/** Look for something to carry or nectar to drink nearby. */
	private boolean findWork(ServerLevel level, double range) {
		AABB box = this.getBoundingBox().inflate(range);
		Optional<CarryableEntity> carryable = level.getEntitiesOfClass(CarryableEntity.class, box, c -> c.isAlive() && c.wantsCarriers()).stream()
			.min(Comparator.comparingDouble(this::distanceToSqr));
		if (carryable.isPresent()) {
			// become a carrier first: the object only keeps carriers that are carrying it
			this.setState(State.CARRY);
			this.target = carryable.get();
			if (carryable.get().addCarrier(this)) {
				return true;
			}
			this.setState(State.IDLE);
		}
		if (this.maturity() < FLOWER && this.type.grows()) {
			Optional<NectarEntity> nectar = level.getEntitiesOfClass(NectarEntity.class, box.inflate(3.0), NectarEntity::isAlive).stream()
				.min(Comparator.comparingDouble(this::distanceToSqr));
			if (nectar.isPresent()) {
				this.setState(State.DRINK);
				this.target = nectar.get();
				return true;
			}
		}
		return false;
	}

	private void startAttack(LivingEntity enemy, boolean latch) {
		this.setState(State.ATTACK);
		this.target = enemy;
		this.latched = latch && !this.type.flies();
		if (this.latched) {
			float yaw = enemy.getYRot() * Mth.DEG_TO_RAD;
			Vec3 relative = this.position().subtract(enemy.position()).yRot(yaw);
			double halfWidth = enemy.getBbWidth() * 0.5;
			double height = Mth.clamp(relative.y, 0.2, enemy.getBbHeight() * 0.9);
			Vec3 flat = new Vec3(relative.x, 0.0, relative.z);
			flat = flat.lengthSqr() < 1.0E-4 ? new Vec3(halfWidth, 0.0, 0.0) : flat.normalize().scale(halfWidth + 0.1);
			this.latchOffset = new Vec3(flat.x, height, flat.z);
		}
	}

	/** Where a thrown or charging Pikmin lands decides what it does next. */
	private void resolveLanding(ServerLevel level) {
		LivingEntity enemy = this.nearestEnemy(level, 1.5);
		if (enemy != null) {
			this.startAttack(enemy, false);
			return;
		}
		if (this.findWork(level, 1.8)) {
			return;
		}
		this.setState(State.IDLE);
	}

	/** Rock Pikmin smash glass and ice; Purple Pikmin stun what they land on. */
	private void onImpact(ServerLevel level, @Nullable Entity hit) {
		if (this.type == PikminType.ROCK) {
			BlockPos center = this.blockPosition();
			for (BlockPos pos : BlockPos.betweenClosed(center.offset(-1, -1, -1), center.offset(1, 1, 1))) {
				BlockState block = level.getBlockState(pos);
				if (block.is(BlockTags.IMPERMEABLE) || block.is(BlockTags.ICE)) {
					level.destroyBlock(pos, true, this);
				}
			}
			if (hit instanceof LivingEntity living) {
				living.hurtServer(level, this.damageSources().mobAttack(this), this.type.attack * 2.0F);
			}
		} else if (this.type == PikminType.PURPLE) {
			level.sendParticles(ParticleTypes.EXPLOSION, this.getX(), this.getY(), this.getZ(), 1, 0.0, 0.0, 0.0, 0.0);
			for (LivingEntity living : level.getEntitiesOfClass(LivingEntity.class, this.getBoundingBox().inflate(2.0), this::isAttackable)) {
				living.hurtServer(level, this.damageSources().mobAttack(this), this.type.attack * 2.0F);
				living.addEffect(new MobEffectInstance(MobEffects.SLOWNESS, 60, 3), this);
			}
		}
	}

	// ------------------------------------------------------------------ saving

	@Override
	protected void addAdditionalSaveData(ValueOutput output) {
		super.addAdditionalSaveData(output);
		output.putInt("Maturity", this.maturity());
		State state = this.state();
		// mid-air or busy Pikmin come back idle
		output.putString("State", state == State.BURIED || state == State.FOLLOW ? state.name() : State.IDLE.name());
		output.putInt("BuriedTicks", this.buriedTicks);
		if (this.leader != null) {
			output.store("Leader", UUIDUtil.CODEC, this.leader);
		}
	}

	@Override
	protected void readAdditionalSaveData(ValueInput input) {
		super.readAdditionalSaveData(input);
		this.setMaturity(input.getIntOr("Maturity", LEAF));
		this.leader = input.read("Leader", UUIDUtil.CODEC).orElse(null);
		State state;
		try {
			state = State.valueOf(input.getStringOr("State", State.IDLE.name()));
		} catch (IllegalArgumentException e) {
			state = State.IDLE;
		}
		this.setState(state);
		this.buriedTicks = input.getIntOr("BuriedTicks", 0);
	}
}
