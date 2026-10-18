package io.github.malik05051.pikcraft.client.render;

import java.util.Map;
import java.util.WeakHashMap;

import com.mojang.blaze3d.vertex.PoseStack;
import com.mojang.math.Axis;
import io.github.malik05051.pikcraft.PikCraft;
import io.github.malik05051.pikcraft.client.model.SmallModels;
import io.github.malik05051.pikcraft.entity.CorpseEntity;
import io.github.malik05051.pikcraft.entity.NectarEggEntity;
import io.github.malik05051.pikcraft.entity.NectarEntity;
import io.github.malik05051.pikcraft.entity.PelletEntity;
import io.github.malik05051.pikcraft.entity.PelletPosyEntity;
import io.github.malik05051.pikcraft.entity.SeedEntity;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.entity.EntityRenderDispatcher;
import net.minecraft.client.renderer.entity.EntityRenderer;
import net.minecraft.client.renderer.entity.EntityRendererProvider;
import net.minecraft.client.renderer.entity.LivingEntityRenderer;
import net.minecraft.client.renderer.entity.MobRenderer;
import net.minecraft.client.renderer.entity.layers.RenderLayer;
import net.minecraft.client.renderer.entity.state.EntityRenderState;
import net.minecraft.client.renderer.entity.state.LivingEntityRenderState;
import net.minecraft.client.renderer.rendertype.RenderTypes;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import net.minecraft.client.renderer.texture.OverlayTexture;
import net.minecraft.resources.Identifier;
import net.minecraft.util.Mth;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.LivingEntity;
import org.jspecify.annotations.Nullable;

/** Renderers for seeds, pellets, bodies, pellet posies, nectar eggs and nectar. */
public final class SmallRenderers {
	private static Identifier texture(String name) {
		return PikCraft.id("textures/entity/" + name + ".png");
	}

	private static void toModelSpace(PoseStack poseStack) {
		poseStack.scale(-1.0F, -1.0F, 1.0F);
		poseStack.translate(0.0F, -1.501F, 0.0F);
	}

	// ------------------------------------------------------------------ seed

	public static class SeedState extends EntityRenderState {
		public int colour;
		public boolean landed;
	}

	public static class SeedRenderer extends EntityRenderer<SeedEntity, SeedState> {
		private static final Identifier TEXTURE = texture("seed");
		private final SmallModels.Simple<SeedState> model;

		public SeedRenderer(EntityRendererProvider.Context context) {
			super(context);
			this.model = new SmallModels.Simple<>(context.bakeLayer(SmallModels.SEED));
			this.shadowRadius = 0.1F;
		}

		@Override
		public SeedState createRenderState() {
			return new SeedState();
		}

		@Override
		public void extractRenderState(SeedEntity entity, SeedState state, float partialTicks) {
			super.extractRenderState(entity, state, partialTicks);
			state.colour = entity.seedType().color;
			state.landed = entity.onGround();
		}

		@Override
		public void submit(SeedState state, PoseStack poseStack, SubmitNodeCollector collector, CameraRenderState camera) {
			poseStack.pushPose();
			if (!state.landed) {
				poseStack.translate(0.0F, 0.1F, 0.0F);
				poseStack.rotateDegrees(Axis.YP, state.ageInTicks * 25.0F);
				poseStack.rotateDegrees(Axis.ZP, state.ageInTicks * 18.0F);
			}
			poseStack.scale(1.5F, 1.5F, 1.5F);
			toModelSpace(poseStack);
			collector.submitModel(this.model, state, poseStack, RenderTypes.entityCutout(TEXTURE), state.lightCoords, OverlayTexture.NO_OVERLAY,
				0xFF000000 | state.colour, null, state.outlineColor);
			poseStack.popPose();
			super.submit(state, poseStack, collector, camera);
		}
	}

	// ------------------------------------------------------------------ pellet

	public static class PelletState extends LivingEntityRenderState {
		public int value = 1;
		public int colour;
		public float size = 1.0F;
	}

	public static class PelletRenderer extends LivingEntityRenderer<PelletEntity, PelletState, SmallModels.Simple<PelletState>> {
		private static final Identifier[] TEXTURES = {texture("pellet_1"), texture("pellet_5"), texture("pellet_10"), texture("pellet_20")};

		public PelletRenderer(EntityRendererProvider.Context context) {
			super(context, new SmallModels.Simple<>(context.bakeLayer(SmallModels.PELLET)), 0.35F);
		}

