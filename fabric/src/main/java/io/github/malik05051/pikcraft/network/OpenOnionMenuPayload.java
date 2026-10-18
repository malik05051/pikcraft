package io.github.malik05051.pikcraft.network;

import io.github.malik05051.pikcraft.PikCraft;
import io.netty.buffer.ByteBuf;
import net.minecraft.network.codec.ByteBufCodecs;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.protocol.common.custom.CustomPacketPayload;

/** Server to client: open (or refresh) the Onion menu with the current counts. */
public record OpenOnionMenuPayload(int onionId, int onionType, int stored, int squad, int field) implements CustomPacketPayload {
	public static final Type<OpenOnionMenuPayload> TYPE = new Type<>(PikCraft.id("open_onion_menu"));
	public static final StreamCodec<ByteBuf, OpenOnionMenuPayload> CODEC = StreamCodec.composite(ByteBufCodecs.VAR_INT, OpenOnionMenuPayload::onionId, ByteBufCodecs.VAR_INT, OpenOnionMenuPayload::onionType, ByteBufCodecs.VAR_INT, OpenOnionMenuPayload::stored, ByteBufCodecs.VAR_INT, OpenOnionMenuPayload::squad, ByteBufCodecs.VAR_INT, OpenOnionMenuPayload::field, OpenOnionMenuPayload::new);

	@Override
	public Type<? extends CustomPacketPayload> type() {
		return TYPE;
	}
}
