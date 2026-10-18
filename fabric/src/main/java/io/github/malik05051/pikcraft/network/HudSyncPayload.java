package io.github.malik05051.pikcraft.network;

import java.util.List;

import io.github.malik05051.pikcraft.PikCraft;
import io.netty.buffer.ByteBuf;
import net.minecraft.network.codec.ByteBufCodecs;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.protocol.common.custom.CustomPacketPayload;

/** Server to client: the player's squad by type, the selected type and the Pikmin out in the field. */
public record HudSyncPayload(List<Integer> squad, int selected, int field) implements CustomPacketPayload {
	public static final Type<HudSyncPayload> TYPE = new Type<>(PikCraft.id("hud_sync"));
	public static final StreamCodec<ByteBuf, HudSyncPayload> CODEC = StreamCodec.composite(
		ByteBufCodecs.VAR_INT.apply(ByteBufCodecs.list(16)), HudSyncPayload::squad,
		ByteBufCodecs.VAR_INT, HudSyncPayload::selected,
		ByteBufCodecs.VAR_INT, HudSyncPayload::field,
		HudSyncPayload::new);

	public int total() {
		return this.squad.stream().mapToInt(Integer::intValue).sum();
	}

	@Override
	public Type<? extends CustomPacketPayload> type() {
		return TYPE;
	}
}
