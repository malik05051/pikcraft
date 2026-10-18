package io.github.malik05051.pikcraft.test;

import java.util.List;

import io.github.malik05051.pikcraft.PikminType;
import io.github.malik05051.pikcraft.client.ClientState;
import io.github.malik05051.pikcraft.client.CutsceneCamera;
import io.github.malik05051.pikcraft.client.screen.OnionScreen;
import io.github.malik05051.pikcraft.entity.CorpseEntity;
import io.github.malik05051.pikcraft.entity.NectarEntity;
import io.github.malik05051.pikcraft.entity.OnionEntity;
import io.github.malik05051.pikcraft.entity.PelletEntity;
import io.github.malik05051.pikcraft.entity.PelletPosyEntity;
import io.github.malik05051.pikcraft.entity.PikminEntity;
import io.github.malik05051.pikcraft.registry.ModEntities;
import io.github.malik05051.pikcraft.registry.ModItems;
import io.github.malik05051.pikcraft.squad.Squads;
import net.fabricmc.fabric.api.client.gametest.v1.FabricClientGameTest;
import net.fabricmc.fabric.api.client.gametest.v1.context.ClientGameTestContext;
import net.fabricmc.fabric.api.client.gametest.v1.context.TestDedicatedServerConnection;
import net.fabricmc.fabric.api.client.gametest.v1.context.TestDedicatedServerContext;
import net.fabricmc.fabric.api.client.gametest.v1.context.TestServerContext;
import net.fabricmc.fabric.api.client.gametest.v1.context.TestSingleplayerContext;
import net.minecraft.client.CameraType;
import net.minecraft.client.gui.screens.inventory.InventoryScreen;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.Mob;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;

/**
 * Plays through the mod in a real client: the first Onion and its scenes, plucking, the squad, throwing,
 * fighting, carrying, the pellet posy, nectar, the menus and the night; then the same start on a
 * dedicated server. Screenshots land in build/run/clientGameTest/screenshots.
 */
public class PikCraftClientTests implements FabricClientGameTest {
	@Override
	public void runTest(ClientGameTestContext context) {
		this.singleplayer(context);
		this.dedicatedServer(context);
	}

