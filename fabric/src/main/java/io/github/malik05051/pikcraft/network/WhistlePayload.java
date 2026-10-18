package io.github.malik05051.pikcraft.network;

import io.github.malik05051.pikcraft.PikCraft;
import io.netty.buffer.ByteBuf;
import net.minecraft.network.codec.ByteBufCodecs;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.protocol.common.custom.CustomPacketPayload;

/** Client to server: blow the whistle where the player looks, with a radius that grows while held. */
public record WhistlePayload(float radius) implements CustomPacketPayload {
	public static final Type<WhistlePayload> TYPE = new Type<>(PikCraft.id("whistle"));
	public static final StreamCodec<ByteBuf, WhistlePayload> CODEC = ByteBufCodecs.FLOAT.map(WhistlePayload::new, WhistlePayload::radius);
	public static final float MAX_RADIUS = 6.0F;

	@Override
	public Type<? extends CustomPacketPayload> type() {
		return TYPE;
	}
}
