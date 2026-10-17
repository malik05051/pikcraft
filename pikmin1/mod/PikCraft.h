#ifndef _PIKCRAFT_H
#define _PIKCRAFT_H

#include "types.h"

class BaseShape;
class CollGroup;
class Graphics;
class RandomAccessStream;
class Vector3f;

/**
 * @brief PikCraft: Minecraft-style block terrain.
 *
 * The map model carries a block heightmap chunk (written by the PikCraft patcher). When it is
 * present, the blocks are drawn instead of the original terrain, map collision queries are answered
 * from the heightmap, and the player can break (D-pad left) and place (D-pad right) blocks at the cursor.
 */
namespace PikCraft {
enum {
	ChunkID = 0x5043, // 'PC'
};

void readChunk(BaseShape* shape, RandomAccessStream& stream, u32 length);
void init(BaseShape* mapShape);
void update();
void draw(Graphics& gfx);
CollGroup* getCollTris(immut Vector3f& pos);

/// Map shape whose collision is answered from the blocks (null when PikCraft is inactive).
extern BaseShape* sCollShape;
} // namespace PikCraft

#endif
