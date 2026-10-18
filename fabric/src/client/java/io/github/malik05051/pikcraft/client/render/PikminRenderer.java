package io.github.malik05051.pikcraft.client.render;

import com.mojang.blaze3d.vertex.PoseStack;
import com.mojang.math.Axis;
import io.github.malik05051.pikcraft.PikCraft;
import io.github.malik05051.pikcraft.PikminType;
import io.github.malik05051.pikcraft.client.model.PikminModel;
import io.github.malik05051.pikcraft.client.model.PikminRenderState;
import io.github.malik05051.pikcraft.entity.PikminEntity;
import net.minecraft.client.renderer.entity.EntityRendererProvider;
import net.minecraft.client.renderer.entity.MobRenderer;
import net.minecraft.resources.Identifier;

public class PikminRenderer extends MobRenderer<PikminEntity, PikminRenderState, PikminModel> {
	private static final Identifier[] TEXTURES = new Identifier[PikminType.values().length];

	static {
		for (PikminType type : PikminType.values()) {
			TEXTURES[type.ordinal()] = PikCraft.id("textures/entity/pikmin/" + type.id() + ".png");
		}
	}

	public PikminRenderer(EntityRendererProvider.Context context) {
		super(context, new PikminModel(context.bakeLayer(PikminModel.LAYER)), 0.22F);
	}

	@Override
	public Identifier getTextureLocation(PikminRenderState state) {
		return TEXTURES[state.type.ordinal()];
	}

	@Override
	public PikminRenderState createRenderState() {
		return new PikminRenderState();
	}

	@Override
	public void extractRenderState(PikminEntity entity, PikminRenderState state, float partialTicks) {
		super.extractRenderState(entity, state, partialTicks);
		state.type = entity.pikminType();
		state.maturity = entity.maturity();
		state.state = entity.state();
		state.drowning = entity.isDrowning();
		state.sinceAction = entity.tickCount - entity.actionTick() + partialTicks;
		state.phase = (entity.getId() * 37) % 100;
		if (state.state == PikminEntity.State.BURIED) {
			state.shadowRadius = 0.0F;
		}
	}

	@Override
	protected void setupRotations(PikminRenderState state, PoseStack poseStack, float bodyRot, float entityScale) {
		super.setupRotations(state, poseStack, bodyRot, entityScale);
		if (state.state == PikminEntity.State.BURIED) {
			poseStack.translate(0.0F, -0.875F, 0.0F); // head in the ground, stem in the air
		} else if (state.state == PikminEntity.State.THROWN) {
			poseStack.translate(0.0F, 0.45F, 0.0F);
			poseStack.rotateDegrees(Axis.XP, state.ageInTicks * 38.0F);
			poseStack.translate(0.0F, -0.45F, 0.0F);
		}
	}

	@Override
	protected void scale(PikminRenderState state, PoseStack poseStack) {
		float scale = state.type == PikminType.PURPLE ? 1.3F : state.type == PikminType.WHITE ? 0.9F : 1.0F;
		poseStack.scale(scale, scale, scale);
	}
}
