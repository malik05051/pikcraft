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
import net.minecraft.client.renderer.entity.state.EntityRenderState;
import net.minecraft.client.renderer.entity.state.LivingEntityRenderState;
import net.minecraft.client.renderer.rendertype.RenderTypes;
import net.minecraft.util.Mth;

/** The models of the small things: seeds, pellets, pellet posies, nectar eggs and nectar. */
public final class SmallModels {
	public static final ModelLayerLocation SEED = new ModelLayerLocation(PikCraft.id("seed"), "main");
	public static final ModelLayerLocation PELLET = new ModelLayerLocation(PikCraft.id("pellet"), "main");
	public static final ModelLayerLocation POSY = new ModelLayerLocation(PikCraft.id("pellet_posy"), "main");
	public static final ModelLayerLocation POSY_PELLET = new ModelLayerLocation(PikCraft.id("pellet_posy"), "pellet");
	public static final ModelLayerLocation NECTAR_EGG = new ModelLayerLocation(PikCraft.id("nectar_egg"), "main");
	public static final ModelLayerLocation NECTAR = new ModelLayerLocation(PikCraft.id("nectar"), "main");

	private static MeshDefinition mesh() {
		return new MeshDefinition();
	}

	private static PartDefinition root(MeshDefinition mesh, CubeListBuilder cubes) {
		return mesh.getRoot().addOrReplaceChild("root", cubes, PartPose.offset(0.0F, 24.0F, 0.0F));
	}

	public static LayerDefinition seed() {
		MeshDefinition mesh = mesh();
		PartDefinition root = root(mesh, CubeListBuilder.create().texOffs(0, 0).addBox(-1.0F, -3.0F, -1.0F, 2.0F, 3.0F, 2.0F));
		root.addOrReplaceChild("leaf", CubeListBuilder.create().texOffs(8, 0).addBox(0.0F, -3.0F, -1.5F, 0.0F, 3.0F, 3.0F), PartPose.offset(0.0F, -3.0F, 0.0F));
		return LayerDefinition.create(mesh, 16, 16);
	}

	public static LayerDefinition pellet() {
		MeshDefinition mesh = mesh();
		root(mesh, CubeListBuilder.create().texOffs(0, 0).addBox(-4.0F, -3.0F, -4.0F, 8.0F, 3.0F, 8.0F));
		return LayerDefinition.create(mesh, 32, 32);
	}

	public static LayerDefinition posy() {
		MeshDefinition mesh = mesh();
		PartDefinition root = root(mesh, CubeListBuilder.create().texOffs(0, 0).addBox(-1.0F, -18.0F, -1.0F, 2.0F, 18.0F, 2.0F));
		root.addOrReplaceChild("leaf_left", CubeListBuilder.create().texOffs(8, 0).addBox(0.0F, -1.0F, -2.0F, 7.0F, 0.0F, 4.0F),
			PartPose.offsetAndRotation(1.0F, -6.0F, 0.0F, 0.0F, 0.0F, -0.5F));
		root.addOrReplaceChild("leaf_right", CubeListBuilder.create().texOffs(8, 4).addBox(-7.0F, -1.0F, -2.0F, 7.0F, 0.0F, 4.0F),
			PartPose.offsetAndRotation(-1.0F, -9.0F, 0.0F, 0.0F, 0.0F, 0.5F));
		PartDefinition head = root.addOrReplaceChild("head", CubeListBuilder.create().texOffs(0, 32).addBox(-7.0F, -10.0F, 0.0F, 14.0F, 14.0F, 0.0F),
			PartPose.offset(0.0F, -18.0F, 0.0F));
		head.addOrReplaceChild("side", CubeListBuilder.create().texOffs(28, 32).addBox(0.0F, -10.0F, -7.0F, 0.0F, 14.0F, 14.0F), PartPose.ZERO);
		return LayerDefinition.create(mesh, 64, 64);
	}

	/** The pellet held in a posy's flower, drawn separately so it takes the pellet's colour. */
	public static LayerDefinition posyPellet() {
		MeshDefinition mesh = mesh();
		PartDefinition root = root(mesh, CubeListBuilder.create());
		root.addOrReplaceChild("head", CubeListBuilder.create().texOffs(0, 20).addBox(-4.0F, -7.0F, -1.5F, 8.0F, 8.0F, 3.0F), PartPose.offset(0.0F, -18.0F, 0.0F));
		return LayerDefinition.create(mesh, 32, 32);
	}

	public static LayerDefinition nectarEgg() {
		MeshDefinition mesh = mesh();
		root(mesh, CubeListBuilder.create()
			.texOffs(0, 0).addBox(-4.0F, -11.0F, -4.0F, 8.0F, 10.0F, 8.0F)
			.texOffs(0, 18).addBox(-3.0F, -13.0F, -3.0F, 6.0F, 2.0F, 6.0F)
			.texOffs(24, 18).addBox(-3.0F, -1.0F, -3.0F, 6.0F, 1.0F, 6.0F));
		return LayerDefinition.create(mesh, 64, 32);
	}

	public static LayerDefinition nectar() {
		MeshDefinition mesh = mesh();
		root(mesh, CubeListBuilder.create().texOffs(0, 0).addBox(-6.0F, -1.0F, -6.0F, 12.0F, 1.0F, 12.0F)
			.texOffs(0, 13).addBox(-3.0F, -1.5F, -3.0F, 6.0F, 1.0F, 6.0F));
		return LayerDefinition.create(mesh, 64, 32);
	}

	/** A model that only spins or bobs as a whole. */
	public static class Simple<S extends EntityRenderState> extends EntityModel<S> {
		public Simple(ModelPart root) {
			super(root.getChild("root"), RenderTypes::entityCutout);
		}
	}

	public static class PosyState extends LivingEntityRenderState {
		public int value;
		public int colour;
		public float sinceHit = 100.0F;
	}

	public static class PosyModel extends EntityModel<PosyState> {
		private final ModelPart head;

		public PosyModel(ModelPart root) {
			super(root.getChild("root"), RenderTypes::entityCutout);
			this.head = this.root.getChild("head");
		}

		@Override
		public void setupAnim(PosyState state) {
			super.setupAnim(state);
			float shake = Math.max(0.0F, 1.0F - state.sinceHit / 12.0F);
			this.head.zRot = Mth.sin(state.sinceHit * 2.2F) * 0.4F * shake + Mth.sin(state.ageInTicks * 0.04F) * 0.05F;
			this.head.yRot = Mth.sin(state.ageInTicks * 0.03F) * 0.25F;
			this.root.zRot = Mth.sin(state.ageInTicks * 0.05F) * 0.03F + Mth.sin(state.sinceHit * 1.6F) * 0.12F * shake;
		}
	}

	private SmallModels() {
	}
}