		@Override
		public Identifier getTextureLocation(PelletState state) {
			return TEXTURES[switch (state.value) {
				case 5 -> 1;
				case 10 -> 2;
				case 20 -> 3;
				default -> 0;
			}];
		}

		@Override
		public PelletState createRenderState() {
			return new PelletState();
		}

		@Override
		public void extractRenderState(PelletEntity entity, PelletState state, float partialTicks) {
			super.extractRenderState(entity, state, partialTicks);
			state.value = entity.value();
			state.colour = entity.colour().color;
			state.size = entity.size() * 2.0F;
			state.hasRedOverlay = false;
		}

		@Override
		protected int getModelTint(PelletState state) {
			return 0xFF000000 | state.colour;
		}

		@Override
		protected void scale(PelletState state, PoseStack poseStack) {
			poseStack.scale(state.size, state.size, state.size);
		}
	}

	// ------------------------------------------------------------------ the body of a defeated enemy

	public static class CorpseState extends EntityRenderState {
		public @Nullable EntityRenderState body;
		public float yaw;
		public float wobble;
	}

	/** Draws the defeated mob itself, lying on its side, rocking a little while it is carried. */
	public static class CorpseRenderer extends EntityRenderer<CorpseEntity, CorpseState> {
		private static final Map<CorpseEntity, Entity> BODIES = new WeakHashMap<>();
		private final EntityRenderDispatcher dispatcher;

		public CorpseRenderer(EntityRendererProvider.Context context) {
			super(context);
			this.dispatcher = context.getEntityRenderDispatcher();
			this.shadowRadius = 0.5F;
		}

		@Override
		public CorpseState createRenderState() {
			return new CorpseState();
		}

		@Override
		public void extractRenderState(CorpseEntity corpse, CorpseState state, float partialTicks) {
			super.extractRenderState(corpse, state, partialTicks);
			state.body = null;
			Entity body = BODIES.get(corpse);
			if (body == null || body.getType() != corpse.bodyType().orElse(null)) {
				body = corpse.bodyType().map(type -> type.create(corpse.level(), EntitySpawnReason.LOAD)).orElse(null);
				if (body == null) {
					return;
				}
				body.setId(-1000 - corpse.getId());
				BODIES.put(corpse, body);
			}
			body.setPos(corpse.position());
			body.setOldPosAndRot(corpse.position(), 0.0F, 0.0F);
			body.setYRot(0.0F);
			if (body instanceof LivingEntity living) {
				living.yBodyRot = living.yBodyRotO = 0.0F;
				living.yHeadRot = living.yHeadRotO = 0.0F;
			}
			body.tickCount = 0;
			EntityRenderState bodyState = this.dispatcher.extractEntity(body, partialTicks);
			bodyState.shadowPieces.clear();
			bodyState.lightCoords = state.lightCoords;
			bodyState.outlineColor = state.outlineColor;
			if (bodyState instanceof LivingEntityRenderState living) {
				living.deathTime = 0.0F;
				living.hasRedOverlay = false;
				living.walkAnimationSpeed = 0.0F;
			}
			state.body = bodyState;
			state.yaw = corpse.bodyYaw();
			boolean moving = corpse.getDeltaMovement().horizontalDistanceSqr() > 1.0E-4 || corpse.isPassenger();
			state.wobble = moving ? Mth.sin(state.ageInTicks * 0.5F) * 6.0F : 0.0F;
		}

		@Override
		public void submit(CorpseState state, PoseStack poseStack, SubmitNodeCollector collector, CameraRenderState camera) {
			if (state.body != null) {
				poseStack.pushPose();
				poseStack.rotateDegrees(Axis.YP, -state.yaw);
				poseStack.translate(0.0F, state.body.boundingBoxWidth * 0.5F, 0.0F);
				poseStack.rotateDegrees(Axis.ZP, 90.0F + state.wobble);
				poseStack.translate(0.0F, -state.body.boundingBoxWidth * 0.5F, 0.0F);
				this.dispatcher.submit(state.body, camera, 0.0, 0.0, 0.0, poseStack, collector);
				poseStack.popPose();
			}
			super.submit(state, poseStack, collector, camera);
		}
	}

	// ------------------------------------------------------------------ pellet posy

	public static class PosyRenderer extends MobRenderer<PelletPosyEntity, SmallModels.PosyState, SmallModels.PosyModel> {
		private static final Identifier TEXTURE = texture("pellet_posy");
		private static final Identifier[] PELLETS = {texture("pellet_1"), texture("pellet_5"), texture("pellet_10"), texture("pellet_20")};

