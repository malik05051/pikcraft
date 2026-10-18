package io.github.malik05051.pikcraft.client.model;

import io.github.malik05051.pikcraft.PikCraft;
import net.minecraft.client.model.EntityModel;
import net.minecraft.client.model.geom.ModelLayerLocation;
import net.minecraft.client.model.geom.ModelPart;
import net.minecraft.client.model.geom.PartPose;
import net.minecraft.client.model.geom.builders.CubeListBuilder;
import net.minecraft.client.model.geom.builders.LayerDefinition;
import net.minecraft.client.model.geom.builders.MeshDefinition;
import net.minecraft.client.model.geom.builders.PartDefinition;
import net.minecraft.client.renderer.entity.state.LivingEntityRenderState;
import net.minecraft.client.renderer.rendertype.RenderTypes;
import net.minecraft.util.Mth;

/** The Onion: a round bulb on three long legs, with a spinning flower on top. */
public class OnionModel extends EntityModel<OnionModel.State> {
	public static final ModelLayerLocation LAYER = new ModelLayerLocation(PikCraft.id("onion"), "main");
	private static final int LEGS = 3;
	private static final float BULB_Y = -30.0F;

	private final ModelPart bulb;
	private final ModelPart petals;
	private final ModelPart[] legs = new ModelPart[LEGS];

	public static class State extends LivingEntityRenderState {
		public int type;
		/** 0 = asleep on the ground, 1 = standing on its legs. */
		public float awake;
		public float lift;
		public boolean following;
	}

	public OnionModel(ModelPart root) {
		super(root.getChild("root"), RenderTypes::entityCutout);
		this.bulb = this.root.getChild("bulb");
		this.petals = this.bulb.getChild("stalk").getChild("petals");
		for (int i = 0; i < LEGS; i++) {
			this.legs[i] = this.root.getChild("leg" + i);
		}
	}

	public static LayerDefinition createLayer() {
		MeshDefinition mesh = new MeshDefinition();
		PartDefinition root = mesh.getRoot().addOrReplaceChild("root", CubeListBuilder.create(), PartPose.offset(0.0F, 24.0F, 0.0F));
		PartDefinition bulb = root.addOrReplaceChild("bulb", CubeListBuilder.create()
				.texOffs(0, 0).addBox(-12.0F, -14.0F, -12.0F, 24.0F, 14.0F, 24.0F)
				.texOffs(0, 40).addBox(-8.0F, 0.0F, -8.0F, 16.0F, 4.0F, 16.0F)
				.texOffs(0, 60).addBox(-8.0F, -18.0F, -8.0F, 16.0F, 4.0F, 16.0F),
			PartPose.offset(0.0F, BULB_Y, 0.0F));
		PartDefinition stalk = bulb.addOrReplaceChild("stalk", CubeListBuilder.create().texOffs(64, 40).addBox(-1.0F, -8.0F, -1.0F, 2.0F, 8.0F, 2.0F),
			PartPose.offset(0.0F, -18.0F, 0.0F));
		stalk.addOrReplaceChild("petals", CubeListBuilder.create().texOffs(80, 80).addBox(-8.0F, 0.0F, -8.0F, 16.0F, 0.0F, 16.0F),
			PartPose.offset(0.0F, -8.0F, 0.0F));
		for (int i = 0; i < LEGS; i++) {
			float angle = i * Mth.TWO_PI / LEGS;
			root.addOrReplaceChild("leg" + i, CubeListBuilder.create().texOffs(72, 40).addBox(-1.0F, 0.0F, -1.0F, 2.0F, 32.0F, 2.0F),
				PartPose.offsetAndRotation(Mth.sin(angle) * 8.0F, BULB_Y + 2.0F, Mth.cos(angle) * 8.0F, 0.0F, angle, 0.0F));
		}
		return LayerDefinition.create(mesh, 128, 128);
	}

	@Override
	public void setupAnim(State state) {
		super.setupAnim(state);
		float awake = state.awake;
		float age = state.ageInTicks;
		float flying = Mth.clamp(state.lift / 4.0F, 0.0F, 1.0F);
		// asleep it sits on the ground with its legs tucked in; awake it stands tall
		float sink = (1.0F - awake) * 26.0F;
		this.bulb.y += sink + Mth.sin(age * 0.05F) * 0.6F * awake;
		float spread = Mth.lerp(awake, 1.45F, 0.42F) + flying * 0.5F;
		for (int i = 0; i < LEGS; i++) {
			this.legs[i].y += sink;
			this.legs[i].xRot = spread;
		}
		float spin = awake * (0.03F + flying * 0.5F + (state.following ? 0.1F : 0.0F));
		this.petals.yRot = age * spin;
		this.petals.xRot = Mth.sin(age * 0.1F) * 0.05F * awake;
		this.petals.visible = awake > 0.05F;
	}
}
