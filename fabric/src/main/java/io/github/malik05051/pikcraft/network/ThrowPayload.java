package io.github.malik05051.pikcraft.network;

import io.github.malik05051.pikcraft.PikCraft;
import io.netty.buffer.ByteBuf;
import net.minecraft.network.codec.ByteBufCodecs;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.protocol.common.custom.CustomPacketPayload;

/** Client to server: throw the selected Pikmin at what the player looks at. */
public record ThrowPayload() implements CustomPacketPayload {
	public static final Type<ThrowPayload> TYPE = new Type<>(PikCraft.id("throw"));
	public static final StreamCodec<ByteBuf, ThrowPayload> CODEC = StreamCodec.unit(new ThrowPayload());

	@Override
	public Type<? extends CustomPacketPayload> type() {
		return TYPE;
	}
}
