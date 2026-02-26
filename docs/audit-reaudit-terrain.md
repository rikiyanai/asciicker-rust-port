# Terrain.cpp Audit Findings

## Source File
`/Users/r/Downloads/asciicker-Y9-2/terrain.cpp`

---

## 1. SAH Cost Threshold

**Status: NOT FOUND**

There is **no Surface Area Heuristic (SAH)** implementation in `terrain.cpp`. The quadtree in this codebase does not use SAH for split decisions.

The only references to "split" in the file are:
- Line 832: Comment describing tree insertion logic ("Tree insertion -- parent split, quadrant calculation")
- Line 2119: Comment about triangle diagonal splits ("Each terrain cell is split into 2 triangles")

The quadtree uses a simple "grow upward" expansion strategy rather than cost-based splitting. Tree expansion occurs when coordinates fall outside current bounds, not based on any cost heuristic.

---

## 2. Neighbor Flag Mapping

**Location: Lines 38-48 (documentation), Lines 617-644 (UpdateNodes), Lines 1106-1171 (AddTerrainPatch), Lines 742-761 (DelTerrainPatch)**

### Documentation (Lines 38-48)
```cpp
 * ## Neighbor Flags Layout
 * ```
 *     bit7  bit0  bit1
 *       \    |    /
 *        +-------+
 *   bit6-|   P   |-bit2
 *        +-------+
 *       /    |    \
 *     bit5  bit4  bit3
 * ```
```

### UpdateNodes Function (Lines 617-644)
```cpp
static void UpdateNodes(Patch* p)
{
    QuadItem* q = p;
    Node* n = p->parent;

    while (n)
    {
        int lo = 0xffff;
        int hi = 0x0000;
        int fl = 0xFF;  // <-- Initialize to all bits set

        for (int i = 0; i < 4; i++)
        {
            if (n->quad[i])
            {
                lo = n->quad[i]->lo < lo ? n->quad[i]->lo : lo;
                hi = n->quad[i]->hi > hi ? n->quad[i]->hi : hi;
                fl = fl & n->quad[i]->flags;  // <-- AND operation: only bits present in ALL children
            }
        }

        n->lo = lo;
        n->hi = hi;
        n->flags = fl;  // <-- Flags are intersection (AND) of all children's flags

        n = n->parent;
    }
}
```

### Flag Setting in AddTerrainPatch (Lines 1106-1125)
```cpp
Patch* np[8] =
{
    GetTerrainPatch(t, nx - 1, ny - 1),  // bit 0
    GetTerrainPatch(t, nx,     ny - 1),  // bit 1
    GetTerrainPatch(t, nx + 1, ny - 1),  // bit 2
    GetTerrainPatch(t, nx + 1, ny),      // bit 3
    GetTerrainPatch(t, nx + 1, ny + 1),  // bit 4
    GetTerrainPatch(t, nx,     ny + 1),  // bit 5
    GetTerrainPatch(t, nx - 1, ny + 1),  // bit 6
    GetTerrainPatch(t, nx - 1, ny),      // bit 7
};

for (int i = 0; i < 8; i++)
{
    if (np[i])
    {
        int f = np[i]->flags;
        int j = (i + 4) & 7;  // <-- Opposite direction (add 4 = 180 degrees)
        np[i]->flags |= 1 << j;   // Set neighbor's flag pointing to self
        p->flags |= 1 << i;       // Set self's flag pointing to neighbor
        // ...
    }
}
```

### Flag Clearing in DelTerrainPatch (Lines 742-761)
```cpp
Patch* np[8] =
{
    flags & 0x01 ? GetTerrainPatch(t, x - 1, y - 1) : 0,  // bit 0: (-1,-1)
    flags & 0x02 ? GetTerrainPatch(t, x,     y - 1) : 0,  // bit 1: ( 0,-1)
    flags & 0x04 ? GetTerrainPatch(t, x + 1, y - 1) : 0,  // bit 2: (+1,-1)
    flags & 0x08 ? GetTerrainPatch(t, x + 1, y)     : 0,  // bit 3: (+1, 0)
    flags & 0x10 ? GetTerrainPatch(t, x + 1, y + 1) : 0,  // bit 4: (+1,+1)
    flags & 0x20 ? GetTerrainPatch(t, x,     y + 1) : 0,  // bit 5: ( 0,+1)
    flags & 0x40 ? GetTerrainPatch(t, x - 1, y + 1) : 0,  // bit 6: (-1,+1)
    flags & 0x80 ? GetTerrainPatch(t, x - 1, y)     : 0,  // bit 7: (-1, 0)
};

for (int i = 0; i < 8; i++)
{
    if (np[i])
    {
        int j = (i + 4) & 7;  // <-- Opposite direction
        np[i]->flags &= ~(1 << j);  // Clear neighbor's flag pointing to deleted patch
    }
}
```

