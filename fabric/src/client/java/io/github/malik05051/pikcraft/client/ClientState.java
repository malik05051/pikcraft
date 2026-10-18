package io.github.malik05051.pikcraft.client;

import java.util.List;

import io.github.malik05051.pikcraft.PikminType;
import io.github.malik05051.pikcraft.network.HudSyncPayload;
import it.unimi.dsi.fastutil.ints.Int2ObjectOpenHashMap;

/** What the server last told this client: the squad, and the enemies Pikmin are fighting. */
public final class ClientState {
	public static HudSyncPayload hud = new HudSyncPayload(List.of(0, 0, 0, 0, 0, 0, 0), 0, 0);
	public static final Int2ObjectOpenHashMap<EnemyHealth> ENEMIES = new Int2ObjectOpenHashMap<>();

	public record EnemyHealth(float health, long time) {
	}

	public static int squadOf(PikminType type) {
		return type.ordinal() < hud.squad().size() ? hud.squad().get(type.ordinal()) : 0;
	}

	public static PikminType selected() {
		return PikminType.byId(hud.selected());
	}

	private ClientState() {
	}
}
