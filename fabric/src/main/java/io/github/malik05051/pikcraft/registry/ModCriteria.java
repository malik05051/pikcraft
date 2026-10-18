package io.github.malik05051.pikcraft.registry;

import java.util.Optional;

import com.mojang.serialization.Codec;
import com.mojang.serialization.codecs.RecordCodecBuilder;
import io.github.malik05051.pikcraft.PikCraft;
import net.minecraft.advancements.triggers.SimpleCriterionTrigger;
import net.minecraft.core.Holder;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.level.storage.loot.predicates.LootItemCondition;

/** One advancement trigger for every PikCraft event ("pikcraft:event" with an "event" name). */
public final class ModCriteria {
	public static final EventTrigger EVENT = Registry.register(BuiltInRegistries.TRIGGER_TYPES, PikCraft.id("event"), new EventTrigger());

	public static void trigger(ServerPlayer player, String event) {
		EVENT.trigger(player, event);
	}

	public static void init() {
	}

	private ModCriteria() {
	}

	public static class EventTrigger extends SimpleCriterionTrigger<EventTrigger.Instance> {
		@Override
		public Codec<Instance> codec() {
			return Instance.CODEC;
		}

		public void trigger(ServerPlayer player, String event) {
			this.trigger(player, instance -> instance.event().equals(event));
		}

		public record Instance(Optional<Holder<LootItemCondition>> player, String event) implements SimpleCriterionTrigger.SimpleInstance {
			public static final Codec<Instance> CODEC = RecordCodecBuilder.create(i -> i.group(
				LootItemCondition.CODEC.optionalFieldOf("player").forGetter(Instance::player),
				Codec.STRING.fieldOf("event").forGetter(Instance::event)
			).apply(i, Instance::new));
		}
	}
}