---

## 3. Height Interpolation Formula

**Location: Lines 1630-1691**

The `QueryTerrainSample` function implements **piecewise bilinear interpolation** based on triangle diagonal orientation.

### Core Formula (Lines 1630-1685)
```cpp
void QueryTerrainSample(Patch* p, int x, int y, void(*cb)(Patch* p, int u, int v, double coords[3], void* cookie), void* cookie)
{
    static const double sxy = (double)VISUAL_CELLS / (double)HEIGHT_CELLS;

    for (int v = 0; v < VISUAL_CELLS; v++)
    {
        double fv = (2 * v + 1) * HEIGHT_CELLS / (2.0 * VISUAL_CELLS);
        int hy = (int)floor(fv);
        fv -= hy;  // <-- Fractional part in [0,1)

        for (int u = 0; u < VISUAL_CELLS; u++)
        {
            double fu = (2 * u + 1) * HEIGHT_CELLS / (2.0 * VISUAL_CELLS);
            int hx = (int)floor(fu);
            fu -= hx;  // <-- Fractional part in [0,1)

            double h;
            bool rot = p->diag & (1 << (hx + hy * HEIGHT_CELLS));

            // Diagonal orientation determines which triangle vertices to use
            if (rot)  // Diagonal from bottom-left to top-right
            {
                if (u < VISUAL_CELLS - v)
                {
                    // v[2], v[0], v[1] - lower-left triangle
                    h = p->height[hy][hx] +
                        fu * (p->height[hy][hx + 1] - p->height[hy][hx]) +
                        fv * (p->height[hy + 1][hx] - p->height[hy][hx]);
                }
                else
                {
                    // v[2], v[1], v[3] - upper-right triangle
                    h = p->height[hy + 1][hx + 1] +
                        (1 - fu) * (p->height[hy + 1][hx] - p->height[hy + 1][hx + 1]) +
                        (1 - fv) * (p->height[hy][hx + 1] - p->height[hy + 1][hx + 1]);
                }
            }
            else  // Diagonal from top-left to bottom-right (default)
            {
                if (u < y)  // <-- BUG: Should be 'v' not 'y' (line 1671)
                {
                    // v[0], v[3], v[2] - lower-right triangle
                    h = p->height[hy][hx] +
                        fu * (p->height[hy + 1][hx + 1] - p->height[hy + 1][hx]) +
                        fv * (p->height[hy + 1][hx] - p->height[hy][hx]);
                }
                else
                {
                    // v[0], v[1], v[3] - upper-left triangle
                    h = p->height[hy][hx] +
                        fu * (p->height[hy][hx + 1] - p->height[hy][hx]) +
                        fv * (p->height[hy + 1][hx + 1] - p->height[hy][hx + 1]);
                }
            }
            // ...
        }
    }
}
```

### Interpolation Coefficients
- `fu` = fractional x position within height cell (range [0, 1))
- `fv` = fractional y position within height cell (range [0, 1))
- Height vertices: `[hy][hx]`, `[hy][hx+1]`, `[hy+1][hx]`, `[hy+1][hx+1]`

### Known Issue
Line 1671 has a bug: `if (u < y)` should likely be `if (u < v)` - comparing loop variables from different scopes.

---

## 4. Tap3x3 Function - Boundary Check Logic

**Location: Lines 413-555**

The `Tap3x3` class provides access to the 3x3 neighborhood of patches around a center patch, handling wrapping at patch boundaries.

