package io.github.malik05051.pikcraft;

import io.github.malik05051.pikcraft.network.ModNetworking;
import io.github.malik05051.pikcraft.registry.ModAttachments;
import io.github.malik05051.pikcraft.registry.ModCriteria;
import io.github.malik05051.pikcraft.registry.ModEntities;
import io.github.malik05051.pikcraft.registry.ModItems;
import io.github.malik05051.pikcraft.registry.ModSounds;
import io.github.malik05051.pikcraft.world.WorldEvents;
import net.fabricmc.api.ModInitializer;
import net.minecraft.resources.Identifier;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class PikCraft implements ModInitializer {
	public static final String MOD_ID = "pikcraft";
	public static final Logger LOGGER = LoggerFactory.getLogger(MOD_ID);

	public static Identifier id(String path) {
		return Identifier.fromNamespaceAndPath(MOD_ID, path);
	}

	@Override
	public void onInitialize() {
		ModSounds.init();
		ModEntities.init();
		ModItems.init();
		ModCriteria.init();
		ModAttachments.init();
		ModNetworking.init();
		WorldEvents.init();
		LOGGER.info("PikCraft loaded");
	}
}
