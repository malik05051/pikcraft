package io.github.malik05051.pikcraft.client.model;

import io.github.malik05051.pikcraft.PikminType;
import io.github.malik05051.pikcraft.entity.PikminEntity;
import net.minecraft.client.renderer.entity.state.LivingEntityRenderState;

public class PikminRenderState extends LivingEntityRenderState {
	public PikminType type = PikminType.RED;
	public int maturity;
	public PikminEntity.State state = PikminEntity.State.IDLE;
	public boolean drowning;
	/** Ticks since the last attack or pluck. */
	public float sinceAction = 100.0F;
	/** A per-Pikmin phase so a crowd doesn't move in lockstep. */
	public float phase;
}
