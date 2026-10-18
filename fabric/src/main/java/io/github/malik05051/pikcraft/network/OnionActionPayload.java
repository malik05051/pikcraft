package io.github.malik05051.pikcraft.network;

import io.github.malik05051.pikcraft.PikCraft;
import io.netty.buffer.ByteBuf;
import net.minecraft.network.codec.ByteBufCodecs;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.protocol.common.custom.CustomPacketPayload;

/** Client to server, from the Onion menu: a positive amount calls Pikmin out, a negative one sends them in. */
public record OnionActionPayload(int onionId, int amount) implements CustomPacketPayload {
	public static final Type<OnionActionPayload> TYPE = new Type<>(PikCraft.id("onion_action"));
	public static final StreamCodec<ByteBuf, OnionActionPayload> CODEC = StreamCodec.composite(ByteBufCodecs.VAR_INT, OnionActionPayload::onionId, ByteBufCodecs.VAR_INT, OnionActionPayload::amount, OnionActionPayload::new);

	@Override
	public Type<? extends CustomPacketPayload> type() {
		return TYPE;
	}
}
