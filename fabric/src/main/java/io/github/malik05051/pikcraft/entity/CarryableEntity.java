package io.github.malik05051.pikcraft.entity;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.EnumMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;

import io.github.malik05051.pikcraft.PikminType;
import io.github.malik05051.pikcraft.registry.ModSounds;
import net.minecraft.network.syncher.EntityDataAccessor;
import net.minecraft.network.syncher.EntityDataSerializers;
import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.util.Mth;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.Mob;
import net.minecraft.world.entity.PathfinderMob;
import net.minecraft.world.entity.ai.attributes.AttributeSupplier;
import net.minecraft.world.entity.ai.attributes.Attributes;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.level.Level;
import net.minecraft.world.phys.Vec3;
import org.jspecify.annotations.Nullable;

/**
 * Something Pikmin carry back to an Onion: pellets and the bodies of defeated enemies.
 *
 * It only moves once its carriers are strong enough, and then walks itself to the best Onion with its
 * own path finding; the carriers hold on around it. A player can also pick it up and carry it.
 */
public abstract class CarryableEntity extends PathfinderMob {
	private static final EntityDataAccessor<Integer> DATA_CARRIERS = SynchedEntityData.defineId(CarryableEntity.class, EntityDataSerializers.INT);
	private static final EntityDataAccessor<Integer> DATA_STRENGTH = SynchedEntityData.defineId(CarryableEntity.class, EntityDataSerializers.INT);

	private final List<PikminEntity> carriers = new ArrayList<>();
	private @Nullable OnionEntity destination;
	private int chantCooldown;
	private int retargetCooldown;

	protected CarryableEntity(EntityType<? extends CarryableEntity> type, Level level) {
		super(type, level);
	}

	public static AttributeSupplier.Builder createAttributes() {
		return Mob.createMobAttributes().add(Attributes.MAX_HEALTH, 1000.0).add(Attributes.MOVEMENT_SPEED, 0.2).add(Attributes.KNOCKBACK_RESISTANCE, 1.0)
			.add(Attributes.STEP_HEIGHT, 1.0);
	}

	@Override
	protected void defineSynchedData(SynchedEntityData.Builder builder) {
		super.defineSynchedData(builder);
		builder.define(DATA_CARRIERS, 0);
		builder.define(DATA_STRENGTH, 0);
	}

	/** Pikmin strength needed to lift it. */
	public abstract int weight();

	/** Seeds an Onion of the given colour makes from it. */
	public abstract int seedsFor(PikminType onionType);

	public int maxCarriers() {
		return Math.max(2, this.weight() * 2);
	}

	public int carrierCount() {
		return this.entityData.get(DATA_CARRIERS);
	}

	public int liftStrength() {
		return this.entityData.get(DATA_STRENGTH);
	}

	public boolean isLifted() {
		return this.liftStrength() >= this.weight() || this.isPassenger();
	}

	public boolean wantsCarriers() {
		return this.carriers.size() < this.maxCarriers() && !this.isPassenger();
	}

	public boolean hasCarrier(PikminEntity pikmin) {
		return this.carriers.contains(pikmin);
	}

	public boolean addCarrier(PikminEntity pikmin) {
		if (!this.wantsCarriers() || this.carriers.contains(pikmin)) {
			return false;
		}
		this.carriers.add(pikmin);
		this.updateCarriers();
		return true;
	}

	public void removeCarrier(PikminEntity pikmin) {
		if (this.carriers.remove(pikmin)) {
			this.updateCarriers();
		}
	}

	private void updateCarriers() {
		this.carriers.removeIf(p -> !p.isAlive() || p.state() != PikminEntity.State.CARRY || p.target() != this);
		this.entityData.set(DATA_CARRIERS, this.carriers.size());
		this.entityData.set(DATA_STRENGTH, this.carriers.stream().mapToInt(p -> p.pikminType().strength).sum());
	}

	/** Where a carrier stands: evenly spread around the object. */
	public Vec3 carrierSpot(PikminEntity pikmin) {
		int index = Math.max(0, this.carriers.indexOf(pikmin));
		double angle = index * Mth.TWO_PI / Math.max(1, this.carriers.size());
		double radius = this.getBbWidth() * 0.5 + 0.35;
		return this.position().add(Math.cos(angle) * radius, 0.0, Math.sin(angle) * radius);
	}

	public void dropAll() {
		for (PikminEntity carrier : List.copyOf(this.carriers)) {
			carrier.setState(PikminEntity.State.IDLE);
		}
		this.carriers.clear();
		this.updateCarriers();
	}

	// ------------------------------------------------------------------ player carrying

