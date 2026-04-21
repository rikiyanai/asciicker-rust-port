# Alex Harri K-D Tree Implementation Audit

## Overview

This document provides a comprehensive audit of the k-d tree implementation used in Alex Harri's ASCII renderer for nearest-neighbor character matching in space. The implementation is 6D vector found in two key source files within the Alex Harri ASCII renderer repository.

## Source Files

The k-d tree implementation consists of two TypeScript files located in the character lookup module:

- `/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/characterLookup/KdTree.ts` (105 lines)
- `/Users/r/Projects/ascii research/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/characterLookup/CharacterMatcher.ts` (53 lines)

## 1. K-D Tree Construction Parameters

### 1.1 Dimensionality

The k-d tree operates in 6-dimensional vector space. This corresponds to the 6 sampling points used to capture character shape information. Each character in the alphabet is represented as a 6D vector derived from lightness samples at 6 circular positions within each character cell.

### 1.2 Construction Algorithm

The tree uses a standard recursive median-split construction algorithm. The constructor accepts an array of objects, each containing a vector and associated data:

```typescript
constructor(vectors: Array<{ vector: number[]; data: T }>) {
  if (vectors.length === 0) {
    throw new Error("Cannot create K-d tree with empty vectors array");
  }
  this.dimensions = vectors[0].vector.length;
  this.root = this.buildTree(vectors, 0);
}
```

The construction process proceeds as follows: the tree starts with the full set of character vectors, sorts them by the current splitting axis, selects the median element as the node, and recursively constructs left and right subtrees from the elements below and above the median respectively.

### 1.3 Node Structure

Each k-d tree node contains the following fields:

```typescript
interface KdTreeNode<T> {
  vector: number[];      // The 6D vector at this node
  data: T;               // Associated data (character string)
  left?: KdTreeNode<T>;  // Left child subtree
  right?: KdTreeNode<T>; // Right child subtree
  axis: number;         // Split axis (0-5)
}
```

### 1.4 Alphabet Size Considerations

The typical alphabet contains approximately 80-95 ASCII characters. With 6D vectors, this results in a tree with a maximum depth of approximately log2(95) ≈ 7 levels. The tree is relatively shallow due to the moderate number of characters being indexed.

## 2. Split Strategy and Depth Limits

### 2.1 Axis Selection Strategy

The implementation uses a cyclical axis selection strategy based on tree depth. The splitting axis is determined by the formula: axis = depth % dimensions. For 6D vectors, this produces the following sequence:

- Depth 0: axis 0
- Depth 1: axis 1
- Depth 2: axis 2
- Depth 3: axis 3
- Depth 4: axis 4
- Depth 5: axis 5
- Depth 6: axis 0 (cycles back)

This ensures that each dimension is used for splitting approximately equally throughout the tree.

### 2.2 Median Split Implementation

The median split is implemented using JavaScript's array sort method:

```typescript
const axis = depth % this.dimensions;
vectors.sort((a, b) => a.vector[axis] - b.vector[axis]);
const medianIndex = Math.floor(vectors.length / 2);
const median = vectors[medianIndex];
```

Note that this implementation uses in-place sorting, which modifies the input array. The code slices the array to create left and right partitions, which mitigates some of the sorting overhead but still requires O(n log n) sorting at each level.

### 2.3 Depth Limit

There is no explicit depth limit in this implementation. The tree builds to full depth, terminating only when a leaf node is reached (vectors.length === 1). This means every character vector becomes a leaf node in the final tree.

### 2.4 Leaf Node Handling

When a node reaches a single vector, it creates a leaf node with no children:

```typescript
if (vectors.length === 1) {
  return {
    vector: vectors[0].vector,
    data: vectors[0].data,
    axis: depth % this.dimensions,
  };
}
```

## 3. Cache Quantization Details

### 3.1 Quantization Parameters

The cache quantization uses fixed parameters defined as constants:

```typescript
const BITS = 5;
const RANGE = 8;
```

These values determine how sampling vectors are converted to cache keys.

### 3.2 Quantization Algorithm

The quantization converts each component of a 6D vector to a 5-bit integer (values 0-31), but the actual range is clamped to 0-7 due to the RANGE value:

```typescript
function quantizeToKey(vector: number[]): number {
  let key = 0;
  for (let i = 0; i < vector.length; i++) {
    const quantized = Math.min(RANGE - 1, Math.floor(vector[i] * RANGE));
    key = (key << BITS) | quantized;
  }
  return key;
}
```

### 3.3 Key Space Analysis

For 6D vectors with 5 bits per component, the total key space is 2^(5*6) = 2^30 = 1,073,741,824 possible keys. However, due to the clamping behavior (RANGE - 1 = 7), each component only uses 3 bits effectively (values 0-7), giving a practical key space of 2^18 = 262,144 unique keys.

### 3.4 Input Normalization Assumption

The quantization formula assumes input vectors are normalized to the range [0, 1]. The multiplication by RANGE (8) and floor operation converts normalized values to integer indices. If input vectors can exceed 1.0, they will be clamped to 7 due to the Math.min() call.

