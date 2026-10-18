package io.github.malik05051.pikcraft.entity;

import io.github.malik05051.pikcraft.registry.ModEntities;
import io.github.malik05051.pikcraft.registry.ModSounds;
import net.minecraft.core.particles.ParticleTypes;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.Mob;
import net.minecraft.world.entity.PathfinderMob;
import net.minecraft.world.entity.ai.attributes.AttributeSupplier;
import net.minecraft.world.entity.ai.attributes.Attributes;
import net.minecraft.world.level.Level;

/** A spotted egg lying in the grass. Break it (or let the Pikmin break it) to spill nectar. */
public class NectarEggEntity extends PathfinderMob {
	public NectarEggEntity(EntityType<? extends NectarEggEntity> type, Level level) {
		super(type, level);
	}

	public static AttributeSupplier.Builder createAttributes() {
		return Mob.createMobAttributes().add(Attributes.MAX_HEALTH, 6.0).add(Attributes.MOVEMENT_SPEED, 0.0).add(Attributes.KNOCKBACK_RESISTANCE, 1.0);
	}

	@Override
	public void die(DamageSource source) {
		super.die(source);
		if (this.level() instanceof ServerLevel level) {
			this.playSound(ModSounds.EGG_BREAK, 1.0F, 1.0F);
			level.sendParticles(ParticleTypes.ITEM_SLIME, this.getX(), this.getY() + 0.4, this.getZ(), 20, 0.3, 0.3, 0.3, 0.1);
			int puddles = 1 + this.random.nextInt(2);
			for (int i = 0; i < puddles; i++) {
				NectarEntity nectar = ModEntities.NECTAR.create(level, EntitySpawnReason.TRIGGERED);
				if (nectar != null) {
					nectar.snapTo(this.getX() + (this.random.nextDouble() - 0.5), this.getY() + 0.2, this.getZ() + (this.random.nextDouble() - 0.5), 0.0F, 0.0F);
					level.addFreshEntity(nectar);
				}
			}
			this.discard();
		}
	}

	@Override
	public boolean isPushable() {
		return false;
	}

	@Override
	protected boolean shouldDropLoot(ServerLevel level) {
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
}
