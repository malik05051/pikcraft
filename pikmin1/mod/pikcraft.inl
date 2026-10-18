// PikCraft block terrain: included at the end of mapMgr.cpp.
//
// The patcher stores a block heightmap in the map model (chunk 'PC'). Each column has a ground layer
// (its top solid block), an optional water layer above it, the type of its top block and the
// original map code (surface attribute) of the terrain there.
//
// - Drawing: visible faces of each column are drawn straight from the heightmap every frame, so a
//   broken or placed block shows up immediately.
// - Collision: the map's collision queries (BaseShape::getCollTris) are answered with triangles
//   spanning the column centres. They are built on demand into a small cache that stands in for the
//   map model's own triangle and vertex lists, so all of the game's movement code works unchanged.
// - Breaking and placing: D-pad left breaks the top block under the cursor, D-pad right places one.

#include "Dolphin/gx.h"
#include "Dolphin/os.h"
#include "Kontroller.h"
#include "Navi.h"
#include "NaviMgr.h"
#include "RumbleMgr.h"
#include "SoundID.h"

namespace PikCraft {

enum BlockType {
	Block_Air    = 0,
	Block_Grass  = 1,
	Block_Dirt   = 2,
	Block_Stone  = 3,
	Block_Sand   = 4,
	Block_Log    = 5,
	Block_Planks = 6,
	Block_Leaves = 7,
	Block_Water  = 8,
	Block_Mud    = 9,
};

// Atlas tiles (4x4 grid of 16x16 textures). Every block uses the tile with its own index, except
// grass, whose top and sides differ.
enum AtlasTile {
	Tile_GrassTop  = 0,
	Tile_GrassSide = 1,
	Tile_Dirt      = 2,
};

static const s16 NO_GROUND       = -32768;
static const u32 CHUNK_MAGIC     = 0x50434231; // 'PCB1'
static const int CACHE_SLOTS     = 1024;       // power of two
static const int GROUP_RING      = 8;          // collision groups can be requested while others are in use
static const int MAX_GROUP_TRIS  = 512;
static const f32 COLL_REACH      = 64.0f; // same reach as the map's own collision grid cells
static const int BUILD_HEADROOM  = 8;     // how many blocks above the highest terrain can be placed
static const int MIN_DEPTH_ABOVE = 1;     // the lowest layer can't be broken

struct ChunkHeader {
	u32 mMagic;
	u32 mVersion;
	f32 mOriginX;
	f32 mOriginY;
	f32 mOriginZ;
	f32 mBlockSize;
	s32 mSizeX;
	s32 mSizeZ;
	s32 mFloor;
	s32 mTop;
	u32 mGroundOffset;
	u32 mWaterOffset;
	u32 mSurfaceOffset;
	u32 mCodeOffset;
	u32 mAtlasOffset;
	u32 mAtlasSize;
};

struct World {
	BaseShape* mShape;
	f32 mOriginX;
	f32 mOriginY;
	f32 mOriginZ;
	f32 mSize;
	int mSizeX;
	int mSizeZ;
	int mFloor;
	int mTop;

	s16* mGround;           // current top solid layer per column
	u8* mSurface;           // current top block type per column
	immut s16* mOrigGround; // as loaded
	immut u8* mOrigSurface; // as loaded
	immut s16* mWater;      // water surface layer (or NO_GROUND)
	immut u8* mCode;        // map attribute (bits 0-2), slip code (3-4), bald flag (5)

	GXTexObj mAtlas;
	u8 mHeld;    // block type placed by D-pad right (the last one broken)
	int mTarget; // column under the cursor, or -1

