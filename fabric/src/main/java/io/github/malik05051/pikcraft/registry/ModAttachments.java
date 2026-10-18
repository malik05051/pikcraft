package io.github.malik05051.pikcraft.registry;

import com.mojang.serialization.Codec;
import io.github.malik05051.pikcraft.PikCraft;
import net.fabricmc.fabric.api.attachment.v1.AttachmentRegistry;
import net.fabricmc.fabric.api.attachment.v1.AttachmentType;

/** Per-player data, saved with the player. */
public final class ModAttachments {
	/** Whether this player's first, sleeping Onion has been placed near them. */
	public static final AttachmentType<Boolean> START_ONION = AttachmentRegistry.create(PikCraft.id("start_onion"),
		builder -> builder.persistent(Codec.BOOL).initializer(() -> false).copyOnDeath());
	public static final AttachmentType<Integer> ONIONS_AWAKENED = AttachmentRegistry.create(PikCraft.id("onions_awakened"),
		builder -> builder.persistent(Codec.INT).initializer(() -> 0).copyOnDeath());
	/** The Pikmin type thrown next (an index into PikminType). */
	public static final AttachmentType<Integer> SELECTED_TYPE = AttachmentRegistry.create(PikCraft.id("selected_type"),
		builder -> builder.persistent(Codec.INT).initializer(() -> 0).copyOnDeath());

	public static void init() {
	}

	private ModAttachments() {
	}
}
