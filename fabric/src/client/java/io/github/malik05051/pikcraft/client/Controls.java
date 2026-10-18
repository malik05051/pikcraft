package io.github.malik05051.pikcraft.client;

import com.mojang.blaze3d.platform.InputConstants;
import io.github.malik05051.pikcraft.PikCraft;
import io.github.malik05051.pikcraft.PikminType;
import io.github.malik05051.pikcraft.item.WhistleItem;
import io.github.malik05051.pikcraft.network.ChargePayload;
import io.github.malik05051.pikcraft.network.DismissPayload;
import io.github.malik05051.pikcraft.network.SelectTypePayload;
import io.github.malik05051.pikcraft.network.ThrowPayload;
import io.github.malik05051.pikcraft.network.WhistlePayload;
import io.github.malik05051.pikcraft.registry.ModItems;
import net.fabricmc.fabric.api.client.keymapping.v1.KeyMappingHelper;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayNetworking;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.client.player.LocalPlayer;
import net.minecraft.core.particles.DustParticleOptions;
import net.minecraft.util.Mth;
import net.minecraft.world.level.ClipContext;
import net.minecraft.world.phys.HitResult;
import net.minecraft.world.phys.Vec3;

/**
 * Pikmin controls. Keys: throw (R), whistle (hold V), charge (C), dismiss (X), next type (G).
 * With the whistle in hand, attack throws and use blows the whistle, like the games' A and B buttons.
 */
public final class Controls {
	private static final KeyMapping.Category CATEGORY = KeyMapping.Category.register(PikCraft.id("pikmin"));
	public static final KeyMapping THROW = key("throw", InputConstants.KEY_R);
	public static final KeyMapping WHISTLE = key("whistle", InputConstants.KEY_V);
	public static final KeyMapping CHARGE = key("charge", InputConstants.KEY_C);
	public static final KeyMapping DISMISS = key("dismiss", InputConstants.KEY_X);
	public static final KeyMapping NEXT_TYPE = key("next_type", InputConstants.KEY_G);

	private static int whistleTicks = -1;

	private static KeyMapping key(String name, int key) {
		return KeyMappingHelper.registerKeyMapping(new KeyMapping("key.pikcraft." + name, key, CATEGORY));
	}

	public static void init() {
		// class loading registers the keys
	}

	/** Runs at the start of each client tick, before vanilla reads the same clicks. */
	public static void tick(Minecraft minecraft) {
		LocalPlayer player = minecraft.player;
		if (player == null || minecraft.gui.screen() != null || CutsceneCamera.isPlaying()) {
			whistleTicks = -1;
			return;
		}
		boolean whistleInHand = player.getMainHandItem().is(ModItems.WHISTLE);
		while (THROW.consumeClick()) {
			ClientPlayNetworking.send(new ThrowPayload());
		}
		if (whistleInHand) {
			while (minecraft.options.keyAttack.consumeClick()) {
				ClientPlayNetworking.send(new ThrowPayload());
			}
		}
		while (CHARGE.consumeClick()) {
			ClientPlayNetworking.send(new ChargePayload());
		}
		while (DISMISS.consumeClick()) {
			ClientPlayNetworking.send(new DismissPayload());
		}
		while (NEXT_TYPE.consumeClick()) {
			ClientPlayNetworking.send(new SelectTypePayload(player.isShiftKeyDown() ? -1 : 1));
		}

		boolean hasWhistle = player.getInventory().contains(stack -> stack.is(ModItems.WHISTLE));
		if (WHISTLE.isDown() && hasWhistle) {
			whistleTicks++;
			if (whistleTicks % 4 == 0 && whistleTicks <= 40) {
				ClientPlayNetworking.send(new WhistlePayload(WhistleItem.radiusAfter(whistleTicks)));
			}
			showWhistleRing(minecraft, WhistleItem.radiusAfter(Math.min(whistleTicks, 40)));
		} else {
			whistleTicks = -1;
		}
		if (player.isUsingItem() && player.getUseItem().is(ModItems.WHISTLE)) {
			showWhistleRing(minecraft, WhistleItem.radiusAfter(player.getTicksUsingItem()));
		}
	}

	/** The whistle's circle on the ground where the player is looking. */
	private static void showWhistleRing(Minecraft minecraft, float radius) {
		LocalPlayer player = minecraft.player;
		Vec3 eye = player.getEyePosition();
		Vec3 end = eye.add(player.getLookAngle().scale(22.0));
		HitResult hit = minecraft.level.clip(new ClipContext(eye, end, ClipContext.Block.COLLIDER, ClipContext.Fluid.NONE, player));
		Vec3 center = hit.getType() == HitResult.Type.MISS
			? minecraft.level.clip(new ClipContext(end, end.add(0.0, -22.0, 0.0), ClipContext.Block.COLLIDER, ClipContext.Fluid.ANY, player)).getLocation()
			: hit.getLocation();
		int points = Mth.ceil(radius * 6.0F);
		float spin = player.tickCount * 0.15F;
		for (int i = 0; i < points; i++) {
			double angle = spin + i * Mth.TWO_PI / points;
			minecraft.level.addParticle(new DustParticleOptions(0xFFE14A, 0.9F), center.x + Math.cos(angle) * radius, center.y + 0.15,
				center.z + Math.sin(angle) * radius, 0.0, 0.0, 0.0);
		}
	}

	public static String typeName(PikminType type) {
		return "entity.pikcraft." + type.id() + "_pikmin";
	}

	private Controls() {
	}
}
