package io.github.malik05051.pikcraft.client.mixin;

import io.github.malik05051.pikcraft.client.CutsceneCamera;
import net.minecraft.client.Camera;
import net.minecraft.world.phys.Vec3;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Lets a cutscene take over the camera. */
@Mixin(Camera.class)
public abstract class CameraMixin {
	@Shadow
	private boolean detached;

	@Shadow
	protected abstract void setRotation(float yRot, float xRot);

	@Shadow
	protected abstract void setPosition(Vec3 position);

	@Inject(method = "alignWithEntity", at = @At("TAIL"))
	private void pikcraft$cutscene(float partialTicks, CallbackInfo ci) {
		CutsceneCamera.Shot shot = CutsceneCamera.shot(partialTicks);
		if (shot != null) {
			this.setRotation(shot.yRot(), shot.xRot());
			this.setPosition(shot.position());
			this.detached = true;
		}
	}
}