	s32* mCacheKey;
	Vector3f* mCacheVerts;
	CollTriInfo* mCacheTris;
	CollGroup mGroups[GROUP_RING];
	CollTriInfo* mGroupTris[GROUP_RING][MAX_GROUP_TRIS];
	int mNextGroup;
};

BaseShape* sCollShape;
static World* sWorld;
static u8* sChunk;
static u32 sChunkSize;
static BaseShape* sChunkShape;

/**
 * @brief Keeps the block chunk of a model being read (only map models have one).
 */
void readChunk(BaseShape* shape, RandomAccessStream& stream, u32 length)
{
	sChunk = new (0x20) u8[length];
	stream.read(sChunk, length);
	sChunkSize  = length;
	sChunkShape = shape;
}

static inline int floorToInt(f32 v)
{
	int i = (int)v;
	return (v < i) ? i - 1 : i;
}

static inline int colIndex(World* w, int i, int k) { return k * w->mSizeX + i; }

static inline bool inGrid(World* w, int i, int k) { return i >= 0 && k >= 0 && i < w->mSizeX && k < w->mSizeZ; }

static inline f32 layerTop(World* w, int layer) { return w->mOriginY + (layer + 1) * w->mSize; }

/**
 * @brief Block type at a layer below or at the top of a column.
 */
static u8 blockAt(World* w, int col, int y)
{
	int ground = w->mGround[col];
	if (y > ground) {
		return (w->mWater[col] != NO_GROUND && y <= w->mWater[col]) ? Block_Water : Block_Air;
	}
	int orig = w->mOrigGround[col];
	if (y == ground || y > orig) {
		// the top block, or a stack of placed blocks
		return w->mSurface[col];
	}
	u8 origSurface = w->mOrigSurface[col];
	if (y == orig) {
		return origSurface;
	}
	if (origSurface == Block_Stone || origSurface == Block_Planks) {
		return origSurface;
	}
	return (orig - y < 3) ? Block_Dirt : Block_Stone;
}

static int tileFor(u8 block, int face) // face: 0 top, 1 side, 2 bottom
{
	if (block == Block_Grass) {
		return (face == 0) ? Tile_GrassTop : (face == 1) ? Tile_GrassSide : Tile_Dirt;
	}
	return block;
}

/**
 * @brief Sets up the block world if the map model carries a block chunk.
 */
void init(BaseShape* mapShape)
{
	sWorld      = nullptr;
	sCollShape  = nullptr;
	if (!sChunk || sChunkShape != mapShape) {
		return;
	}
	ChunkHeader* hdr = (ChunkHeader*)sChunk;
	if (hdr->mMagic != CHUNK_MAGIC) {
		return;
	}

	World* w    = new World;
	w->mShape   = mapShape;
	w->mOriginX = hdr->mOriginX;
	w->mOriginY = hdr->mOriginY;
	w->mOriginZ = hdr->mOriginZ;
	w->mSize    = hdr->mBlockSize;
	w->mSizeX   = hdr->mSizeX;
	w->mSizeZ   = hdr->mSizeZ;
	w->mFloor   = hdr->mFloor;
	w->mTop     = hdr->mTop;

	int count       = w->mSizeX * w->mSizeZ;
	w->mOrigGround  = (immut s16*)(sChunk + hdr->mGroundOffset);
	w->mWater       = (immut s16*)(sChunk + hdr->mWaterOffset);
	w->mOrigSurface = sChunk + hdr->mSurfaceOffset;
	w->mCode        = sChunk + hdr->mCodeOffset;

	// work on copies, so every visit to the map starts from the original terrain
	w->mGround  = new s16[count];
	w->mSurface = new u8[count];
	memcpy(w->mGround, w->mOrigGround, count * sizeof(s16));
	memcpy(w->mSurface, w->mOrigSurface, count);

	u8* atlas = sChunk + hdr->mAtlasOffset;
	DCFlushRange(atlas, hdr->mAtlasSize);
	GXInitTexObj(&w->mAtlas, atlas, 64, 64, GX_TF_RGB5A3, GX_CLAMP, GX_CLAMP, GX_FALSE);
	GXInitTexObjLOD(&w->mAtlas, GX_NEAR, GX_NEAR, 0.0f, 0.0f, 0.0f, GX_FALSE, GX_FALSE, GX_ANISO_1);

	w->mHeld   = Block_Dirt;
	w->mTarget = -1;

	w->mCacheKey   = new s32[CACHE_SLOTS];
	w->mCacheVerts = new Vector3f[CACHE_SLOTS * 4];
	w->mCacheTris  = new CollTriInfo[CACHE_SLOTS * 2];
	for (int i = 0; i < CACHE_SLOTS; i++) {
		w->mCacheKey[i] = -1;
	}
	w->mNextGroup = 0;

	// the movement code indexes the map model's own triangle and vertex lists: point them at the cache
	mapShape->mTriList    = w->mCacheTris;
	mapShape->mTriCount   = CACHE_SLOTS * 2;
	mapShape->mVertexList = w->mCacheVerts;

	sWorld     = w;
	sCollShape = mapShape;
	OSReport("PikCraft: %d x %d columns of %.1f, free %d\n", w->mSizeX, w->mSizeZ, w->mSize, gsys->getHeap(SYSHEAP_App)->getFree());
}

////////////////////////////////////////////////////////////////////////////// collision

static inline int cellSlot(World* w, int i, int k)
{
	if (i < 0 || k < 0 || i >= w->mSizeX - 1 || k >= w->mSizeZ - 1) {
		return -1;
	}
	return colIndex(w, i, k) & (CACHE_SLOTS - 1);
}

static f32 collisionHeight(World* w, int i, int k)
{
	int ground = w->mGround[colIndex(w, i, k)];
	if (ground == NO_GROUND) {
		// outside the terrain: a tall wall keeps everyone in
		ground = w->mTop + 12;
	}
	return layerTop(w, ground);
}

static void setUpTri(World* w, CollTriInfo& tri, int slot, int a, int b, int c, int col, int adj0, int adj1, int adj2)
{
	tri.mVertexIndices[0] = slot * 4 + a;
	tri.mVertexIndices[1] = slot * 4 + b;
	tri.mVertexIndices[2] = slot * 4 + c;

	immut Vector3f& v0 = w->mCacheVerts[tri.mVertexIndices[0]];
	immut Vector3f& v1 = w->mCacheVerts[tri.mVertexIndices[1]];
	immut Vector3f& v2 = w->mCacheVerts[tri.mVertexIndices[2]];
	// map triangles store their normal as (v2 - v0) x (v1 - v0)
	Vector3f n = v2 - v0;
	n.CP(v1 - v0);
	n.normalise();
	tri.mTriangle.mNormal = n;
	tri.mTriangle.mOffset = n.DP(v0);

	u8 code                    = w->mCode[col];
	tri.mMapCode               = ((code & 7) << 29) | (((code >> 3) & 3) << 27) | (((code >> 5) & 1) << 25);
	tri.mCollRoomIndex         = 0;
	tri.mAdjacentTriIndices[0] = adj0;
	tri.mAdjacentTriIndices[1] = adj1;
	tri.mAdjacentTriIndices[2] = adj2;
	tri.init(nullptr, w->mCacheVerts);
}

/**
 * @brief Collision triangles for the cell between column centres (i, k) and (i + 1, k + 1).
 */
static CollTriInfo* cellTris(World* w, int i, int k)
{
	int slot = cellSlot(w, i, k);
	int key  = colIndex(w, i, k);
	if (w->mCacheKey[slot] != key) {
		w->mCacheKey[slot] = key;
		f32 x0             = w->mOriginX + (i + 0.5f) * w->mSize;
		f32 z0             = w->mOriginZ + (k + 0.5f) * w->mSize;
		Vector3f* v        = &w->mCacheVerts[slot * 4];
		v[0].set(x0, collisionHeight(w, i, k), z0);                             // a
		v[1].set(x0 + w->mSize, collisionHeight(w, i + 1, k), z0);              // b
		v[2].set(x0, collisionHeight(w, i, k + 1), z0 + w->mSize);              // c
		v[3].set(x0 + w->mSize, collisionHeight(w, i + 1, k + 1), z0 + w->mSize); // d

		// neighbouring triangles across each edge (slot * 2 + 0 is (a, b, c), slot * 2 + 1 is (b, d, c))
		int north = cellSlot(w, i, k - 1);
		int west  = cellSlot(w, i - 1, k);
		int east  = cellSlot(w, i + 1, k);
		int south = cellSlot(w, i, k + 1);
		CollTriInfo* tris = &w->mCacheTris[slot * 2];
		setUpTri(w, tris[0], slot, 0, 1, 2, colIndex(w, i, k), north < 0 ? -1 : north * 2 + 1, slot * 2 + 1, west < 0 ? -1 : west * 2 + 1);
		setUpTri(w, tris[1], slot, 1, 3, 2, colIndex(w, i + 1, k + 1), east < 0 ? -1 : east * 2, south < 0 ? -1 : south * 2, slot * 2);
	}
	return &w->mCacheTris[slot * 2];
}

static void invalidateColumn(World* w, int i, int k)
{
	for (int dk = -1; dk <= 0; dk++) {
		for (int di = -1; di <= 0; di++) {
			int slot = cellSlot(w, i + di, k + dk);
			if (slot >= 0) {
				w->mCacheKey[slot] = -1;
			}
		}
	}
}

/**
 * @brief Map collision around a position, built from the heightmap.
 */
CollGroup* getCollTris(immut Vector3f& pos)
{
	World* w = sWorld;
	int ring = w->mNextGroup;
	w->mNextGroup = (ring + 1) % GROUP_RING;
	CollGroup* group   = &w->mGroups[ring];
	CollTriInfo** list = w->mGroupTris[ring];

	int ci    = floorToInt((pos.x - w->mOriginX) / w->mSize - 0.5f);
	int ck    = floorToInt((pos.z - w->mOriginZ) / w->mSize - 0.5f);
	int reach = int(COLL_REACH / w->mSize) + 1;
	int count = 0;
	for (int k = ck - reach; k <= ck + reach; k++) {
		for (int i = ci - reach; i <= ci + reach; i++) {
			if (cellSlot(w, i, k) < 0 || count + 2 > MAX_GROUP_TRIS) {
				continue;
			}
			CollTriInfo* tris = cellTris(w, i, k);
			list[count++]     = &tris[0];
			list[count++]     = &tris[1];
		}
	}

	group->mTriCount              = count;
	group->mFarCulledTriCount     = 0;
	group->mTriangleList          = list;
	group->mFarCulledTriDistances = nullptr;
	group->mModel                 = (Shape*)w->mShape;
	group->mVertexList            = w->mCacheVerts;
	group->mJointIndex            = 0;
	group->mPlatCollision         = nullptr;
	group->mNextCollGroup         = nullptr;
	return group;
}

////////////////////////////////////////////////////////////////////////////// breaking and placing

static void changed(World* w, int i, int k)
{
	invalidateColumn(w, i, k);
	Vector3f pos(w->mOriginX + (i + 0.5f) * w->mSize, layerTop(w, w->mGround[colIndex(w, i, k)]), w->mOriginZ + (k + 0.5f) * w->mSize);
	effectMgr->create(EffectMgr::EFF_SD_DirtCloud, pos, nullptr, nullptr);
	effectMgr->create(EffectMgr::EFF_Navi_PunchB, pos, nullptr, nullptr);
	rumbleMgr->start(RUMBLE_Unk2, 0, nullptr);
}

static void breakBlock(World* w, Navi* navi, int col)
{
	int ground = w->mGround[col];
	if (ground <= w->mFloor + MIN_DEPTH_ABOVE) {
		return;
	}
	u8 broken = w->mSurface[col];
	w->mHeld  = (broken == Block_Grass) ? Block_Dirt : broken;

	int below = ground - 1;
	int orig  = w->mOrigGround[col];
	if (below == orig) {
		w->mSurface[col] = w->mOrigSurface[col];
	} else if (below < orig) {
		u8 origSurface   = w->mOrigSurface[col];
		bool uniform     = origSurface == Block_Stone || origSurface == Block_Planks;
		w->mSurface[col] = uniform ? origSurface : (orig - below < 3) ? (u8)Block_Dirt : (u8)Block_Stone;
	} // above the original ground: a stack of placed blocks of the same type
	w->mGround[col] = below;
	changed(w, col % w->mSizeX, col / w->mSizeX);
	navi->playEventSound(navi, SE_PIKI_ATTACK_HIT);
}

static void placeBlock(World* w, Navi* navi, int col)
{
	int ground = w->mGround[col];
	if (ground >= w->mTop + BUILD_HEADROOM) {
		return;
	}
	w->mGround[col]  = ground + 1;
	w->mSurface[col] = w->mHeld;
	changed(w, col % w->mSizeX, col / w->mSizeX);
	navi->playEventSound(navi, SE_PIKI_ATTACK_HIT);
}

/**
 * @brief Tracks the column under the cursor and handles breaking and placing.
 */
void update()
{
	World* w = sWorld;
	if (!w) {
		return;
	}
	w->mTarget = -1;
	if (gameflow.mPauseAll || gameflow.mIsUIOverlayActive || (gameflow.mMoviePlayer && gameflow.mMoviePlayer->mIsActive)) {
		return;
	}
	Navi* navi = naviMgr->getNavi();
	if (!navi || !navi->isAlive()) {
		return;
	}

	// the cursor position is kept relative to the captain
	Vector3f cursor = navi->mSRT.t + navi->mCursorPosition;
	int i           = floorToInt((cursor.x - w->mOriginX) / w->mSize);
	int k           = floorToInt((cursor.z - w->mOriginZ) / w->mSize);
	if (!inGrid(w, i, k) || w->mGround[colIndex(w, i, k)] == NO_GROUND) {
		return;
	}
	int col    = colIndex(w, i, k);
	w->mTarget = col;

	if (navi->mKontroller->keyClick(KBBTN_DPAD_LEFT)) {
		breakBlock(w, navi, col);
	} else if (navi->mKontroller->keyClick(KBBTN_DPAD_RIGHT)) {
		placeBlock(w, navi, col);
	}
}

////////////////////////////////////////////////////////////////////////////// drawing

static f32 sTileU;
static f32 sTileV;

// Minecraft-style face shading, on top of the game's own lighting
static const u32 SHADE_TOP = 0xFFFFFFFF;
static const u32 SHADE_X   = 0xC0C0C0FF;
static const u32 SHADE_Z   = 0xDCDCDCFF;
static const GXColor AMBIENT = { 150, 150, 150, 255 }; // keeps sides facing away from the sun readable

static inline void setTile(int tile)
{
	sTileU = (tile & 3) * 0.25f;
	sTileV = (tile >> 2) * 0.25f;
}

static inline void vertex(f32 x, f32 y, f32 z, immut Vector3f& n, u32 shade, f32 u, f32 v)
{
	GXPosition3f32(x, y, z);
	GXNormal3f32(n.x, n.y, n.z);
	GXColor1u32(shade);
	GXTexCoord2f32(sTileU + u * 0.25f, sTileV + v * 0.25f);
}

/**
 * @brief Draws a quad given counter-clockwise (seen from outside) corners; p1 and p2 are the top
 * edge for side faces. GX treats clockwise as front-facing, so the corners go out reversed.
 */
static inline void quad(immut Vector3f& p0, immut Vector3f& p1, immut Vector3f& p2, immut Vector3f& p3, immut Vector3f& n, u32 shade)
{
	GXBegin(GX_QUADS, GX_VTXFMT0, 4);
	vertex(p0.x, p0.y, p0.z, n, shade, 0.0f, 1.0f);
	vertex(p3.x, p3.y, p3.z, n, shade, 1.0f, 1.0f);
	vertex(p2.x, p2.y, p2.z, n, shade, 1.0f, 0.0f);
	vertex(p1.x, p1.y, p1.z, n, shade, 0.0f, 0.0f);
	GXEnd();
}

static const Vector3f UP(0.0f, 1.0f, 0.0f);
static const Vector3f EAST(1.0f, 0.0f, 0.0f);
static const Vector3f WEST(-1.0f, 0.0f, 0.0f);
static const Vector3f SOUTH(0.0f, 0.0f, 1.0f);
static const Vector3f NORTH(0.0f, 0.0f, -1.0f);

/**
 * @brief Draws one column's visible faces: solid blocks (water = false) or its water (water = true).
 */
static void drawColumn(World* w, int i, int k, bool water)
{
	int col    = colIndex(w, i, k);
	int ground = w->mGround[col];
	if (ground == NO_GROUND) {
		return;
	}
	int waterTop = w->mWater[col];
	bool wet     = waterTop != NO_GROUND && waterTop > ground;
	if (water && !wet) {
		return;
	}

	f32 s  = w->mSize;
	f32 x0 = w->mOriginX + i * s;
	f32 z0 = w->mOriginZ + k * s;
	f32 x1 = x0 + s;
	f32 z1 = z0 + s;

	int from = water ? ground + 1 : w->mFloor;
	int top  = water ? waterTop : ground;

	// top face
	f32 yTop = layerTop(w, top);
	setTile(tileFor(water ? (u8)Block_Water : w->mSurface[col], 0));
	quad(Vector3f(x0, yTop, z0), Vector3f(x0, yTop, z1), Vector3f(x1, yTop, z1), Vector3f(x1, yTop, z0), UP, SHADE_TOP);

	// side faces, one per exposed layer
	static const int dirs[4][2] = { { 1, 0 }, { -1, 0 }, { 0, 1 }, { 0, -1 } };
	for (int d = 0; d < 4; d++) {
		int ni = i + dirs[d][0];
		int nk = k + dirs[d][1];
		if (!inGrid(w, ni, nk)) {
			continue;
		}
		int ncol    = colIndex(w, ni, nk);
		int nground = w->mGround[ncol];
		if (nground == NO_GROUND) {
			continue; // edge of the terrain, never seen
		}
		int exposedFrom = nground + 1;
		if (water) {
			int nwater = w->mWater[ncol];
			if (nwater != NO_GROUND && nwater > exposedFrom - 1) {
				exposedFrom = nwater + 1;
			}
		}
		if (exposedFrom < from) {
			exposedFrom = from;
		}
		for (int y = exposedFrom; y <= top; y++) {
			f32 yb = layerTop(w, y - 1);
			f32 yt = yb + s;
			setTile(tileFor(water ? (u8)Block_Water : blockAt(w, col, y), 1));
			switch (d) {
			case 0:
				quad(Vector3f(x1, yb, z0), Vector3f(x1, yt, z0), Vector3f(x1, yt, z1), Vector3f(x1, yb, z1), EAST, SHADE_X);
				break;
			case 1:
				quad(Vector3f(x0, yb, z1), Vector3f(x0, yt, z1), Vector3f(x0, yt, z0), Vector3f(x0, yb, z0), WEST, SHADE_X);
				break;
			case 2:
				quad(Vector3f(x1, yb, z1), Vector3f(x1, yt, z1), Vector3f(x0, yt, z1), Vector3f(x0, yb, z1), SOUTH, SHADE_Z);
				break;
			case 3:
				quad(Vector3f(x0, yb, z0), Vector3f(x0, yt, z0), Vector3f(x1, yt, z0), Vector3f(x1, yb, z0), NORTH, SHADE_Z);
				break;
			}
		}
	}
}

static void drawPass(World* w, Graphics& gfx, bool water)
{
	static const int CHUNK = 8;
	f32 s = w->mSize;
	for (int ck = 0; ck < w->mSizeZ; ck += CHUNK) {
		for (int ci = 0; ci < w->mSizeX; ci += CHUNK) {
			BoundBox box;
			box.mMin.set(w->mOriginX + ci * s, layerTop(w, w->mFloor - 1), w->mOriginZ + ck * s);
			box.mMax.set(w->mOriginX + (ci + CHUNK) * s, layerTop(w, w->mTop + BUILD_HEADROOM), w->mOriginZ + (ck + CHUNK) * s);
			if (!gfx.mCamera->isBoundVisible(box, 0x8000 | 0x10 | 0x20 | 0x1 | 0x2 | 0x4 | 0x8)) {
				continue;
			}
			for (int k = ck; k < ck + CHUNK && k < w->mSizeZ; k++) {
				for (int i = ci; i < ci + CHUNK && i < w->mSizeX; i++) {
					drawColumn(w, i, k, water);
				}
			}
		}
	}
}

static void drawTarget(World* w, Graphics& gfx)
{
	if (w->mTarget < 0) {
		return;
	}
	int i  = w->mTarget % w->mSizeX;
	int k  = w->mTarget / w->mSizeX;
	f32 s  = w->mSize;
	f32 x0 = w->mOriginX + i * s;
	f32 z0 = w->mOriginZ + k * s;
	f32 y  = layerTop(w, w->mGround[w->mTarget]) + 0.5f;

	gfx.setLighting(false, nullptr);
	GXSetNumTexGens(0);
	GXSetNumTevStages(1);
	GXSetTevOrder(GX_TEVSTAGE0, GX_TEXCOORD_NULL, GX_TEXMAP_NULL, GX_COLOR0A0);
	GXSetTevOp(GX_TEVSTAGE0, GX_PASSCLR);
	GXSetChanCtrl(GX_COLOR0A0, GX_FALSE, GX_SRC_REG, GX_SRC_VTX, 0, GX_DF_NONE, GX_AF_NONE);
	GXSetBlendMode(GX_BM_BLEND, GX_BL_SRCALPHA, GX_BL_INVSRCALPHA, GX_LO_CLEAR);
	GXSetZMode(GX_TRUE, GX_LEQUAL, GX_FALSE);
	GXSetLineWidth(18, GX_TO_ZERO);

	GXClearVtxDesc();
	GXSetVtxDesc(GX_VA_POS, GX_DIRECT);
	GXSetVtxDesc(GX_VA_CLR0, GX_DIRECT);
	GXSetVtxAttrFmt(GX_VTXFMT0, GX_VA_POS, GX_POS_XYZ, GX_F32, 0);
	GXSetVtxAttrFmt(GX_VTXFMT0, GX_VA_CLR0, GX_CLR_RGBA, GX_RGBA8, 0);
	u32 colour = 0xFFFFFFC0;
	GXBegin(GX_LINESTRIP, GX_VTXFMT0, 5);
	GXPosition3f32(x0, y, z0);
	GXColor1u32(colour);
	GXPosition3f32(x0 + s, y, z0);
	GXColor1u32(colour);
	GXPosition3f32(x0 + s, y, z0 + s);
	GXColor1u32(colour);
	GXPosition3f32(x0, y, z0 + s);
	GXColor1u32(colour);
	GXPosition3f32(x0, y, z0);
	GXColor1u32(colour);
	GXEnd();
}

/**
 * @brief Draws the block terrain (called right after the map model is drawn).
 */
void draw(Graphics& gfx)
{
	World* w = sWorld;
	if (!w) {
		return;
	}

	Matrix4f viewMtx;
	gfx.calcViewMatrix(Matrix4f::ident, viewMtx);
	gfx.useMatrix(viewMtx, 0);
	// the game's lights, with the material colour taken from the vertex (face shading)
	PVWLightingInfo lighting;
	lighting.mCtrlFlag = LightingControlFlags::EnableColor0 | (GX_DF_CLAMP << LightingControlFlags::DiffFnColor0Shift)
	                   | LightingControlFlags::MatSrcColor0Vtx | LightingControlFlags::MatSrcAlpha0Vtx;
	gfx.setLighting(true, &lighting);
	GXSetChanAmbColor(GX_COLOR0A0, AMBIENT);

	gfx.useTexture(nullptr, 0);
	GXLoadTexObj(&w->mAtlas, GX_TEXMAP0);
	GXSetNumTexGens(1);
	GXSetTexCoordGen2(GX_TEXCOORD0, GX_TG_MTX2X4, GX_TG_TEX0, GX_IDENTITY, GX_FALSE, GX_PTIDENTITY);
	GXSetNumIndStages(0);
	GXSetTevDirect(GX_TEVSTAGE0);
	GXSetNumTevStages(1);
	GXSetTevOrder(GX_TEVSTAGE0, GX_TEXCOORD0, GX_TEXMAP0, GX_COLOR0A0);
	GXSetTevOp(GX_TEVSTAGE0, GX_MODULATE);
	GXSetCullMode(GX_CULL_BACK);

	GXClearVtxDesc();
	GXSetVtxDesc(GX_VA_POS, GX_DIRECT);
	GXSetVtxDesc(GX_VA_NRM, GX_DIRECT);
	GXSetVtxDesc(GX_VA_CLR0, GX_DIRECT);
	GXSetVtxDesc(GX_VA_TEX0, GX_DIRECT);
	GXSetVtxAttrFmt(GX_VTXFMT0, GX_VA_POS, GX_POS_XYZ, GX_F32, 0);
	GXSetVtxAttrFmt(GX_VTXFMT0, GX_VA_NRM, GX_NRM_XYZ, GX_F32, 0);
	GXSetVtxAttrFmt(GX_VTXFMT0, GX_VA_CLR0, GX_CLR_RGBA, GX_RGBA8, 0);
	GXSetVtxAttrFmt(GX_VTXFMT0, GX_VA_TEX0, GX_TEX_ST, GX_F32, 0);

	// solid blocks
	GXSetBlendMode(GX_BM_NONE, GX_BL_ONE, GX_BL_ZERO, GX_LO_COPY);
	GXSetAlphaCompare(GX_ALWAYS, 0, GX_AOP_OR, GX_ALWAYS, 0);
	GXSetZMode(GX_TRUE, GX_LEQUAL, GX_TRUE);
	drawPass(w, gfx, false);

	// water, see-through
	GXSetBlendMode(GX_BM_BLEND, GX_BL_SRCALPHA, GX_BL_INVSRCALPHA, GX_LO_CLEAR);
	GXSetZMode(GX_TRUE, GX_LEQUAL, GX_FALSE);
	drawPass(w, gfx, true);

	drawTarget(w, gfx);

	// hand the pipeline back in the state the game's materials expect
	GXSetBlendMode(GX_BM_NONE, GX_BL_ONE, GX_BL_ZERO, GX_LO_COPY);
	GXSetZMode(GX_TRUE, GX_LEQUAL, GX_TRUE);
	gfx.setLighting(true, nullptr);
	gfx.setAmbient();
	gfx.useTexture(nullptr, 0);
}

} // namespace PikCraft