		public PosyRenderer(EntityRendererProvider.Context context) {
			super(context, new SmallModels.PosyModel(context.bakeLayer(SmallModels.POSY)), 0.4F);
			SmallModels.PosyModel pellet = new SmallModels.PosyModel(context.bakeLayer(SmallModels.POSY_PELLET));
			this.addLayer(new RenderLayer<>(this) {
				@Override
				public void submit(PoseStack poseStack, SubmitNodeCollector collector, int light, SmallModels.PosyState state, float yRot, float xRot) {
					if (state.value <= 0) {
						return;
					}
					Identifier texture = PELLETS[state.value >= 20 ? 3 : state.value >= 10 ? 2 : state.value >= 5 ? 1 : 0];
					collector.submitModel(pellet, state, poseStack, RenderTypes.entityCutout(texture), light,
						LivingEntityRenderer.getOverlayCoords(state, 0.0F), 0xFF000000 | state.colour, null, state.outlineColor);
				}
			});
		}

		@Override
		public Identifier getTextureLocation(SmallModels.PosyState state) {
			return TEXTURE;
		}

		@Override
		public SmallModels.PosyState createRenderState() {
			return new SmallModels.PosyState();
		}

		@Override
		public void extractRenderState(PelletPosyEntity entity, SmallModels.PosyState state, float partialTicks) {
			super.extractRenderState(entity, state, partialTicks);
			state.value = entity.pelletValue();
			state.colour = entity.pelletColour().color;
			state.sinceHit = entity.tickCount - entity.hitTick() + partialTicks;
			state.hasRedOverlay = false;
		}
	}

	// ------------------------------------------------------------------ nectar egg

	public static class NectarEggRenderer extends MobRenderer<NectarEggEntity, LivingEntityRenderState, SmallModels.Simple<LivingEntityRenderState>> {
		private static final Identifier TEXTURE = texture("nectar_egg");

		public NectarEggRenderer(EntityRendererProvider.Context context) {
			super(context, new SmallModels.Simple<>(context.bakeLayer(SmallModels.NECTAR_EGG)), 0.35F);
		}

		@Override
		public Identifier getTextureLocation(LivingEntityRenderState state) {
			return TEXTURE;
		}

		@Override
		public LivingEntityRenderState createRenderState() {
			return new LivingEntityRenderState();
		}

		@Override
		protected void setupRotations(LivingEntityRenderState state, PoseStack poseStack, float bodyRot, float entityScale) {
			super.setupRotations(state, poseStack, bodyRot, entityScale);
			if (state.hasRedOverlay) {
				poseStack.rotateDegrees(Axis.ZP, Mth.sin(state.ageInTicks * 2.5F) * 12.0F); // it wobbles when hit
			}
		}

		@Override
		protected float getFlipDegrees() {
			return 0.0F; // it cracks, it doesn't fall over
		}
	}

	// ------------------------------------------------------------------ nectar

	public static class NectarState extends EntityRenderState {
		public float amount;
	}

	public static class NectarRenderer extends EntityRenderer<NectarEntity, NectarState> {
		private static final Identifier TEXTURE = texture("nectar");
		private final SmallModels.Simple<NectarState> model;

		public NectarRenderer(EntityRendererProvider.Context context) {
			super(context);
			this.model = new SmallModels.Simple<>(context.bakeLayer(SmallModels.NECTAR));
		}

		@Override
		public NectarState createRenderState() {
			return new NectarState();
		}

		@Override
		public void extractRenderState(NectarEntity entity, NectarState state, float partialTicks) {
			super.extractRenderState(entity, state, partialTicks);
			state.amount = entity.sips() / 5.0F;
		}

		@Override
		public void submit(NectarState state, PoseStack poseStack, SubmitNodeCollector collector, CameraRenderState camera) {
			poseStack.pushPose();
			float size = 0.45F + 0.55F * state.amount;
			poseStack.translate(0.0F, 0.01F, 0.0F);
			poseStack.scale(size, 1.0F + Mth.sin(state.ageInTicks * 0.15F) * 0.15F, size);
			toModelSpace(poseStack);
			collector.submitModel(this.model, state, poseStack, RenderTypes.entityTranslucent(TEXTURE), 0xF000F0, OverlayTexture.NO_OVERLAY,
				0xE0FFFFFF, null, state.outlineColor);
			poseStack.popPose();
			super.submit(state, poseStack, collector, camera);
		}
	}

	private SmallRenderers() {
	}
}
