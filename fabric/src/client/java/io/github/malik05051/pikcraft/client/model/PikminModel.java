package io.github.malik05051.pikcraft.client.model;

import io.github.malik05051.pikcraft.PikCraft;
import io.github.malik05051.pikcraft.PikminType;
import io.github.malik05051.pikcraft.entity.PikminEntity;
import net.minecraft.client.model.EntityModel;
import net.minecraft.client.model.geom.ModelLayerLocation;
import net.minecraft.client.model.geom.ModelPart;
import net.minecraft.client.model.geom.PartPose;
import net.minecraft.client.model.geom.builders.CubeListBuilder;
import net.minecraft.client.model.geom.builders.LayerDefinition;
import net.minecraft.client.model.geom.builders.MeshDefinition;
import net.minecraft.client.model.geom.builders.PartDefinition;
import net.minecraft.client.renderer.rendertype.RenderTypes;
import net.minecraft.util.Mth;

/**
 * A blocky Pikmin: big head, little body, a stem with a leaf, bud or flower, plus the parts only some
 * types have (Red's nose, Yellow's ears, Purple's hair, Winged's wings). Texture layout: tools/make_assets.py.
 */
public class PikminModel extends EntityModel<PikminRenderState> {
	public static final ModelLayerLocation LAYER = new ModelLayerLocation(PikCraft.id("pikmin"), "main");

	private final ModelPart body;
	private final ModelPart head;
	private final ModelPart nose;
	private final ModelPart leftEar;
	private final ModelPart rightEar;
	private final ModelPart hair;
	private final ModelPart stem;
	private final ModelPart leafX;
	private final ModelPart leafZ;
	private final ModelPart bud;
	private final ModelPart flowerX;
	private final ModelPart flowerZ;
	private final ModelPart leftArm;
	private final ModelPart rightArm;
	private final ModelPart leftLeg;
	private final ModelPart rightLeg;
	private final ModelPart leftWing;
	private final ModelPart rightWing;

	public PikminModel(ModelPart root) {
		super(root.getChild("root"), RenderTypes::entityCutout);
		this.body = this.root.getChild("body");
		this.head = this.root.getChild("head");
		this.nose = this.head.getChild("nose");
		this.leftEar = this.head.getChild("left_ear");
		this.rightEar = this.head.getChild("right_ear");
		this.hair = this.head.getChild("hair");
		this.stem = this.head.getChild("stem");
		this.leafX = this.stem.getChild("leaf_x");
		this.leafZ = this.stem.getChild("leaf_z");
		this.bud = this.stem.getChild("bud");
		this.flowerX = this.stem.getChild("flower_x");
		this.flowerZ = this.stem.getChild("flower_z");
		this.leftArm = this.body.getChild("left_arm");
		this.rightArm = this.body.getChild("right_arm");
		this.leftWing = this.body.getChild("left_wing");
		this.rightWing = this.body.getChild("right_wing");
		this.leftLeg = this.root.getChild("left_leg");
		this.rightLeg = this.root.getChild("right_leg");
	}

