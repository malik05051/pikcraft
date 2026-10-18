package io.github.malik05051.pikcraft.client;

import io.github.malik05051.pikcraft.entity.OnionEntity;
import io.github.malik05051.pikcraft.network.OnionCutscenePayload;
import net.minecraft.client.Minecraft;
import net.minecraft.util.Mth;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.phys.Vec3;
import org.jspecify.annotations.Nullable;

/**
 * Short third-person camera scenes: finding the first sleeping Onion (the camera swings out from
 * behind the player to show them standing before it) and an Onion waking up.
 */
public final class CutsceneCamera {
	private static int onionId = -1;
	private static int kind;
	private static long start;
	private static boolean hidGui;

	public record Shot(Vec3 position, float yRot, float xRot) {
	}

	public static void play(int onion, int sceneKind) {
		Minecraft minecraft = Minecraft.getInstance();
		if (minecraft.level == null) {
			return;
		}
		onionId = onion;
		kind = sceneKind;
		start = minecraft.level.getGameTime();
		if (!minecraft.gui.hud.isHidden()) {
			minecraft.gui.hud.toggle();
			hidGui = true;
		}
	}

	public static boolean isPlaying() {
		return onionId >= 0;
	}

	private static int length() {
		return kind == OnionCutscenePayload.DISCOVER ? 120 : 70;
	}

	public static void stop() {
		onionId = -1;
		if (hidGui) {
			if (Minecraft.getInstance().gui.hud.isHidden()) {
				Minecraft.getInstance().gui.hud.toggle();
			}
			hidGui = false;
		}
	}

	/** The camera for this frame, or null when no scene is playing. */
	public static @Nullable Shot shot(float partialTicks) {
		Minecraft minecraft = Minecraft.getInstance();
		if (onionId < 0 || minecraft.level == null || minecraft.player == null) {
			return null;
		}
		Entity entity = minecraft.level.getEntity(onionId);
		float time = minecraft.level.getGameTime() - start + partialTicks;
		if (!(entity instanceof OnionEntity onion) || time > length()) {
			stop();
			return null;
		}
		Vec3 player = minecraft.player.getPosition(partialTicks);
		Vec3 target = onion.getPosition(partialTicks).add(0.0, 1.6, 0.0);
		Vec3 flat = new Vec3(target.x - player.x, 0.0, target.z - player.z);
		Vec3 forward = flat.lengthSqr() < 1.0E-4 ? new Vec3(0.0, 0.0, 1.0) : flat.normalize();
		Vec3 side = new Vec3(-forward.z, 0.0, forward.x);
		Vec3 camera;
		Vec3 lookAt;
		if (kind == OnionCutscenePayload.DISCOVER) {
			// from over the player's shoulder, swing round to the side to show the explorer and the Onion
			float t = Mth.clamp(time / 90.0F, 0.0F, 1.0F);
			float ease = t * t * (3.0F - 2.0F * t);
			double angle = Mth.lerp(ease, 0.0F, 1.25F);
			double distance = Mth.lerp(ease, 3.5F, 7.0F);
			Vec3 back = forward.scale(-Math.cos(angle)).add(side.scale(Math.sin(angle)));
			camera = player.add(0.0, 1.9 + ease * 1.2, 0.0).add(back.scale(distance));
			lookAt = player.add(0.0, 1.2, 0.0).lerp(target, 0.35 + 0.25 * ease);
		} else {
			// low and wide on the Onion as it stands up
			float t = Mth.clamp(time / length(), 0.0F, 1.0F);
			Vec3 away = player.subtract(target).multiply(1.0, 0.0, 1.0);
			away = away.lengthSqr() < 1.0E-4 ? forward.scale(-1.0) : away.normalize();
			// from the side, so the player knocked back is in the shot too
			Vec3 orbit = away.yRot(Mth.lerp(t, 1.25F, 0.85F));
			camera = target.add(orbit.scale(9.0)).add(0.0, -0.4 + t * 1.6, 0.0);
			lookAt = target.add(0.0, 0.4 + t * 0.8, 0.0);
		}
		Vec3 dir = lookAt.subtract(camera);
		float yRot = (float) (Mth.atan2(dir.z, dir.x) * Mth.RAD_TO_DEG) - 90.0F;
		float xRot = (float) (-Mth.atan2(dir.y, dir.horizontalDistance()) * Mth.RAD_TO_DEG);
		return new Shot(camera, yRot, xRot);
	}

	private CutsceneCamera() {
	}
}
