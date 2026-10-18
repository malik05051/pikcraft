package io.github.malik05051.pikcraft.client;

import com.mojang.blaze3d.vertex.PoseStack;
import io.github.malik05051.pikcraft.PikCraft;
import io.github.malik05051.pikcraft.PikminType;
import io.github.malik05051.pikcraft.client.hud.PikminHud;
import io.github.malik05051.pikcraft.client.model.OnionModel;
import io.github.malik05051.pikcraft.client.model.PikminModel;
import io.github.malik05051.pikcraft.client.model.SmallModels;
import io.github.malik05051.pikcraft.client.render.OnionRenderer;
import io.github.malik05051.pikcraft.client.render.PikminRenderer;
import io.github.malik05051.pikcraft.client.render.SmallRenderers;
import io.github.malik05051.pikcraft.client.screen.OnionScreen;
import io.github.malik05051.pikcraft.network.EnemyHealthPayload;
import io.github.malik05051.pikcraft.network.HudSyncPayload;
import io.github.malik05051.pikcraft.network.OnionCutscenePayload;
import io.github.malik05051.pikcraft.network.OpenOnionMenuPayload;
import io.github.malik05051.pikcraft.registry.ModEntities;
import it.unimi.dsi.fastutil.ints.Int2ObjectMap;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayConnectionEvents;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayNetworking;
import net.fabricmc.fabric.api.client.rendering.v1.EntityRendererRegistry;
import net.fabricmc.fabric.api.client.rendering.v1.ModelLayerRegistry;
import net.fabricmc.fabric.api.client.rendering.v1.hud.HudElementRegistry;
import net.fabricmc.fabric.api.client.rendering.v1.hud.VanillaHudElements;
import net.fabricmc.fabric.api.client.rendering.v1.level.LevelRenderContext;
import net.fabricmc.fabric.api.client.rendering.v1.level.LevelRenderEvents;
import net.minecraft.ChatFormatting;
import net.minecraft.client.Minecraft;
import net.minecraft.util.LightCoordsUtil;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import net.minecraft.network.chat.Component;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.phys.Vec3;

public class PikCraftClient implements ClientModInitializer {
	/** How long an enemy's health stays shown after a Pikmin hits it. */
	private static final long ENEMY_HEALTH_TICKS = 80L;

