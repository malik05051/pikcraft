package io.github.malik05051.pikcraft.network;

import io.github.malik05051.pikcraft.entity.OnionEntity;
import io.github.malik05051.pikcraft.squad.Squads;
import net.fabricmc.fabric.api.networking.v1.PayloadTypeRegistry;
import net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking;
import net.minecraft.server.level.ServerPlayer;

public final class ModNetworking {
	public static void init() {
		PayloadTypeRegistry.serverboundPlay().register(ThrowPayload.TYPE, ThrowPayload.CODEC);
		PayloadTypeRegistry.serverboundPlay().register(WhistlePayload.TYPE, WhistlePayload.CODEC);
		PayloadTypeRegistry.serverboundPlay().register(ChargePayload.TYPE, ChargePayload.CODEC);
		PayloadTypeRegistry.serverboundPlay().register(DismissPayload.TYPE, DismissPayload.CODEC);
		PayloadTypeRegistry.serverboundPlay().register(SelectTypePayload.TYPE, SelectTypePayload.CODEC);
		PayloadTypeRegistry.serverboundPlay().register(OnionActionPayload.TYPE, OnionActionPayload.CODEC);
		PayloadTypeRegistry.clientboundPlay().register(OpenOnionMenuPayload.TYPE, OpenOnionMenuPayload.CODEC);
		PayloadTypeRegistry.clientboundPlay().register(OnionCutscenePayload.TYPE, OnionCutscenePayload.CODEC);
		PayloadTypeRegistry.clientboundPlay().register(EnemyHealthPayload.TYPE, EnemyHealthPayload.CODEC);
		PayloadTypeRegistry.clientboundPlay().register(HudSyncPayload.TYPE, HudSyncPayload.CODEC);

		ServerPlayNetworking.registerGlobalReceiver(ThrowPayload.TYPE, (payload, context) -> Squads.throwPikmin(context.player()));
		ServerPlayNetworking.registerGlobalReceiver(WhistlePayload.TYPE, (payload, context) -> {
			if (Squads.hasWhistle(context.player())) {
				Squads.whistle(context.player(), Math.clamp(payload.radius(), 1.0F, WhistlePayload.MAX_RADIUS));
			}
		});
		ServerPlayNetworking.registerGlobalReceiver(ChargePayload.TYPE, (payload, context) -> Squads.charge(context.player()));
		ServerPlayNetworking.registerGlobalReceiver(DismissPayload.TYPE, (payload, context) -> Squads.dismiss(context.player()));
		ServerPlayNetworking.registerGlobalReceiver(SelectTypePayload.TYPE, (payload, context) -> Squads.selectNext(context.player(), payload.step()));
		ServerPlayNetworking.registerGlobalReceiver(OnionActionPayload.TYPE, (payload, context) -> {
			ServerPlayer player = context.player();
			if (player.level().getEntity(payload.onionId()) instanceof OnionEntity onion && onion.isAwake() && player.distanceToSqr(onion) < 12 * 12) {
				int amount = Math.clamp(payload.amount(), -100, 100);
				if (amount > 0) {
					onion.withdraw(player, amount);
				} else if (amount < 0) {
					onion.deposit(player, -amount);
				}
				onion.openMenu(player);
			}
		});
	}

	private ModNetworking() {
	}
}