	private void singleplayer(ClientGameTestContext context) {
		try (TestSingleplayerContext world = context.worldBuilder().create()) {
			world.getConnection().waitForChunksRender();
			TestServerContext server = world.getServer();
			server.runCommand("gamemode survival @a");
			server.runCommand("difficulty normal");
			server.runCommand("gamerule spawn_mobs false");
			server.runCommand("gamerule advance_time false");
			server.runCommand("time set 1000");

			// the first, sleeping Onion appears near the player with its discovery scene
			context.waitFor(mc -> CutsceneCamera.isPlaying(), 400);
			context.waitTicks(40);
			context.takeScreenshot("01_discover_onion");
			context.waitFor(mc -> !CutsceneCamera.isPlaying(), 400);
			server.runCommand("tp @p ~ ~ ~ facing entity @e[type=pikcraft:onion,limit=1] feet");
			context.waitTicks(10);
			context.takeScreenshot("02_dormant_onion");

			// touching it wakes it up: it pushes the player back and spits a seed
			server.runOnServer(s -> onion(s).awaken(player(s)));
			context.waitTicks(30);
			context.takeScreenshot("03_onion_awakens");
			context.waitFor(mc -> !CutsceneCamera.isPlaying(), 400);
			server.waitFor(s -> !pikmin(s, p -> p.state() == PikminEntity.State.BURIED).isEmpty(), 400);
			server.runCommand("tp @p ~ ~ ~ facing entity @e[type=pikcraft:onion,limit=1] feet");
			context.waitTicks(20);
			context.takeScreenshot("04_sprout");
			server.runOnServer(s -> pikmin(s, p -> p.state() == PikminEntity.State.BURIED).forEach(p -> p.pluck(player(s))));

			// a squad of every type, with a whistle in hand
			server.runOnServer(s -> {
				ServerPlayer player = player(s);
				player.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND, new ItemStack(ModItems.WHISTLE));
				int i = 0;
				for (PikminType type : PikminType.values()) {
					for (int n = 0; n < 3; n++, i++) {
						PikminEntity pikmin = ModEntities.pikmin(type).create(player.level(), EntitySpawnReason.COMMAND);
						pikmin.snapTo(player.getX() + (i % 5) - 2, player.getY(), player.getZ() + 2 + i / 5, 0.0F, 0.0F);
						pikmin.setMaturity(n);
						player.level().addFreshEntity(pikmin);
						pikmin.follow(player);
					}
				}
			});
			context.waitFor(mc -> ClientState.hud.total() >= 20, 200);
			context.runOnClient(mc -> mc.options.setCameraType(CameraType.THIRD_PERSON_FRONT));
			context.waitTicks(60);
			context.takeScreenshot("05_squad");

			// throwing
			context.runOnClient(mc -> mc.options.setCameraType(CameraType.THIRD_PERSON_BACK));
			server.runCommand("tp @p ~ ~ ~ 0 20");
			context.waitTicks(10);
			server.runOnServer(s -> Squads.throwPikmin(player(s)));
			context.waitTicks(7);
			context.takeScreenshot("06_throw");
			context.waitTicks(40);

			// a fight: the enemy's health shows, and its body is carried home
			server.runOnServer(s -> {
				ServerPlayer player = player(s);
				Mob zombie = net.minecraft.world.entity.EntityTypes.ZOMBIE.create(player.level(), EntitySpawnReason.COMMAND);
				zombie.snapTo(player.getX(), player.getY(), player.getZ() + 6, 180.0F, 0.0F);
				zombie.setNoAi(true);
				player.level().addFreshEntity(zombie);
			});
			server.runCommand("tp @p ~ ~ ~ facing entity @e[type=minecraft:zombie,limit=1] feet");
			context.waitTicks(5);
			server.runOnServer(s -> Squads.charge(player(s)));
			context.waitFor(mc -> !ClientState.ENEMIES.isEmpty(), 300);
			context.waitTicks(10);
			context.takeScreenshot("07_fight");
			server.waitFor(s -> !entities(s, CorpseEntity.class).isEmpty(), 1200);
			context.waitTicks(80);
			context.takeScreenshot("08_carry_body");
			server.waitFor(s -> entities(s, CorpseEntity.class).isEmpty(), 1200);

			// the pellet posy: hit right in the middle and the pellet drops
			server.runCommand("tp @p ~ ~ ~ 0 0");
			server.runOnServer(s -> {
				ServerPlayer player = player(s);
				PelletPosyEntity posy = ModEntities.PELLET_POSY.create(player.level(), EntitySpawnReason.COMMAND);
				posy.snapTo(player.getX(), player.getY(), player.getZ() + 2.5, 180.0F, 0.0F);
				player.level().addFreshEntity(posy);
			});
			context.waitTicks(20);
			context.runOnClient(mc -> mc.options.setCameraType(CameraType.FIRST_PERSON));
			server.runOnServer(s -> {
				ServerPlayer player = player(s);
				PelletPosyEntity posy = entities(s, PelletPosyEntity.class).getFirst();
				Vec3 middle = posy.position().add(0.0, posy.getBbHeight() * 0.82, 0.0);
				player.lookAt(net.minecraft.commands.arguments.EntityAnchorArgument.Anchor.EYES, middle);
			});
			context.waitTicks(5);
			context.takeScreenshot("09_pellet_posy");
			server.runOnServer(s -> player(s).attack(entities(s, PelletPosyEntity.class).getFirst()));
			server.waitFor(s -> !entities(s, PelletEntity.class).isEmpty(), 100);
			server.runOnServer(s -> Squads.dismiss(player(s)));
			context.runOnClient(mc -> mc.options.setCameraType(CameraType.THIRD_PERSON_BACK));
			context.waitTicks(60);
			context.takeScreenshot("10_carry_pellet");

			// a nectar egg breaks into nectar, and Pikmin drink it
			server.runOnServer(s -> {
				ServerPlayer player = player(s);
				Mob egg = ModEntities.NECTAR_EGG.create(player.level(), EntitySpawnReason.COMMAND);
				egg.snapTo(player.getX() + 2, player.getY(), player.getZ() + 3, 0.0F, 0.0F);
				player.level().addFreshEntity(egg);
			});
			context.waitTicks(20);
			context.takeScreenshot("11_nectar_egg");
			server.runCommand("kill @e[type=pikcraft:nectar_egg]");
			server.waitFor(s -> !entities(s, NectarEntity.class).isEmpty(), 100);
			context.waitTicks(50);
			context.takeScreenshot("12_nectar");

			// the menus
			server.runOnServer(s -> onion(s).openMenu(player(s)));
			context.waitForScreen(OnionScreen.class);
			context.waitTicks(5);
			context.takeScreenshot("13_onion_menu");
			context.setScreen(() -> null);
			context.setScreen(() -> new InventoryScreen(net.minecraft.client.Minecraft.getInstance().player));
			context.waitTicks(5);
			context.takeScreenshot("14_inventory");
			context.setScreen(() -> null);

			// the health wheel
			server.runCommand("damage @p 9 minecraft:generic");
			context.runOnClient(mc -> mc.options.setCameraType(CameraType.FIRST_PERSON));
			context.waitTicks(30);
			context.takeScreenshot("15_health_wheel");

			// night: Onions fly up and stop making Pikmin
			server.runCommand("effect give @p minecraft:instant_health 1 5");
			server.runCommand("time set 14000");
			server.waitFor(s -> onion(s).lift() > 15.0F, 600);
			server.runCommand("tp @p ~ ~ ~ facing entity @e[type=pikcraft:onion,limit=1] feet");
			context.waitTicks(20);
			context.takeScreenshot("16_night_onion");
			context.runOnClient(mc -> mc.options.setCameraType(CameraType.FIRST_PERSON));
		}
	}

	/** Normal multiplayer: a dedicated server gives a joining player their Onion and syncs their squad. */
	private void dedicatedServer(ClientGameTestContext context) {
		try (TestDedicatedServerContext server = context.worldBuilder().createServer();
			TestDedicatedServerConnection connection = server.connect()) {
			connection.waitForChunksRender();
			server.runCommand("op @a");
			server.runCommand("gamemode survival @a");
			server.runCommand("time set 1000");
			context.waitFor(mc -> CutsceneCamera.isPlaying(), 400);
			context.waitFor(mc -> !CutsceneCamera.isPlaying(), 400);
			server.runOnServer(s -> {
				ServerPlayer player = player(s);
				onion(s).awaken(player);
				for (int i = 0; i < 6; i++) {
					PikminEntity pikmin = ModEntities.pikmin(PikminType.byId(i)).create(player.level(), EntitySpawnReason.COMMAND);
					pikmin.snapTo(player.getX() + i - 3, player.getY(), player.getZ() + 2, 0.0F, 0.0F);
					player.level().addFreshEntity(pikmin);
					pikmin.follow(player);
				}
			});
			context.waitFor(mc -> ClientState.hud.total() == 6, 200);
			context.waitFor(mc -> !CutsceneCamera.isPlaying(), 400);
			context.runOnClient(mc -> mc.options.setCameraType(CameraType.THIRD_PERSON_BACK));
			context.waitTicks(40);
			context.takeScreenshot("17_dedicated_server");
			context.runOnClient(mc -> mc.options.setCameraType(CameraType.FIRST_PERSON));
		}
	}

	private static ServerPlayer player(MinecraftServer server) {
		return server.getPlayerList().getPlayers().getFirst();
	}

	private static ServerLevel level(MinecraftServer server) {
		return player(server).level();
	}

	private static <T extends Entity> List<T> entities(MinecraftServer server, Class<T> type) {
		return level(server).getEntitiesOfClass(type, new AABB(player(server).blockPosition()).inflate(64.0), Entity::isAlive);
	}

	private static OnionEntity onion(MinecraftServer server) {
		return entities(server, OnionEntity.class).getFirst();
	}

	private static List<PikminEntity> pikmin(MinecraftServer server, java.util.function.Predicate<PikminEntity> filter) {
		return entities(server, PikminEntity.class).stream().filter(filter).toList();
	}
}