	public static LayerDefinition createLayer() {
		MeshDefinition mesh = new MeshDefinition();
		PartDefinition root = mesh.getRoot().addOrReplaceChild("root", CubeListBuilder.create(), PartPose.offset(0.0F, 24.0F, 0.0F));
		PartDefinition body = root.addOrReplaceChild("body", CubeListBuilder.create().texOffs(0, 12).addBox(-1.5F, 0.0F, -1.0F, 3.0F, 5.0F, 2.0F),
			PartPose.offset(0.0F, -8.0F, 0.0F));
		body.addOrReplaceChild("left_arm", CubeListBuilder.create().texOffs(24, 0).addBox(0.0F, 0.0F, -0.5F, 1.0F, 4.0F, 1.0F),
			PartPose.offset(1.5F, 0.5F, 0.0F));
		body.addOrReplaceChild("right_arm", CubeListBuilder.create().texOffs(28, 0).addBox(-1.0F, 0.0F, -0.5F, 1.0F, 4.0F, 1.0F),
			PartPose.offset(-1.5F, 0.5F, 0.0F));
		body.addOrReplaceChild("left_wing", CubeListBuilder.create().texOffs(0, 24).addBox(0.0F, -1.0F, 0.0F, 0.0F, 4.0F, 5.0F),
			PartPose.offset(0.5F, 0.5F, 1.0F));
		body.addOrReplaceChild("right_wing", CubeListBuilder.create().texOffs(12, 24).addBox(0.0F, -1.0F, 0.0F, 0.0F, 4.0F, 5.0F),
			PartPose.offset(-0.5F, 0.5F, 1.0F));
		root.addOrReplaceChild("left_leg", CubeListBuilder.create().texOffs(24, 6).addBox(-0.5F, 0.0F, -0.5F, 1.0F, 3.0F, 1.0F),
			PartPose.offset(0.8F, -3.0F, 0.0F));
		root.addOrReplaceChild("right_leg", CubeListBuilder.create().texOffs(28, 6).addBox(-0.5F, 0.0F, -0.5F, 1.0F, 3.0F, 1.0F),
			PartPose.offset(-0.8F, -3.0F, 0.0F));
		PartDefinition head = root.addOrReplaceChild("head", CubeListBuilder.create().texOffs(0, 0).addBox(-3.0F, -6.0F, -3.0F, 6.0F, 6.0F, 6.0F),
			PartPose.offset(0.0F, -8.0F, 0.0F));
		head.addOrReplaceChild("nose", CubeListBuilder.create().texOffs(32, 0).addBox(-0.5F, -3.0F, -5.0F, 1.0F, 1.0F, 2.0F), PartPose.ZERO);
		head.addOrReplaceChild("left_ear", CubeListBuilder.create().texOffs(24, 12).addBox(3.0F, -5.0F, -1.0F, 1.0F, 3.0F, 2.0F), PartPose.ZERO);
		head.addOrReplaceChild("right_ear", CubeListBuilder.create().texOffs(30, 12).addBox(-4.0F, -5.0F, -1.0F, 1.0F, 3.0F, 2.0F), PartPose.ZERO);
		head.addOrReplaceChild("hair", CubeListBuilder.create().texOffs(24, 20).addBox(-2.0F, -8.0F, 0.0F, 4.0F, 2.0F, 0.0F), PartPose.ZERO);
		PartDefinition stem = head.addOrReplaceChild("stem", CubeListBuilder.create().texOffs(32, 4).addBox(-0.5F, -6.0F, -0.5F, 1.0F, 6.0F, 1.0F),
			PartPose.offset(0.0F, -6.0F, 0.0F));
		stem.addOrReplaceChild("leaf_x", CubeListBuilder.create().texOffs(40, 0).addBox(0.0F, -4.0F, -2.0F, 0.0F, 4.0F, 4.0F), PartPose.offset(0.0F, -6.0F, 0.0F));
		stem.addOrReplaceChild("leaf_z", CubeListBuilder.create().texOffs(48, 0).addBox(-2.0F, -4.0F, 0.0F, 4.0F, 4.0F, 0.0F), PartPose.offset(0.0F, -6.0F, 0.0F));
		stem.addOrReplaceChild("bud", CubeListBuilder.create().texOffs(48, 4).addBox(-1.0F, -2.0F, -1.0F, 2.0F, 2.0F, 2.0F), PartPose.offset(0.0F, -6.0F, 0.0F));
		stem.addOrReplaceChild("flower_x", CubeListBuilder.create().texOffs(40, 10).addBox(0.0F, -5.0F, -2.5F, 0.0F, 5.0F, 5.0F), PartPose.offset(0.0F, -6.0F, 0.0F));
		stem.addOrReplaceChild("flower_z", CubeListBuilder.create().texOffs(50, 10).addBox(-2.5F, -5.0F, 0.0F, 5.0F, 5.0F, 0.0F), PartPose.offset(0.0F, -6.0F, 0.0F));
		return LayerDefinition.create(mesh, 64, 64);
	}

