package io.github.malik05051.pikcraft.network;

import io.github.malik05051.pikcraft.PikCraft;
import io.netty.buffer.ByteBuf;
import net.minecraft.network.codec.ByteBufCodecs;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.protocol.common.custom.CustomPacketPayload;

/** Client to server: select the next (or previous) Pikmin type in the squad to throw. */
public record SelectTypePayload(int step) implements CustomPacketPayload {
	public static final Type<SelectTypePayload> TYPE = new Type<>(PikCraft.id("select_type"));
	public static final StreamCodec<ByteBuf, SelectTypePayload> CODEC = ByteBufCodecs.VAR_INT.map(SelectTypePayload::new, SelectTypePayload::step);

	@Override
	public Type<? extends CustomPacketPayload> type() {
		return TYPE;
	}
}