### 3.5 Cache Implementation

The cache is implemented as a simple JavaScript Map:

```typescript
private cache = new Map<number, string>();
```

The cache stores quantized keys mapped to character strings. There is no eviction policy or size limit.

## 4. Performance-Related Configuration

### 4.1 Query Method Selection

The CharacterMatcher provides two query methods:

1. `findBestCharacter(samplingVector: number[])` - Direct k-d tree search without cache
2. `findBestCharacterQuantized(samplingVector: number[])` - Cache-aware search

The cached version is used in the actual rendering pipeline:

```typescript
findBestCharacterQuantized(samplingVector: number[]): string {
  const key = quantizeToKey(samplingVector);
  if (this.cache.has(key)) {
    return this.cache.get(key)!;
  }
  const result = this.findBestCharacter(samplingVector);
  this.cache.set(key, result);
  return result;
}
```

### 4.2 Distance Metric

The implementation uses Euclidean distance:

```typescript
private distance(vector1: number[], vector2: number[]): number {
  let sum = 0;
  for (let i = 0; i < this.dimensions; i++) {
    const diff = vector1[i] - vector2[i];
    sum += diff * diff;
  }
  return Math.sqrt(sum);
}
```

Note that the square root is computed even though it is only needed for the final distance value. In performance-critical code, comparing squared distances would avoid the expensive sqrt operation.

### 4.3 Search Pruning

The k-d tree search uses standard k-d tree pruning optimization:

```typescript
const diff = target[axis] - node.vector[axis];
const primarySide = diff < 0 ? node.left : node.right;
const secondarySide = diff < 0 ? node.right : node.left;

search(primarySide, depth + 1);

if (!best || Math.abs(diff) < best.distance) {
  search(secondarySide, depth + 1);
}
```

The algorithm searches the likely-closer side first, then only searches the opposite side if the splitting plane is within the current best distance. Thisprunes large portions of the tree that cannot possibly contain a closer match.

### 4.4 Cache Hit Rate Optimization

In typical usage, many sampling vectors across a frame will be identical or very similar. The quantized cache provides O(1) lookup for repeated patterns, which is critical for animated scenes where large regions of the grid may have similar sampling results.

### 4.5 Memory Considerations

The cache grows unbounded during runtime. For a typical 160x90 grid (14,400 cells) at 60fps, the cache could potentially grow to contain many thousands of entries. In practice, the limited key space (262,144 keys) provides an upper bound.

## 5. Integration Points

### 5.1 Character Matcher Usage

The CharacterMatcher is instantiated in the renderer components:

```typescript
const characterMatcher = useMemo(() => {
  const matcher = new CharacterMatcher();
  matcher.loadAlphabet(alphabet, effects, exclude);
  return matcher;
}, [alphabet, effects, exclude]);
```

### 5.2 Alphabet Loading

The loadAlphabet method applies effects to character vectors before building the k-d tree:

```typescript
loadAlphabet(alphabet: AlphabetName, effects: Effect[], exclude: string): void {
  const characterVectors = getAlphabetCharacterVectors(alphabet).filter(
    (vector) => !exclude.includes(vector.char),
  );
  const vectors = characterVectors.map(({ vector }) => [...vector]);
  for (const effect of effects) {
    effect(vectors);
  }
  this.kdTree = new KdTree(
    characterVectors.map(({ char }, i) => ({ vector: vectors[i], data: char })),
  );
}
```

Effects such as global crunch normalization are applied to the vectors before tree construction, ensuring the k-d tree is built with the correct vector representation for matching.

## 6. Summary of Key Parameters

| Parameter | Value | Description |
|-----------|-------|-------------|
| Dimensions | 6 | Number of components in each vector |
| Split Strategy | Median | Standard k-d tree median partition |
| Axis Selection | Cyclical (depth % dimensions) | Round-robin through all 6 axes |
| Depth Limit | None (full depth) | Tree builds to completion |
| Distance Metric | Euclidean | Standard L2 norm with sqrt |
| Cache Bits Per Component | 5 | Quantization precision |
| Cache Range | 8 | Quantization multiplier |
| Cache Key Space | 262,144 | Practical unique keys (2^18) |
| Cache Type | Map | JavaScript Map data structure |
| Cache Eviction | None | Unbounded growth |

## 7. Porting Considerations

When porting this implementation to Rust for the asciicker project, the following considerations apply:

1. The median split requires sorting at each level. Consider using a build-time optimization or pre-sorting to avoid repeated sorting during construction.

2. The quantization parameters (BITS=5, RANGE=8) are hardcoded. These may need adjustment based on the expected input vector range in the target application.

3. The Euclidean distance calculation computes sqrt on every comparison. For better performance, consider comparing squared distances to avoid the sqrt operation.

4. The cache uses a simple hash map with no eviction. For long-running applications, consider implementing an LRU cache or other eviction strategy.

5. The k-d tree search performs dynamic allocation for the recursion stack. Consider implementing an iterative version to avoid stack overflow with deeper trees.

