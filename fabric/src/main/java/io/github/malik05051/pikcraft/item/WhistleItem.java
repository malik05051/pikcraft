package io.github.malik05051.pikcraft.item;

import io.github.malik05051.pikcraft.network.WhistlePayload;
import io.github.malik05051.pikcraft.squad.Squads;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.ItemUseAnimation;
import net.minecraft.world.level.Level;

/** Hold use to blow: the circle grows the longer you blow, and every Pikmin inside comes running. */
public class WhistleItem extends Item {
	private static final int USE_TICKS = 40;

	public WhistleItem(Properties properties) {
		super(properties);
	}

	public static float radiusAfter(int ticksUsed) {
		return Math.min(WhistlePayload.MAX_RADIUS, 1.0F + ticksUsed * 0.25F);
	}

	@Override
	public InteractionResult use(Level level, Player player, InteractionHand hand) {
		player.startUsingItem(hand);
		return InteractionResult.CONSUME;
	}

	@Override
	public void onUseTick(Level level, LivingEntity entity, ItemStack stack, int ticksRemaining) {
		int used = this.getUseDuration(stack, entity) - ticksRemaining;
		if (entity instanceof ServerPlayer player && used % 4 == 0) {
			Squads.whistle(player, radiusAfter(used));
		}
	}

	@Override
	public int getUseDuration(ItemStack stack, LivingEntity entity) {
		return USE_TICKS;
	}

	@Override
	public ItemUseAnimation getUseAnimation(ItemStack stack) {
		return ItemUseAnimation.TOOT_HORN;
	}
}
