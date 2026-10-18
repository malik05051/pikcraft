package io.github.malik05051.pikcraft.client.render;

import io.github.malik05051.pikcraft.PikCraft;
import io.github.malik05051.pikcraft.PikminType;
import io.github.malik05051.pikcraft.client.model.OnionModel;
import io.github.malik05051.pikcraft.entity.OnionEntity;
import net.minecraft.client.renderer.entity.EntityRendererProvider;
import net.minecraft.client.renderer.entity.MobRenderer;
import net.minecraft.core.BlockPos;
import net.minecraft.resources.Identifier;
import net.minecraft.util.Mth;

public class OnionRenderer extends MobRenderer<OnionEntity, OnionModel.State, OnionModel> {
	private static final Identifier[] TEXTURES = new Identifier[PikminType.values().length];

	static {
		for (PikminType type : PikminType.values()) {
			TEXTURES[type.ordinal()] = PikCraft.id("textures/entity/onion/" + type.id() + ".png");
		}
	}

	public OnionRenderer(EntityRendererProvider.Context context) {
		super(context, new OnionModel(context.bakeLayer(OnionModel.LAYER)), 1.4F);
	}

	@Override
	public Identifier getTextureLocation(OnionModel.State state) {
		return TEXTURES[state.type];
	}

	@Override
	public OnionModel.State createRenderState() {
		return new OnionModel.State();
	}

	@Override
	public void extractRenderState(OnionEntity entity, OnionModel.State state, float partialTicks) {
		super.extractRenderState(entity, state, partialTicks);
		state.type = entity.onionType().ordinal();
		state.awake = entity.awakeProgress(partialTicks);
		state.awake = state.awake * state.awake * (3.0F - 2.0F * state.awake);
		state.lift = entity.lift();
		state.following = entity.isFollowing();
		state.shadowRadius = 1.4F * (1.0F - Mth.clamp(state.lift / 12.0F, 0.0F, 1.0F));
	}

	@Override
	protected int getModelTint(OnionModel.State state) {
		// sleeping Onions are dull; they light up as they wake
		int shade = (int) Mth.lerp(state.awake, 150.0F, 255.0F);
		return 0xFF000000 | shade << 16 | shade << 8 | shade;
	}

	@Override
	protected int getBlockLightLevel(OnionEntity entity, BlockPos pos) {
		return entity.isDormant() ? super.getBlockLightLevel(entity, pos) : Math.max(9, super.getBlockLightLevel(entity, pos));
	}
}
