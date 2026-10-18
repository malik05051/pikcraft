package io.github.malik05051.pikcraft.client.screen;

import io.github.malik05051.pikcraft.PikCraft;
import io.github.malik05051.pikcraft.PikminType;
import io.github.malik05051.pikcraft.network.OnionActionPayload;
import io.github.malik05051.pikcraft.network.OpenOnionMenuPayload;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayNetworking;
import com.mojang.blaze3d.platform.InputConstants;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.input.KeyEvent;
import net.minecraft.client.renderer.RenderPipelines;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import net.minecraft.util.Mth;

/**
 * The Onion menu, laid out like Pikmin 3's: the Onion's count on the left, the squad's on the right,
 * arrows to move Pikmin between them, then confirm.
 */
public class OnionScreen extends Screen {
	private static final Identifier PANEL = PikCraft.id("textures/gui/onion_menu.png");
	private static final Identifier ICONS = PikCraft.id("textures/gui/pikmin_icons.png");
	private static final int WIDTH = 220;
	private static final int HEIGHT = 120;
	private static final int FIELD_LIMIT = 100;

	private OpenOnionMenuPayload data;
	/** Pikmin to bring out (positive) or send in (negative). */
	private int change;

	public OnionScreen(OpenOnionMenuPayload data) {
		super(Component.translatable("screen.pikcraft.onion"));
		this.data = data;
	}

	public int onionId() {
		return this.data.onionId();
	}

	public void update(OpenOnionMenuPayload data) {
		this.data = data;
		this.change = Mth.clamp(this.change, this.minChange(), this.maxChange());
	}

	private int minChange() {
		return -this.data.squad();
	}

	private int maxChange() {
		return Math.max(0, Math.min(this.data.stored(), FIELD_LIMIT - this.data.field()));
	}

	private void add(int amount) {
		this.change = Mth.clamp(this.change + amount, this.minChange(), this.maxChange());
	}

	@Override
	protected void init() {
		int left = (this.width - WIDTH) / 2;
		int top = (this.height - HEIGHT) / 2;
		int y = top + 70;
		this.addRenderableWidget(Button.builder(Component.literal("<<"), b -> this.add(-10)).bounds(left + 50, y, 26, 20).build());
		this.addRenderableWidget(Button.builder(Component.literal("<"), b -> this.add(-1)).bounds(left + 80, y, 26, 20).build());
		this.addRenderableWidget(Button.builder(Component.literal(">"), b -> this.add(1)).bounds(left + 114, y, 26, 20).build());
		this.addRenderableWidget(Button.builder(Component.literal(">>"), b -> this.add(10)).bounds(left + 144, y, 26, 20).build());
		this.addRenderableWidget(Button.builder(Component.translatable("screen.pikcraft.onion.confirm"), b -> this.confirm())
			.bounds(left + WIDTH / 2 - 40, top + HEIGHT - 26, 80, 20).build());
	}

	private void confirm() {
		if (this.change != 0) {
			ClientPlayNetworking.send(new OnionActionPayload(this.data.onionId(), this.change));
		}
		this.onClose();
	}

	@Override
	public boolean keyPressed(KeyEvent event) {
		switch (event.key()) {
			case InputConstants.KEY_LEFT, InputConstants.KEY_A, InputConstants.KEY_Q -> {
				this.add(event.hasShiftDown() ? -10 : -1);
				return true;
			}
			case InputConstants.KEY_RIGHT, InputConstants.KEY_D -> {
				this.add(event.hasShiftDown() ? 10 : 1);
				return true;
			}
			case InputConstants.KEY_RETURN, InputConstants.KEY_NUMPADENTER, InputConstants.KEY_SPACE -> {
				this.confirm();
				return true;
			}
			default -> {
				return super.keyPressed(event);
			}
		}
	}

	@Override
	public boolean mouseScrolled(double x, double y, double scrollX, double scrollY) {
		this.add(scrollY > 0 ? 1 : -1);
		return true;
	}

	@Override
	public void extractBackground(GuiGraphicsExtractor graphics, int mouseX, int mouseY, float a) {
		super.extractBackground(graphics, mouseX, mouseY, a);
		PikminType type = PikminType.byId(this.data.onionType());
		int left = (this.width - WIDTH) / 2;
		int top = (this.height - HEIGHT) / 2;
		graphics.blit(RenderPipelines.GUI_TEXTURED, PANEL, left, top, 0.0F, 0.0F, WIDTH, HEIGHT, 256, 128);
		graphics.fill(left + 8, top + 6, left + WIDTH - 8, top + 7, 0xFF000000 | type.color);
	}

	@Override
	public void extractRenderState(GuiGraphicsExtractor graphics, int mouseX, int mouseY, float a) {
		super.extractRenderState(graphics, mouseX, mouseY, a);
		PikminType type = PikminType.byId(this.data.onionType());
		int left = (this.width - WIDTH) / 2;
		int top = (this.height - HEIGHT) / 2;
		graphics.centeredText(this.font, Component.translatable("screen.pikcraft.onion.title", Component.translatable("pikmin.pikcraft." + type.id())),
			left + WIDTH / 2, top + 10, 0xFFFFFFFF);

		int onionCount = this.data.stored() - this.change;
		int squadCount = this.data.squad() + this.change;
		// the Onion's side
		graphics.blit(RenderPipelines.GUI_TEXTURED, PANEL, left + 18, top + 28, 224.0F, 0.0F, 32, 32, 256, 128, 0xFF000000 | type.color);
		graphics.centeredText(this.font, Component.translatable("screen.pikcraft.onion.in_onion"), left + 34, top + 62, 0xFFDDDDDD);
		this.bigNumber(graphics, onionCount, left + 62, top + 30);
		// the squad's side
		graphics.blit(RenderPipelines.GUI_TEXTURED, ICONS, left + WIDTH - 46, top + 32, type.ordinal() * 16, 0.0F, 16, 16, 16 * PikminType.values().length, 16);
		graphics.blit(RenderPipelines.GUI_TEXTURED, ICONS, left + WIDTH - 38, top + 40, type.ordinal() * 16, 0.0F, 16, 16, 16 * PikminType.values().length, 16);
		graphics.centeredText(this.font, Component.translatable("screen.pikcraft.onion.squad"), left + WIDTH - 34, top + 62, 0xFFDDDDDD);
		this.bigNumber(graphics, squadCount, left + WIDTH - 98, top + 30);
		// the field count
		int field = this.data.field() + Math.max(0, this.change);
		graphics.centeredText(this.font, Component.translatable("screen.pikcraft.onion.field", field, FIELD_LIMIT), left + WIDTH / 2, top + 57,
			field >= FIELD_LIMIT ? 0xFFFF7060 : 0xFFB8E986);
	}

	private void bigNumber(GuiGraphicsExtractor graphics, int value, int x, int y) {
		graphics.pose().pushMatrix();
		graphics.pose().translate(x, y);
		graphics.pose().scale(2.0F, 2.0F);
		graphics.centeredText(this.font, String.valueOf(value), 9, 0, 0xFFFFFFFF);
		graphics.pose().popMatrix();
	}

	@Override
	public boolean isPauseScreen() {
		return false;
	}
}
