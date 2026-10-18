package io.github.malik05051.pikcraft.registry;

import java.util.function.Function;

import io.github.malik05051.pikcraft.PikCraft;
import io.github.malik05051.pikcraft.PikminType;
import io.github.malik05051.pikcraft.item.WhistleItem;
import net.fabricmc.fabric.api.creativetab.v1.FabricCreativeModeTab;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.ResourceKey;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.item.CreativeModeTab;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Rarity;
import net.minecraft.world.item.SpawnEggItem;

public final class ModItems {
	public static final Item WHISTLE = register("whistle", WhistleItem::new, new Item.Properties().stacksTo(1).rarity(Rarity.UNCOMMON));
	public static final Item[] PIKMIN_SPAWN_EGGS = new Item[PikminType.values().length];

	static {
		for (PikminType type : PikminType.values()) {
			PIKMIN_SPAWN_EGGS[type.ordinal()] = spawnEgg(type.id() + "_pikmin_spawn_egg", ModEntities.pikmin(type));
		}
	}

	public static final Item ONION_SPAWN_EGG = spawnEgg("onion_spawn_egg", ModEntities.ONION);
	public static final Item PELLET_POSY_SPAWN_EGG = spawnEgg("pellet_posy_spawn_egg", ModEntities.PELLET_POSY);
	public static final Item NECTAR_EGG_SPAWN_EGG = spawnEgg("nectar_egg_spawn_egg", ModEntities.NECTAR_EGG);

	public static final CreativeModeTab TAB = Registry.register(BuiltInRegistries.CREATIVE_MODE_TAB, PikCraft.id("pikcraft"),
		FabricCreativeModeTab.builder()
			.title(Component.translatable("itemGroup.pikcraft"))
			.icon(() -> new ItemStack(WHISTLE))
			.displayItems((parameters, output) -> {
				output.accept(WHISTLE);
				for (Item egg : PIKMIN_SPAWN_EGGS) {
					output.accept(egg);
				}
				output.accept(ONION_SPAWN_EGG);
				output.accept(PELLET_POSY_SPAWN_EGG);
				output.accept(NECTAR_EGG_SPAWN_EGG);
			})
			.build());

	private static Item spawnEgg(String name, EntityType<?> type) {
		return register(name, SpawnEggItem::new, new Item.Properties().spawnEgg(type));
	}

	private static Item register(String name, Function<Item.Properties, Item> factory, Item.Properties properties) {
		ResourceKey<Item> key = ResourceKey.create(Registries.ITEM, PikCraft.id(name));
		return Registry.register(BuiltInRegistries.ITEM, key, factory.apply(properties.setId(key)));
	}

	public static void init() {
	}

	private ModItems() {
	}
}
