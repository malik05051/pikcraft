package io.github.malik05051.pikcraft.network;

import io.github.malik05051.pikcraft.PikCraft;
import io.netty.buffer.ByteBuf;
import net.minecraft.network.codec.ByteBufCodecs;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.protocol.common.custom.CustomPacketPayload;

/** Server to client: play a third-person camera scene around an Onion. */
public record OnionCutscenePayload(int onionId, int kind) implements CustomPacketPayload {
	public static final Type<OnionCutscenePayload> TYPE = new Type<>(PikCraft.id("onion_cutscene"));
	public static final StreamCodec<ByteBuf, OnionCutscenePayload> CODEC = StreamCodec.composite(ByteBufCodecs.VAR_INT, OnionCutscenePayload::onionId, ByteBufCodecs.VAR_INT, OnionCutscenePayload::kind, OnionCutscenePayload::new);
	/** The first, sleeping Onion has just been found. */
	public static final int DISCOVER = 0;
	/** The Onion wakes up. */
	public static final int AWAKEN = 1;

	@Override
	public Type<? extends CustomPacketPayload> type() {
		return TYPE;
	}
}
