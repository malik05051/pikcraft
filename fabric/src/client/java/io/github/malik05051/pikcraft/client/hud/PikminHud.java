package io.github.malik05051.pikcraft.client.hud;

import io.github.malik05051.pikcraft.PikCraft;
import io.github.malik05051.pikcraft.PikminType;
import io.github.malik05051.pikcraft.client.ClientState;
import io.github.malik05051.pikcraft.registry.ModItems;
import net.minecraft.client.DeltaTracker;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.player.LocalPlayer;
import net.minecraft.client.renderer.RenderPipelines;
import net.minecraft.resources.Identifier;
import net.minecraft.util.Mth;

/**
 * The Pikmin-style HUD: a health wheel instead of hearts, the squad counter next to the hotbar and the
 * sun meter that counts down the day.
 */
public final class PikminHud {
	private static final Identifier HEALTH_WHEEL = PikCraft.id("textures/gui/health_wheel.png");
	private static final Identifier ICONS = PikCraft.id("textures/gui/pikmin_icons.png");
	private static final Identifier DAY_METER = PikCraft.id("textures/gui/day_meter.png");
	private static final int WHEEL_FRAMES = 21;
	private static final int WHEEL_SIZE = 24;

	/** Replaces the row of hearts with one wheel that empties and turns red. */
	public static void healthWheel(GuiGraphicsExtractor graphics, DeltaTracker delta) {
		Minecraft minecraft = Minecraft.getInstance();
		LocalPlayer player = minecraft.player;
		if (player == null || !minecraft.gameMode.canHurtPlayer()) {
			return;
		}
		int width = graphics.guiWidth();
		int height = graphics.guiHeight();
		float fraction = Mth.clamp(player.getHealth() / Math.max(1.0F, player.getMaxHealth()), 0.0F, 1.0F);
		int frame = Mth.ceil(fraction * (WHEEL_FRAMES - 1));
		int x = width / 2 - 91 - WHEEL_SIZE - 4;
		int y = height - WHEEL_SIZE - 1;
		// it shakes when low, like the games
		if (fraction <= 0.25F && player.tickCount % 10 < 2) {
			x += player.tickCount % 2 == 0 ? 1 : -1;
		}
		graphics.blit(RenderPipelines.GUI_TEXTURED, HEALTH_WHEEL, x, y, frame * WHEEL_SIZE, 0.0F, WHEEL_SIZE, WHEEL_SIZE, WHEEL_FRAMES * WHEEL_SIZE, WHEEL_SIZE);
		if (player.getAbsorptionAmount() > 0.0F) {
			graphics.centeredText(minecraft.font, "+" + Mth.ceil(player.getAbsorptionAmount()), x + WHEEL_SIZE / 2, y - 9, 0xFFFFD84A);
		}
	}

	/** Pikmin 3 style counter: selected type, how many of it, the squad and everyone out in the field. */
	public static void squad(GuiGraphicsExtractor graphics, DeltaTracker delta) {
		Minecraft minecraft = Minecraft.getInstance();
		LocalPlayer player = minecraft.player;
		if (player == null || minecraft.gui.hud.isHidden()) {
			return;
		}
		int squad = ClientState.hud.total();
		int field = ClientState.hud.field();
		boolean whistle = player.getInventory().contains(stack -> stack.is(ModItems.WHISTLE));
		if (squad == 0 && field == 0 && !whistle) {
			return;
		}
		int width = graphics.guiWidth();
		int height = graphics.guiHeight();
		int x = width / 2 + 91 + 6;
		int y = height - 27;
		PikminType selected = ClientState.selected();
		graphics.fill(x, y, x + 66, y + 26, 0x90101820);
		graphics.fill(x, y, x + 66, y + 1, 0xFF000000 | selected.color);
		boolean any = ClientState.squadOf(selected) > 0;
		graphics.blit(RenderPipelines.GUI_TEXTURED, ICONS, x + 3, y + 5, selected.ordinal() * 16, 0.0F, 16, 16, 16 * PikminType.values().length, 16,
			any ? 0xFFFFFFFF : 0x80FFFFFF);
		graphics.text(minecraft.font, "x" + ClientState.squadOf(selected), x + 22, y + 4, any ? 0xFFFFFFFF : 0xFF909090, true);
		graphics.text(minecraft.font, squad + " / " + field, x + 22, y + 15, 0xFFB8E986, true);
		dayMeter(graphics, minecraft);
	}

	private static void dayMeter(GuiGraphicsExtractor graphics, Minecraft minecraft) {
		long time = minecraft.level.getOverworldClockTime() % 24000L;
		boolean night = time >= 12800L && time <= 23200L;
		int x = 6;
		int y = 6;
		graphics.blit(RenderPipelines.GUI_TEXTURED, DAY_METER, x, y + 2, 0.0F, night ? 10.0F : 0.0F, 102, 9, 128, 32);
		if (night) {
			graphics.blit(RenderPipelines.GUI_TEXTURED, DAY_METER, x + 45, y, 16.0F, 20.0F, 12, 12, 128, 32);
		} else {
			// the day runs from sunrise (23200) to dusk (12800)
			long sinceSunrise = Math.floorMod(time - 23200L, 24000L);
			float progress = Mth.clamp(sinceSunrise / 13600.0F, 0.0F, 1.0F);
			graphics.blit(RenderPipelines.GUI_TEXTURED, DAY_METER, x + Math.round(progress * 90.0F), y, 0.0F, 20.0F, 12, 12, 128, 32);
		}
	}

	private PikminHud() {
	}
}