	@Override
	public void onInitializeClient() {
		ModelLayerRegistry.registerModelLayer(PikminModel.LAYER, PikminModel::createLayer);
		ModelLayerRegistry.registerModelLayer(OnionModel.LAYER, OnionModel::createLayer);
		ModelLayerRegistry.registerModelLayer(SmallModels.SEED, SmallModels::seed);
		ModelLayerRegistry.registerModelLayer(SmallModels.PELLET, SmallModels::pellet);
		ModelLayerRegistry.registerModelLayer(SmallModels.POSY, SmallModels::posy);
		ModelLayerRegistry.registerModelLayer(SmallModels.POSY_PELLET, SmallModels::posyPellet);
		ModelLayerRegistry.registerModelLayer(SmallModels.NECTAR_EGG, SmallModels::nectarEgg);
		ModelLayerRegistry.registerModelLayer(SmallModels.NECTAR, SmallModels::nectar);

		for (PikminType type : PikminType.values()) {
			EntityRendererRegistry.register(ModEntities.pikmin(type), PikminRenderer::new);
		}
		EntityRendererRegistry.register(ModEntities.ONION, OnionRenderer::new);
		EntityRendererRegistry.register(ModEntities.SEED, SmallRenderers.SeedRenderer::new);
		EntityRendererRegistry.register(ModEntities.PELLET, SmallRenderers.PelletRenderer::new);
		EntityRendererRegistry.register(ModEntities.CORPSE, SmallRenderers.CorpseRenderer::new);
		EntityRendererRegistry.register(ModEntities.PELLET_POSY, SmallRenderers.PosyRenderer::new);
		EntityRendererRegistry.register(ModEntities.NECTAR_EGG, SmallRenderers.NectarEggRenderer::new);
		EntityRendererRegistry.register(ModEntities.NECTAR, SmallRenderers.NectarRenderer::new);

		Controls.init();
		ClientTickEvents.START_CLIENT_TICK.register(Controls::tick);

		ClientPlayNetworking.registerGlobalReceiver(HudSyncPayload.TYPE, (payload, context) -> ClientState.hud = payload);
		ClientPlayNetworking.registerGlobalReceiver(OnionCutscenePayload.TYPE, (payload, context) -> CutsceneCamera.play(payload.onionId(), payload.kind()));
		ClientPlayNetworking.registerGlobalReceiver(EnemyHealthPayload.TYPE, (payload, context) -> {
			if (context.client().level != null) {
				ClientState.ENEMIES.put(payload.entityId(), new ClientState.EnemyHealth(payload.health(), context.client().level.getGameTime()));
			}
		});
		ClientPlayNetworking.registerGlobalReceiver(OpenOnionMenuPayload.TYPE, (payload, context) -> {
			Minecraft minecraft = context.client();
			if (minecraft.gui.screen() instanceof OnionScreen screen && screen.onionId() == payload.onionId()) {
				screen.update(payload);
			} else {
				minecraft.gui.setScreen(new OnionScreen(payload));
			}
		});
		ClientPlayConnectionEvents.DISCONNECT.register((handler, client) -> {
			ClientState.ENEMIES.clear();
			CutsceneCamera.stop();
		});

		HudElementRegistry.replaceElement(VanillaHudElements.HEALTH_BAR, vanilla -> PikminHud::healthWheel);
		HudElementRegistry.attachElementAfter(VanillaHudElements.HOTBAR, PikCraft.id("squad"), PikminHud::squad);
		LevelRenderEvents.COLLECT_SUBMITS.register(PikCraftClient::submitEnemyHealth);
	}

	/** A coloured percentage over enemies Pikmin are fighting. */
	private static void submitEnemyHealth(LevelRenderContext context) {
		Minecraft minecraft = Minecraft.getInstance();
		if (minecraft.level == null || ClientState.ENEMIES.isEmpty()) {
			return;
		}
		long now = minecraft.level.getGameTime();
		float partialTicks = minecraft.getDeltaTracker().getGameTimeDeltaPartialTick(false);
		CameraRenderState camera = context.levelState().cameraRenderState;
		PoseStack poseStack = context.poseStack();
		var iterator = ClientState.ENEMIES.int2ObjectEntrySet().iterator();
		while (iterator.hasNext()) {
			Int2ObjectMap.Entry<ClientState.EnemyHealth> entry = iterator.next();
			Entity entity = minecraft.level.getEntity(entry.getIntKey());
			if (entity == null || !entity.isAlive() || now - entry.getValue().time() > ENEMY_HEALTH_TICKS) {
				iterator.remove();
				continue;
			}
			float health = entry.getValue().health();
			int percent = Math.max(1, Math.round(health * 100.0F));
			ChatFormatting colour = health > 0.6F ? ChatFormatting.GREEN : health > 0.3F ? ChatFormatting.YELLOW : ChatFormatting.RED;
			Component text = Component.literal(wheel(health) + " ").withStyle(colour).append(Component.literal(percent + "%").withStyle(ChatFormatting.WHITE));
			Vec3 pos = entity.getPosition(partialTicks).subtract(camera.pos);
			poseStack.pushPose();
			poseStack.translate(pos.x, pos.y, pos.z);
			context.submitNodeCollector().submitNameTag(poseStack, new Vec3(0.0, entity.getBbHeight() + 0.45, 0.0), 0, text, true,
				LightCoordsUtil.FULL_BRIGHT, camera);
			poseStack.popPose();
		}
	}

	/** A little pie that empties with the enemy's health. */
	private static String wheel(float health) {
		if (health > 0.875F) {
			return "●"; // full
		} else if (health > 0.625F) {
			return "◕";
		} else if (health > 0.375F) {
			return "◑";
		} else if (health > 0.125F) {
			return "◔";
		}
		return "○";
	}
}