### Constructor (Lines 413-427)
```cpp
struct Tap3x3
{
    Tap3x3(Patch* c)
    {
        assert(c);
        p[0][0] = GetTerrainNeighbor(c, -1, -1);
        p[0][1] = GetTerrainNeighbor(c, 0, -1);
        p[0][2] = GetTerrainNeighbor(c, +1, -1);
        p[1][0] = GetTerrainNeighbor(c, -1, 0);
        p[1][1] = c;
        p[1][2] = GetTerrainNeighbor(c, +1, 0);
        p[2][0] = GetTerrainNeighbor(c, -1, +1);
        p[2][1] = GetTerrainNeighbor(c, 0, +1);
        p[2][2] = GetTerrainNeighbor(c, +1, +1);
    }
    // ...
    Patch* p[3][3];
};
```

### Sample() Boundary Check Logic (Lines 470-516)

**IMPORTANT: This section contains the boundary condition change noted in the TODO at lines 91-92**

```cpp
int Sample(int x, int y)
{
    int px = 1, py = 1;

    // X boundary check
    if (x < 0)
    {
        x += HEIGHT_CELLS;
        px = 0;  // <-- Use left neighbor patch
    }
    else
    if (x /*>=*/ > HEIGHT_CELLS)  // <-- NOTE: Commented out ">=" and changed to ">"
                                    // TODO: "assuming '>' is fresher" - needs verification
    {
        x -= HEIGHT_CELLS;
        px = 2;  // <-- Use right neighbor patch
    }

    // Y boundary check
    if (y < 0)
    {
        y += HEIGHT_CELLS;
        py = 0;  // <-- Use bottom neighbor patch
    }
    else
    if (y /*>=*/ > HEIGHT_CELLS)  // <-- NOTE: Same change as above
    {
        y -= HEIGHT_CELLS;
        py = 2;  // <-- Use top neighbor patch
    }

    // Handle missing neighbor patches
    if (!p[py][px])
    {
        if (px == 0)
            x = 0;              // Clamp to left edge
        else
        if (px == 2)
            x = HEIGHT_CELLS;  // Clamp to right edge

        if (py == 0)
            y = 0;              // Clamp to bottom edge
        else
        if (py == 2)
            y = HEIGHT_CELLS;  // Clamp to top edge

        px = 1;
        py = 1;  // <-- Fall back to center patch
    }

    return p[py][px]->height[y][x];
}
```

### SetDiag() Boundary Check (Lines 429-467)
Similar logic for setting diagonal flags:

```cpp
void SetDiag(int x, int y, bool d)
{
    int px = 1, py = 1;

    if (x < 0)
    {
        x += HEIGHT_CELLS;
        px = 0;
    }
    else
    if (x >= HEIGHT_CELLS)  // <-- Original condition (>=)
    {
        x -= HEIGHT_CELLS;
        px = 2;
    }

    if (y < 0)
    {
        y += HEIGHT_CELLS;
        py = 0;
    }
    else
    if (y >= HEIGHT_CELLS)  // <-- Original condition (>=)
    {
        y -= HEIGHT_CELLS;
        py = 2;
    }

    if (p[py][px])
    {
        if (d)
            p[py][px]->diag |= 1 << (x + y * HEIGHT_CELLS);
        else
            p[py][px]->diag &= ~(1 << (x + y * HEIGHT_CELLS));
    }
    else
    {
        int a = 0;  // <-- TODO: Debug leftover (lines 465-466)
    }
}
```

### Key Observations

1. **Boundary Condition Discrepancy**: `SetDiag()` uses `>=` (original) while `Sample()` uses `>` (changed with comment "assuming '>' is fresher"). This inconsistency is noted in the TODO at lines 91-92.

2. **Debug Leftover**: Lines 465-466 have `int a = 0;` in an empty else block - noted as debug leftover in the TODO at line 93.

3. **Wrap-around Behavior**: When coordinates go beyond patch boundaries, the function wraps to the opposite edge and selects the appropriate neighbor patch (px/py = 0, 1, or 2).

4. **Fallback Handling**: If a neighbor patch doesn't exist (px or py is 0 or 2 but p[py][px] is NULL), the coordinates are clamped to the center patch's edge.

---

## Summary

| Item | Status | Lines |
|------|--------|-------|
| SAH Cost Threshold | **NOT IMPLEMENTED** | N/A |
| Neighbor Flag Mapping | Implemented | 38-48, 617-644, 742-761, 1106-1125 |
| Height Interpolation | Bilinear per triangle | 1630-1691 |
| Tap3x3 Boundary Check | Wrapping with fallback | 413-555 |
