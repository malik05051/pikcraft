package io.github.malik05051.pikcraft.network;

import io.github.malik05051.pikcraft.PikCraft;
import io.netty.buffer.ByteBuf;
import net.minecraft.network.codec.ByteBufCodecs;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.protocol.common.custom.CustomPacketPayload;

/** Server to client: an enemy hit by Pikmin, with the fraction of health it has left. */
public record EnemyHealthPayload(int entityId, float health) implements CustomPacketPayload {
	public static final Type<EnemyHealthPayload> TYPE = new Type<>(PikCraft.id("enemy_health"));
	public static final StreamCodec<ByteBuf, EnemyHealthPayload> CODEC = StreamCodec.composite(ByteBufCodecs.VAR_INT, EnemyHealthPayload::entityId, ByteBufCodecs.FLOAT, EnemyHealthPayload::health, EnemyHealthPayload::new);

	@Override
	public Type<? extends CustomPacketPayload> type() {
		return TYPE;
	}
}
