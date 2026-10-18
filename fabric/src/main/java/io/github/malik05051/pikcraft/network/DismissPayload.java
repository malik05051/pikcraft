package io.github.malik05051.pikcraft.network;

import io.github.malik05051.pikcraft.PikCraft;
import io.netty.buffer.ByteBuf;
import net.minecraft.network.codec.ByteBufCodecs;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.protocol.common.custom.CustomPacketPayload;

/** Client to server: the squad splits up by colour and waits. */
public record DismissPayload() implements CustomPacketPayload {
	public static final Type<DismissPayload> TYPE = new Type<>(PikCraft.id("dismiss"));
	public static final StreamCodec<ByteBuf, DismissPayload> CODEC = StreamCodec.unit(new DismissPayload());

	@Override
	public Type<? extends CustomPacketPayload> type() {
		return TYPE;
	}
}