	@Override
	public void setupAnim(PikminRenderState state) {
		super.setupAnim(state);
		PikminType type = state.type;
		this.nose.visible = type == PikminType.RED;
		this.leftEar.visible = this.rightEar.visible = type == PikminType.YELLOW;
		this.hair.visible = type == PikminType.PURPLE;
		this.leftWing.visible = this.rightWing.visible = type == PikminType.WINGED;
		this.stem.visible = type.grows();
		this.leafX.visible = this.leafZ.visible = state.maturity == PikminEntity.LEAF;
		this.bud.visible = state.maturity == PikminEntity.BUD;
		this.flowerX.visible = this.flowerZ.visible = state.maturity == PikminEntity.FLOWER;

		float age = state.ageInTicks + state.phase;
		float walkPos = state.walkAnimationPos * 1.6F;
		float walkSpeed = Math.min(state.walkAnimationSpeed, 1.0F);

		// look around, walk, sway
		this.head.yRot = state.yRot * Mth.DEG_TO_RAD * 0.6F;
		this.head.xRot = state.xRot * Mth.DEG_TO_RAD * 0.6F;
		this.leftLeg.xRot = Mth.cos(walkPos) * 1.3F * walkSpeed;
		this.rightLeg.xRot = Mth.cos(walkPos + Mth.PI) * 1.3F * walkSpeed;
		this.leftArm.xRot = Mth.cos(walkPos + Mth.PI) * 1.0F * walkSpeed;
		this.rightArm.xRot = Mth.cos(walkPos) * 1.0F * walkSpeed;
		this.leftArm.zRot = -0.15F - Mth.sin(age * 0.09F) * 0.05F;
		this.rightArm.zRot = 0.15F + Mth.sin(age * 0.09F) * 0.05F;
		this.stem.xRot = 0.25F * walkSpeed + Mth.sin(age * 0.07F) * 0.06F;
		this.stem.zRot = Mth.sin(age * 0.05F) * 0.12F;
		this.leafX.yRot = this.leafZ.yRot = Mth.sin(age * 0.11F) * 0.2F;
		this.flowerX.yRot = this.flowerZ.yRot = age * 0.02F;
		this.root.y += Mth.abs(Mth.sin(walkPos)) * -0.8F * walkSpeed;

		switch (state.state) {
			case BURIED -> {
				// only the stem shows above ground; it wiggles now and then
				float wiggle = Mth.sin(age * 0.6F) * Math.max(0.0F, Mth.sin(age * 0.05F) - 0.7F) * 2.0F;
				this.stem.xRot = 0.0F;
				this.stem.zRot = wiggle;
				this.leftArm.xRot = this.rightArm.xRot = 0.0F;
				this.leftLeg.xRot = this.rightLeg.xRot = 0.0F;
				this.head.xRot = this.head.yRot = 0.0F;
			}
			case THROWN -> {
				this.leftArm.zRot = -1.3F;
				this.rightArm.zRot = 1.3F;
				this.leftLeg.xRot = 0.6F;
				this.rightLeg.xRot = -0.6F;
				this.stem.xRot = 0.9F;
			}
			case ATTACK -> {
				float bash = Math.max(0.0F, 1.0F - state.sinceAction / 7.0F);
				this.head.xRot = 0.2F + bash * 0.9F;
				this.leftArm.xRot = -1.6F + Mth.sin(age * 1.4F) * 0.6F;
				this.rightArm.xRot = -1.6F + Mth.cos(age * 1.4F) * 0.6F;
				this.stem.xRot = -0.5F * bash + 0.2F;
			}
			case CARRY -> {
				this.leftArm.xRot = this.rightArm.xRot = -2.9F;
				this.leftArm.zRot = -0.1F;
				this.rightArm.zRot = 0.1F;
				this.head.xRot = -0.25F;
			}
			case DRINK -> {
				this.head.xRot = 0.55F + Mth.sin(age * 0.5F) * 0.08F;
				this.leftArm.xRot = this.rightArm.xRot = -0.9F;
			}
			case CHARGE -> {
				this.leftArm.xRot = this.rightArm.xRot = -1.4F;
				this.head.xRot = 0.25F;
				this.stem.xRot = 0.6F;
			}
			default -> {
			}
		}
		if (state.drowning) {
			this.leftArm.xRot = this.rightArm.xRot = -3.0F;
			this.leftArm.zRot = -0.4F + Mth.sin(age * 0.9F) * 0.5F;
			this.rightArm.zRot = 0.4F - Mth.sin(age * 0.9F) * 0.5F;
			this.head.xRot = -0.4F;
		}
		if (type == PikminType.WINGED) {
			float flap = Mth.sin(age * 2.2F) * 0.55F;
			this.leftWing.yRot = 0.7F + flap;
			this.rightWing.yRot = -0.7F - flap;
		}
		if (type == PikminType.PURPLE) {
			this.hair.zRot = Mth.sin(age * 0.2F) * 0.1F;
		}
	}
}