	@Override
	protected InteractionResult mobInteract(Player player, InteractionHand hand) {
		if (hand == InteractionHand.MAIN_HAND && player.getMainHandItem().isEmpty() && !player.isVehicle() && !this.isPassenger()) {
			if (!this.level().isClientSide()) {
				this.dropAll();
				this.startRiding(player, true, true);
			}
			return InteractionResult.SUCCESS;
		}
		return super.mobInteract(player, hand);
	}

	// ------------------------------------------------------------------ walking home

	@Override
	protected void customServerAiStep(ServerLevel level) {
		super.customServerAiStep(level);
		if (this.tickCount % 10 == 0) {
			this.updateCarriers();
		}
		if (this.isPassenger()) {
			this.tryDeliverNearby(level);
			return;
		}
		if (!this.isLifted()) {
			this.getNavigation().stop();
			return;
		}
		if (--this.retargetCooldown <= 0 || this.destination == null || !this.destination.isAlive()) {
			this.retargetCooldown = 40;
			this.destination = this.chooseOnion(level);
		}
		if (this.destination == null) {
			this.getNavigation().stop();
			return;
		}
		if (this.tryDeliverNearby(level)) {
			return;
		}
		double surplus = (double) this.liftStrength() / Math.max(1, this.weight());
		double speed = Mth.clamp(0.55 + 0.25 * (surplus - 1.0), 0.55, 1.1);
		if (!this.destination.acceptsDeliveries()) {
			// the Onion is up in the sky for the night: wait under it
			if (this.distanceToSqr(this.destination.groundPosition()) > 9.0) {
				this.getNavigation().moveTo(this.destination.groundPosition().x, this.destination.groundPosition().y, this.destination.groundPosition().z, speed);
			} else {
				this.getNavigation().stop();
			}
			return;
		}
		this.getNavigation().moveTo(this.destination.getX(), this.destination.getY(), this.destination.getZ(), speed);
		if (--this.chantCooldown <= 0) {
			this.chantCooldown = 30;
			this.playSound(ModSounds.CARRY, 0.6F, 0.9F + this.random.nextFloat() * 0.2F);
		}
	}

	private boolean tryDeliverNearby(ServerLevel level) {
		for (OnionEntity onion : level.getEntitiesOfClass(OnionEntity.class, this.getBoundingBox().inflate(2.0), OnionEntity::acceptsDeliveries)) {
			onion.deliver(this);
			return true;
		}
		return false;
	}

	/** The Onion of the carriers' main colour, or the nearest awake Onion. */
	private @Nullable OnionEntity chooseOnion(ServerLevel level) {
		List<OnionEntity> onions = level.getEntitiesOfClass(OnionEntity.class, this.getBoundingBox().inflate(160.0), OnionEntity::isAwake);
		if (onions.isEmpty()) {
			return null;
		}
		Map<PikminType, Integer> colours = new EnumMap<>(PikminType.class);
		for (PikminEntity carrier : this.carriers) {
			colours.merge(carrier.pikminType(), carrier.pikminType().strength, Integer::sum);
		}
		Comparator<OnionEntity> nearest = Comparator.comparingDouble(this::distanceToSqr);
		Optional<PikminType> main = colours.entrySet().stream().max(Map.Entry.comparingByValue()).map(Map.Entry::getKey);
		if (main.isPresent()) {
			Optional<OnionEntity> matching = onions.stream().filter(o -> o.onionType() == main.get()).min(nearest);
			if (matching.isPresent()) {
				return matching.get();
			}
		}
		return onions.stream().min(nearest).orElse(null);
	}

	/** The carriers whose Onion this is go back to idling when it is taken in. */
	public void onDelivered() {
		for (PikminEntity carrier : List.copyOf(this.carriers)) {
			carrier.setState(PikminEntity.State.IDLE);
		}
		this.carriers.clear();
	}

	// ------------------------------------------------------------------ an object, not a creature

	@Override
	public boolean isInvulnerableTo(ServerLevel level, DamageSource source) {
		return !source.is(net.minecraft.world.damagesource.DamageTypes.GENERIC_KILL) && !source.is(net.minecraft.world.damagesource.DamageTypes.FELL_OUT_OF_WORLD);
	}

	@Override
	public boolean isPushable() {
		return false;
	}

	@Override
	public boolean causeFallDamage(double fallDistance, float damageModifier, DamageSource damageSource) {
		return false;
	}

	@Override
	public boolean removeWhenFarAway(double distanceToClosestPlayer) {
		return false;
	}

	@Override
	public boolean canBeLeashed() {
		return false;
	}
}
