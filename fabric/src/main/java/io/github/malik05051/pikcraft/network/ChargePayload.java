package io.github.malik05051.pikcraft.network;

import io.github.malik05051.pikcraft.PikCraft;
import io.netty.buffer.ByteBuf;
import net.minecraft.network.codec.ByteBufCodecs;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.protocol.common.custom.CustomPacketPayload;

/** Client to server: the whole squad charges where the player looks. */
public record ChargePayload() implements CustomPacketPayload {
	public static final Type<ChargePayload> TYPE = new Type<>(PikCraft.id("charge"));
	public static final StreamCodec<ByteBuf, ChargePayload> CODEC = StreamCodec.unit(new ChargePayload());

	@Override
	public Type<? extends CustomPacketPayload> type() {
		return TYPE;
	}
}
