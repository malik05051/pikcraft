#!/bin/bash
# Rebuilds pikcraft_patcher/data/<version>.dolpatch from the PikCraft game code.
#
# Needs your own Pikmin disc image (the decompilation reads the original executable from it), Python 3,
# ninja and network access for the decompilation's tools.
#
#   mod/build_dolpatch.sh /path/to/Pikmin.iso [GPIP01_00]
set -euo pipefail
ISO=$(realpath "$1")
VERSION=${2:-GPIP01_00}
HERE=$(cd "$(dirname "$0")" && pwd)
WORK=${WORK:-$HERE/../build/decomp}
DECOMP_COMMIT=9a21636   # projectPiki/pikmin revision the hooks apply to

if [ ! -d "$WORK" ]; then
  git clone https://github.com/projectPiki/pikmin "$WORK"
fi
cd "$WORK"
git checkout -q -f "$DECOMP_COMMIT"
git apply "$HERE/decomp-hooks.patch"
cp "$HERE/PikCraft.h" include/PikCraft.h
cp "$HERE/pikcraft.inl" src/plugPikiColin/pikcraft.inl
ln -sf "$ISO" "orig/$VERSION/disc.iso"

python3 configure.py --version "$VERSION"
ninja || true   # the final checksum step fails by design: the executable is modified
test -f "build/$VERSION/main.dol"

cd "$HERE/.."
python3 -m pikcraft_patcher.dolpatch "$ISO" "$WORK/build/$VERSION/main.dol" "pikcraft_patcher/data/$VERSION.dolpatch"
