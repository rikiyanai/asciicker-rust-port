# stb_vorbis.cpp - Function Analysis

**File:** `/Users/r/Downloads/asciicker-Y9-2/stb_vorbis.cpp` (5684 lines)

**Purpose:** Vendored single-file Ogg Vorbis audio decoder (v1.09) - public domain audio decompression library by Sean Barrett.

**License:** Dual-licensed to public domain and under license to copy, modify, publish, and distribute freely.

---

## Error Handling Functions

### `error` (stb_vorbis.cpp:878-885)

**Signature:**
```c
static int error(vorb *f, enum STBVorbisError e)
```

**Purpose:** Sets error state on decoder instance and optionally logs breakpoint for debugging.

**Called by:** 
- Grep: `grep -n "error(f," stb_vorbis.cpp` — used throughout decoder for error reporting

**Calls:** None (only field assignment and debug breakpoint)

**Globals read:** None

**Globals mutated:** `f->error` field mutated to store error code

**Side effects:** Sets `f->error` to provided enum value; conditional breakpoint trigger if not EOF and error is not `VORBIS_need_more_data`.

**Notes:** Return value is always 0 (unused). Designed to support debugging by triggering on non-recoverable errors.

---

## Memory Allocation Functions

### `make_block_array` (stb_vorbis.cpp:907-917)

**Signature:**
```c
static void *make_block_array(void *mem, int count, int size)
```

**Purpose:** Partitions a contiguous memory block into an array of pointers to equal-sized subblocks.

**Called by:** 
- `temp_block_array` macro (stb_vorbis.cpp:904)

**Calls:** None (only pointer arithmetic)

**Globals read:** None

**Globals mutated:** None

**Side effects:** In-place data structure initialization; does not allocate, only reorganizes provided memory.

**Notes:** Used for dynamic allocation from pre-allocated buffers. Returns `void**` pointing to first subblock; subsequent subblocks are accessed by array indexing.

---

### `setup_malloc` (stb_vorbis.cpp:919-930)

**Signature:**
```c
static void *setup_malloc(vorb *f, int sz)
```

**Purpose:** Allocates memory during setup phase, either from pre-allocated buffer or heap malloc.

**Called by:** 
- Multiple setup functions throughout file

**Calls:** `malloc()` (if no pre-allocated buffer)

**Globals read:** None

**Globals mutated:** `f->setup_memory_required`, `f->setup_offset` (tracks cumulative allocation)

**Side effects:** Tracks total setup memory needed; advances offset pointer; may fail and return NULL if buffer exhausted.

**Notes:** Size is aligned to 4-byte boundary. If `f->alloc.alloc_buffer` is set, allocates from that buffer (stack allocation); otherwise uses heap malloc.

---

### `setup_free` (stb_vorbis.cpp:932-936)

**Signature:**
```c
static void setup_free(vorb *f, void *p)
```

**Purpose:** No-op deallocation for setup memory; actual memory is managed by parent decoder.

**Called by:** 
- `vorbis_deinit()` (line 4385)

**Calls:** `free()` (only if using heap malloc)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Calls free() only if decoder not using pre-allocated buffer (otherwise no-op).

**Notes:** Reflects design pattern: setup memory is either stack-based or managed as a single block.

---

### `setup_temp_malloc` (stb_vorbis.cpp:938-947)

**Signature:**
```c
static void *setup_temp_malloc(vorb *f, int sz)
```

**Purpose:** Allocates temporary memory during decoding from high end of pre-allocated buffer or heap.

**Called by:** 
- `temp_alloc` macro and various decode functions

**Calls:** `malloc()` (if no pre-allocated buffer)

**Globals read:** None

**Globals mutated:** `f->temp_offset` (decremented to track temporary allocation from high end)

**Side effects:** Allocates from top of buffer (downward), failing if collision with setup offset. Returns NULL on failure.

**Notes:** Size aligned to 4-byte boundary. Contrasts with `setup_malloc` which allocates from low end upward.

---

### `setup_temp_free` (stb_vorbis.cpp:949-956)

**Signature:**
```c
static void setup_temp_free(vorb *f, void *p, int sz)
```

**Purpose:** Deallocates temporary memory, restoring offset pointer if using pre-allocated buffer.

**Called by:** 
- `temp_free` macro

**Calls:** `free()` (only if using heap malloc)

**Globals read:** None

**Globals mutated:** `f->temp_offset` (incremented to restore available space)

**Side effects:** Reverses temporary allocation (LIFO-like behavior for pre-allocated buffer).

**Notes:** Size parameter used only for pre-allocated buffer case (to restore offset exactly).

---

## CRC32 Functions

### `crc32_init` (stb_vorbis.cpp:961-970)

**Signature:**
```c
static void crc32_init(void)
```

**Purpose:** Initializes static CRC-32 lookup table using polynomial 0x04c11db7 (Ogg Vorbis spec).

**Called by:** 
- Initialization phase of decoder ( in `start_decoder()`)

**Calls:** None (only array initialization loop)

**Globals read:** None

**Globals mutated:** `crc_table[256]` static array

**Side effects:** One-time initialization of global lookup table used for all subsequent CRC calculations.

**Notes:** Uses Ogg Vorbis standard polynomial. Must be called before any `crc32_update()` calls.

---

### `crc32_update` (stb_vorbis.cpp:972-975)

**Signature:**
```c
static __forceinline uint32 crc32_update(uint32 crc, uint8 byte)
```

**Purpose:** Updates running CRC-32 checksum with one byte using pre-computed lookup table.

**Called by:** 
- Ogg page validation throughout decoder

**Calls:** None (only lookup and XOR)

**Globals read:** `crc_table[256]` (initialized by `crc32_init()`)

**Globals mutated:** None

**Side effects:** None; pure function (returns new CRC value).

**Notes:** Marked `__forceinline` for performance. Implements standard CRC-32 rolling update.

---

## Bit Manipulation Functions

### `bit_reverse` (stb_vorbis.cpp:979-986)

**Signature:**
```c
static unsigned int bit_reverse(unsigned int n)
```

**Purpose:** Reverses bit order of 32-bit unsigned integer using parallel bit-swap technique.

**Called by:** 
- Huffman decoding setup, bitreverse operations in FFT

**Calls:** None (only bitwise operations)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None; pure function.

**Notes:** Uses sequence of AND/OR/shift to swap bits at progressively coarser granularity (1-bit, 2-bit, 4-bit, 8-bit, 16-bit).

---

### `square` (stb_vorbis.cpp:988-991)

**Signature:**
```c
static float square(float x)
```

**Purpose:** Computes x² for floating-point value.

**Called by:** 
- Inverse MDCT computation

**Calls:** None (only multiplication)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None; pure function.

**Notes:** Trivial helper; is optimized to direct multiplication in many compilers.

---

### `ilog` (stb_vorbis.cpp:996-1011)

**Signature:**
```c
static int ilog(int32 n)
```

**Purpose:** Computes custom log₂ where ilog(1)=1, ilog(2)=2, ilog(4)=3 (Vorbis spec definition).

**Called by:** 
- Multiple setup and decoding functions for bit-width computation

**Calls:** None (only comparisons and table lookup)

**Globals read:** `log2_4[16]` local static table

**Globals mutated:** None

**Side effects:** None; pure function. Returns 0 for signed n or n >= 2³¹.

**Notes:** Fast implementation via cascading comparisons and 4-bit lookup table. Critical for codec decoding accuracy.

---

## Float Unpacking

### `float32_unpack` (stb_vorbis.cpp:1025-1042)

**Signature:**
```c
static float float32_unpack(uint32 x)
```

**Purpose:** Decodes IEEE 754 32-bit float from Vorbis-specific bit layout.

**Called by:** 
- Codebook setup functions during decoder initialization

**Calls:** None (only bit manipulation and floating-point construction)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None; pure function.

**Notes:** Implements Ogg Vorbis spec for 32-bit float encoding (mantissa/exponent extraction).

---

## Codebook Functions

### `add_entry` (stb_vorbis.cpp:1043-1053)

**Signature:**
```c
static void add_entry(Codebook *c, uint32 huff_code, int symbol, int count, int len, uint32 *values)
```

**Purpose:** Registers a Huffman code entry in codebook during setup.

**Called by:** 
- Huffman tree construction in `compute_codewords()`

**Calls:** None (only struct field assignment)

**Globals read:** None

**Globals mutated:** `c->codeword_lengths`, `c->codewords` (codebook structure fields)

**Side effects:** Updates codebook tables with new Huffman code and symbol.

**Notes:** Called during codec setup; not hot-path during decoding.

---

### `compute_codewords` (stb_vorbis.cpp:1054-1101)

**Signature:**
```c
static int compute_codewords(Codebook *c, uint8 *len, int n, uint32 *values)
```

**Purpose:** Builds canonical Huffman codebook from code lengths and symbol values.

**Called by:** 
- `start_decoder()` during codebook initialization

**Calls:** `add_entry()` (for each symbol)

**Globals read:** None

**Globals mutated:** Codebook structure (`c->codewords`, `c->codeword_lengths`)

**Side effects:** Fully populates codebook Huffman table; returns 1 on success, 0 on failure.

**Notes:** Implements canonical Huffman construction algorithm per Ogg Vorbis spec.

---

### `compute_accelerated_huffman` (stb_vorbis.cpp:1102-1129)

**Signature:**
```c
static void compute_accelerated_huffman(Codebook *c)
```

**Purpose:** Creates lookup tables for fast Huffman decoding using pre-computed bit patterns.

**Called by:** 
- `start_decoder()` after codebook setup

**Calls:** None (only table initialization)

**Globals read:** None

**Globals mutated:** `c->fast_huffman` acceleration table

**Side effects:** Enables fast path decoding; trades memory for speed.

**Notes:** Speed optimization layer over canonical Huffman codes.

---

### `uint32_compare` (stb_vorbis.cpp:1130-1136)

**Signature:**
```c
static int STBV_CDECL uint32_compare(const void *p, const void *q)
```

**Purpose:** Comparison function for qsort on 32-bit unsigned integers.

**Called by:** 
- `qsort()` in `compute_sorted_huffman()`

**Calls:** None (only pointer dereference and subtraction)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None; pure comparator.

**Notes:** Marked `STBV_CDECL` for Windows calling convention compatibility.

---

### `include_in_sort` (stb_vorbis.cpp:1137-1146)

**Signature:**
```c
static int include_in_sort(Codebook *c, uint8 len)
```

**Purpose:** Determines whether a codeword length should be included in sorted Huffman table.

**Called by:** 
- `compute_sorted_huffman()` for filtering

**Calls:** None (only comparison)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None; pure predicate.

**Notes:** Filters out unused code lengths (len==0 or len==255).

---

### `compute_sorted_huffman` (stb_vorbis.cpp:1147-1200)

**Signature:**
```c
static void compute_sorted_huffman(Codebook *c, uint8 *lengths, uint32 *values)
```

**Purpose:** Builds sorted Huffman table for efficient decoding via binary search.

**Called by:** 
- `start_decoder()` during codebook setup

**Calls:** 
- `qsort()` (standard C sort)
- `include_in_sort()` (filter predicate)

**Globals read:** None

**Globals mutated:** `c->sorted_codewords`, `c->sorted_values`, `c->sorted_codeword_lengths` (codebook fields)

**Side effects:** Sorts and partitions Huffman codes by length; enables logarithmic-time lookup.

**Notes:** Alternative to fast Huffman table for memory-constrained scenarios.

---

## Validation & Setup Helper Functions

### `vorbis_validate` (stb_vorbis.cpp:1201-1208)

**Signature:**
```c
static int vorbis_validate(uint8 *data)
```

**Purpose:** Validates Ogg Vorbis header magic bytes ("vorbis" ASCII).

**Called by:** 
- Setup functions to verify stream format

**Calls:** None (only memory comparison)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None; pure validation.

**Notes:** Checks for 6-byte "vorbis" string at provided location.

---

### `lookup1_values` (stb_vorbis.cpp:1209-1219)

**Signature:**
```c
static int lookup1_values(int entries, int dim)
```

**Purpose:** Computes number of distinct values in a Vorbis codebook with multiplicities.

**Called by:** 
- Codebook setup during decoder initialization

**Calls:** None (only arithmetic and bit operations)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None; pure function.

**Notes:** Implements Vorbis spec formula: `V^dim >= entries` where V is returned value.

---

### `compute_twiddle_factors` (stb_vorbis.cpp:1220-1236)

**Signature:**
```c
static void compute_twiddle_factors(int n, float *A, float *B, float *C)
```

**Purpose:** Pre-computes sine/cosine twiddle factors for FFT-based IMDCT operation.

**Called by:** 
- FFT initialization during setup

**Calls:** None (only trigonometric computation)

**Globals read:** `M_PI` constant

**Globals mutated:** Output arrays `A`, `B`, `C` (twiddle factor tables)

**Side effects:** Fills three output arrays with precomputed values for inverse MDCT.

**Notes:** Speed optimization for inverse MDCT decoding.

---

### `compute_window` (stb_vorbis.cpp:1237-1243)

**Signature:**
```c
static void compute_window(int n, float *window)
```

**Purpose:** Computes Vorbis window function (Kiserla-Hann window) for frame overlap.

**Called by:** 
- Frame processing setup

**Calls:** None (only floating-point math)

**Globals read:** `M_PI` constant

**Globals mutated:** Output array `window` (window coefficients)

**Side effects:** Populates window coefficients for frame windowing.

**Notes:** Essential for overlap-add frame processing to avoid artifacts at boundaries.

---

### `compute_bitreverse` (stb_vorbis.cpp:1244-1251)

**Signature:**
```c
static void compute_bitreverse(int n, uint16 *rev)
```

**Purpose:** Pre-computes bit-reversal indices for FFT (Cooley-Tukey radix-2).

**Called by:** 
- FFT setup during decoder initialization

**Calls:** 
- `bit_reverse()` (for each index)

**Globals read:** None

**Globals mutated:** Output array `rev` (bit-reversal permutation table)

**Side effects:** Populates lookup table used during FFT reordering.

**Notes:** Standard technique for in-place FFT implementation.

---

### `init_blocksize` (stb_vorbis.cpp:1252-1268)

**Signature:**
```c
static int init_blocksize(vorb *f, int b, int n)
```

**Purpose:** Initializes MDCT window and FFT tables for specified block size.

**Called by:** 
- `start_decoder()` during codec setup

**Calls:** 
- `compute_window()`, `compute_twiddle_factors()`, `compute_bitreverse()` (setup functions)

**Globals read:** None

**Globals mutated:** `f->blocksize_[b]` and associated FFT/window tables

**Side effects:** Populates decoder's FFT and window data structures for block size `b`.

**Notes:** Called for each block size used in stream (typically 2 sizes: long and short).

---

### `neighbors` (stb_vorbis.cpp:1269-1285)

**Signature:**
```c
static void neighbors(uint16 *x, int n, int *plow, int *phigh)
```

**Purpose:** Finds floor curve interpolation neighbors for a given point.

**Called by:** 
- Floor decoding in `do_floor()` (line 3064)

**Calls:** None (only comparison and assignment)

**Globals read:** None

**Globals mutated:** Outputs `*plow`, `*phigh` (neighbor indices)

**Side effects:** Modifies output pointers with neighbor indices.

**Notes:** Binary search variant for floor interpolation (Vorbis spec part 2).

---

### `point_compare` (stb_vorbis.cpp:1286-1302)

**Signature:**
```c
static int STBV_CDECL point_compare(const void *p, const void *q)
```

**Purpose:** Comparison function for qsort on floor control points (by x coordinate).

**Called by:** 
- `qsort()` in floor setup

**Calls:** None (only pointer dereference and subtraction)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None; pure comparator.

**Notes:** Sorts floor points by x-coordinate for binary search queries.

---

## Bit-Level I/O Functions

### `get8` (stb_vorbis.cpp:1303-1318)

**Signature:**
```c
static uint8 get8(vorb *z)
```

**Purpose:** Reads single byte from input stream.

**Called by:** 
- Multiple parsing functions

**Calls:** `fgetc()` (if FILE mode), or direct buffer read

**Globals read:** `z->f` (file pointer, if FILE mode), `z->stream` (buffer pointer)

**Globals mutated:** `z->stream` or file offset (advances input position)

**Side effects:** Consumes one byte from stream; updates EOF/error state if applicable.

**Notes:** Handles both FILE and buffer input modes conditionally.

---

### `get32` (stb_vorbis.cpp:1319-1328)

**Signature:**
```c
static uint32 get32(vorb *f)
```

**Purpose:** Reads 32-bit little-endian value from stream.

**Called by:** 
- OGG page header parsing

**Calls:** `get8()` (four times)

**Globals read:** None (indirectly via `get8()`)

**Globals mutated:** Stream position (via `get8()`)

**Side effects:** Consumes 4 bytes and advances stream position.

**Notes:** Assumes little-endian byte order (standard for Ogg).

---

### `getn` (stb_vorbis.cpp:1329-1347)

**Signature:**
```c
static int getn(vorb *z, uint8 *data, int n)
```

**Purpose:** Reads n bytes from stream into provided buffer.

**Called by:** 
- Multiple parsing functions

**Calls:** `get8()` (n times) or `memcpy()` (optimized buffer read)

**Globals read:** `z->stream`, `z->stream_end`

**Globals mutated:** `z->stream` (advances position)

**Side effects:** Fills buffer with stream data; returns number of bytes read (is less than n if EOF).

**Notes:** Bounds-checked against `stream_end` to prevent buffer overrun.

---

### `skip` (stb_vorbis.cpp:1348-1362)

**Signature:**
```c
static void skip(vorb *z, int n)
```

**Purpose:** Skips n bytes in stream without storing data.

**Called by:** 
- Alignment and padding skipping

**Calls:** `getn()` (with dummy buffer), or direct pointer arithmetic

**Globals read:** None

**Globals mutated:** `z->stream` (advances position)

**Side effects:** Advances stream position without data transfer.

**Notes:** Optimization for seek-like operations.

---

### `set_file_offset` (stb_vorbis.cpp:1363-1394)

**Signature:**
```c
static int set_file_offset(stb_vorbis *f, unsigned int loc)
```

**Purpose:** Seeks to byte offset in file or buffer stream.

**Called by:** 
- Seeking functions, page scanning

**Calls:** `fseek()` (if FILE mode), or direct buffer pointer update

**Globals read:** `f->f` (file pointer), `f->stream`, `f->stream_start`, `f->stream_end`

**Globals mutated:** File position or `f->stream` pointer

**Side effects:** Repositions input stream; clears bit buffer state.

**Notes:** Handles both FILE and buffer modes; returns 1 on success, 0 on failure (seek out of bounds).

---

## OGG Page/Packet Functions

### `capture_pattern` (stb_vorbis.cpp:1397-1409)

**Signature:**
```c
static int capture_pattern(vorb *f)
```

**Purpose:** Validates Ogg page capture pattern (magic "OggS" bytes).

**Called by:** 
- Page sync and scanning functions

**Calls:** `get32()` (reads 4-byte header)

**Globals read:** None

**Globals mutated:** Stream position (via `get32()`)

**Side effects:** Consumes 4 bytes; returns 1 if pattern matches, 0 otherwise.

**Notes:** First check in Ogg page parsing; critical for stream synchronization.

---

### `start_page_no_capturepattern` (stb_vorbis.cpp:1410-1462)

**Signature:**
```c
static int start_page_no_capturepattern(vorb *f)
```

**Purpose:** Parses Ogg page header (assuming capture pattern already validated).

**Called by:** 
- `start_page()` (line 1463)

**Calls:** 
- `get8()`, `get32()`, `get64()` (read page fields)
- `crc32_update()` (checksum validation)

**Globals read:** None

**Globals mutated:** `f->page_*` fields (segment table, page number, granule, etc.)

**Side effects:** Populates page metadata; validates CRC32; returns 0 on error, 1 on success.

**Notes:** Implements Ogg page format parsing per RFC 3533.

---

### `start_page` (stb_vorbis.cpp:1463-1468)

**Signature:**
```c
static int start_page(vorb *f)
```

**Purpose:** Wrapper to parse Ogg page starting from capture pattern check.

**Called by:** 
- Page scanning and seeking operations

**Calls:** 
- `capture_pattern()` (validate magic)
- `start_page_no_capturepattern()` (parse header)

**Globals read:** None

**Globals mutated:** Stream position and page metadata (via called functions)

**Side effects:** Reads and validates full Ogg page header; returns 0 on error, 1 on success.

**Notes:** Top-level page parsing entry point.

---

### `start_packet` (stb_vorbis.cpp:1469-1483)

**Signature:**
```c
static int start_packet(vorb *f)
```

**Purpose:** Positions stream at start of next Vorbis packet after page boundary.

**Called by:** 
- Packet decoding initialization

**Calls:** 
- `next_segment()` (find packet segment)
- Error logging

**Globals read:** Page segment state

**Globals mutated:** Packet position and length state

**Side effects:** Updates packet boundary markers; returns 0 on error, 1 on success.

**Notes:** Handles packet spanning multiple pages.

---

### `maybe_start_packet` (stb_vorbis.cpp:1484-1504)

**Signature:**
```c
static int maybe_start_packet(vorb *f)
```

**Purpose:** Conditionally starts packet if not already at packet boundary.

**Called by:** 
- Packet decoding functions for defensive parsing

**Calls:** 
- `start_packet()` (if needed)

**Globals read:** Packet position state

**Globals mutated:** Packet state (via `start_packet()` if called)

**Side effects:** May advance packet boundary; returns 0 on error, 1 on success.

**Notes:** Idempotent operation; safe to call multiple times.

---

### `next_segment` (stb_vorbis.cpp:1505-1528)

**Signature:**
```c
static int next_segment(vorb *f)
```

**Purpose:** Advances to next segment in Ogg page segment table.

**Called by:** 
- Packet boundary detection

**Calls:** 
- `start_page()` (if page boundary reached)

**Globals read:** Current page segment table

**Globals mutated:** Segment index; may advance to next page

**Side effects:** Updates segment position; handles page transitions; returns segment length or -1 on error.

**Notes:** Handles variable-length segments and page spanning.

---

### `get8_packet_raw` (stb_vorbis.cpp:1529-1540)

**Signature:**
```c
static int get8_packet_raw(vorb *f)
```

**Purpose:** Reads single byte from packet without bit alignment.

**Called by:** 
- Low-level packet reading

**Calls:** 
- `get8()` (byte read)

**Globals read:** Packet buffer state

**Globals mutated:** Packet position (via `get8()`)

**Side effects:** Consumes 1 byte; returns byte value or error code.

**Notes:** No bit-level synchronization; raw byte access within packet.

---

### `get8_packet` (stb_vorbis.cpp:1541-1547)

**Signature:**
```c
static int get8_packet(vorb *f)
```

**Purpose:** Reads single byte from packet with bit alignment verification.

**Called by:** 
- Packet header parsing

**Calls:** 
- `get8_packet_raw()` (byte read)
- Error handling

**Globals read:** Bit position state

**Globals mutated:** Bit position

**Side effects:** Ensures byte alignment; may return error if not byte-aligned.

**Notes:** Wrapper adding alignment constraints over raw read.

---

### `flush_packet` (stb_vorbis.cpp:1548-1554)

**Signature:**
```c
static void flush_packet(vorb *f)
```

**Purpose:** Discards remaining data in current packet buffer.

**Called by:** 
- Error recovery, packet skipping

**Calls:** None (only field assignment)

**Globals read:** None

**Globals mutated:** `f->packet_bytes` (set to 0)

**Side effects:** Marks packet as consumed; subsequent reads will fail until next packet starts.

**Notes:** Used for error recovery.

---

## Bit-Level Decoding

### `get_bits` (stb_vorbis.cpp:1555-1588)

**Signature:**
```c
static uint32 get_bits(vorb *f, int n)
```

**Purpose:** Reads n bits from bit stream with LSB-first bit ordering.

**Called by:** 
- Every decoding function throughout decoder

**Calls:** 
- `get8_packet_raw()` (refill buffer when empty)

**Globals read:** Bit buffer state (`f->bits`, `f->bitcount`)

**Globals mutated:** Bit buffer and bit counter

**Side effects:** Advances bit position; may fetch new bytes; returns n-bit value.

**Notes:** Critical hot-path function; implements Vorbis bit-reading protocol. Maintains running bit buffer to minimize byte fetches.

---

### `prep_huffman` (stb_vorbis.cpp:1589-1610)

**Signature:**
```c
static __forceinline void prep_huffman(vorb *f)
```

**Purpose:** Prepares bit buffer for Huffman decoding (ensures sufficient bits cached).

**Called by:** 
- Huffman decode functions before code lookup

**Calls:** 
- `get8_packet_raw()` (if buffer needs refill)

**Globals read:** Bit buffer state

**Globals mutated:** Bit buffer (`f->bits`, `f->bitcount`)

**Side effects:** Fills bit buffer to at least 24 bits (typical Huffman max code length).

**Notes:** Marked `__forceinline` for performance; allows Huffman decoder to work with cached bits.

---

## Huffman Decoding

### `codebook_decode_scalar_raw` (stb_vorbis.cpp:1611-1687)

**Signature:**
```c
static int codebook_decode_scalar_raw(vorb *f, Codebook *c)
```

**Purpose:** Decodes single Huffman symbol using linear search (fallback, no acceleration).

**Called by:** 
- `codebook_decode_scalar()` when fast table not available

**Calls:** 
- `get_bits()` (read bits)

**Globals read:** Codebook sorted tables

**Globals mutated:** Bit stream position (via `get_bits()`)

**Side effects:** Advances bit stream; returns decoded symbol or error.

**Notes:** Slow path; used when acceleration tables not built. Implements canonical Huffman binary search.

---

### `codebook_decode_scalar` (stb_vorbis.cpp:1688-1729)

**Signature:**
```c
static int codebook_decode_scalar(vorb *f, Codebook *c)
```

**Purpose:** Decodes single Huffman symbol using fast or fallback method.

**Called by:** 
- Multiple decoding functions

**Calls:** 
- `codebook_decode_scalar_raw()` (fallback if no fast table)
- Fast table lookup (inline)

**Globals read:** Codebook acceleration tables

**Globals mutated:** Bit stream position

**Side effects:** Advances bit stream; returns decoded symbol.

**Notes:** Fast path preferred; falls back to raw decoding if necessary.

---

### `codebook_decode_start` (stb_vorbis.cpp:1730-1749)

**Signature:**
```c
static int codebook_decode_start(vorb *f, Codebook *c)
```

**Purpose:** Initializes state for decoding sequence from codebook.

**Called by:** 
- Multi-symbol decoding sequences

**Calls:** 
- `codebook_decode_scalar()` (decode first symbol)

**Globals read:** None

**Globals mutated:** Decoder state for sequence

**Side effects:** Decodes first symbol; prepares for subsequent `codebook_decode()` calls.

**Notes:** Used for multiplicative codebook entries.

---

### `codebook_decode` (stb_vorbis.cpp:1750-1788)

**Signature:**
```c
static int codebook_decode(vorb *f, Codebook *c, float *output, int len)
```

**Purpose:** Decodes multiple Huffman symbols and converts to float values with lookup/scaling.

**Called by:** 
- Audio frame decoding (residue, floor, book lookups)

**Calls:** 
- `codebook_decode_scalar()` (for each symbol)
- `codebook_decode_start()` (initialization)

**Globals read:** Codebook lookup tables

**Globals mutated:** Output buffer, bit stream position

**Side effects:** Fills output buffer with decoded floating-point values.

**Notes:** Hot-path function; includes multiplicative codebook expansion.

---

### `codebook_decode_step` (stb_vorbis.cpp:1789-1819)

**Signature:**
```c
static int codebook_decode_step(vorb *f, Codebook *c, float *output, int len, int step)
```

**Purpose:** Decodes symbols with strided output (every `step`-th element).

**Called by:** 
- Stereo/multi-channel decoding

**Calls:** 
- `codebook_decode_scalar()` (for each symbol)

**Globals read:** Codebook tables

**Globals mutated:** Output buffer (strided), bit stream

**Side effects:** Fills every `step`-th output slot with decoded values.

**Notes:** Used for interleaved/multi-channel output.

---

### `codebook_decode_deinterleave_repeat` (stb_vorbis.cpp:1820-1889)

**Signature:**
```c
static int codebook_decode_deinterleave_repeat(vorb *f, Codebook *c, float **outputs, int ch, int *c_inter_p, int *p_inter_p, int len, int total_decode)
```

**Purpose:** Decodes symbols into multiple channels with deinterleaving and repetition handling.

**Called by:** 
- Residue decoding for multi-channel content

**Calls:** 
- `codebook_decode_scalar()` (for each symbol)
- `codebook_decode_start()` (initialization)

**Globals read:** Codebook tables, channel interleave patterns

**Globals mutated:** Multi-channel output buffers, bit stream, interleave state

**Side effects:** Fills multiple output buffers with deinterleaved decoded values.

**Notes:** Complex hot-path; handles multiplicative and interleaved codebooks.

---

## Prediction & Interpolation

### `predict_point` (stb_vorbis.cpp:1890-1900)

**Signature:**
```c
static int predict_point(int x, int x0, int x1, int y0, int y1)
```

**Purpose:** Linear interpolation for floor curve points.

**Called by:** 
- Floor decoding (`do_floor()`)

**Calls:** None (only arithmetic)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None; pure function.

**Notes:** Standard linear interpolation formula; returns interpolated y value at x.

---

### `draw_line` (stb_vorbis.cpp:1989-2037)

**Signature:**
```c
static __forceinline void draw_line(float *output, int x0, int y0, int x1, int y1, int n)
```

**Purpose:** Fills output buffer with linearly interpolated values from (x0,y0) to (x1,y1).

**Called by:** 
- Floor synthesis during frame decode

**Calls:** 
- `predict_point()` (for each sample)

**Globals read:** None

**Globals mutated:** `output` buffer (fills range [x0, x1])

**Side effects:** Writes interpolated samples to output.

**Notes:** Marked `__forceinline` for performance. Core loop in real-time decoding.

---

## Residue & Decoding

### `residue_decode` (stb_vorbis.cpp:2038-2056)

**Signature:**
```c
static int residue_decode(vorb *f, Codebook *book, float *target, int offset, int n, int rtype)
```

**Purpose:** Decodes residue vectors from Huffman codebook and writes to output buffer.

**Called by:** 
- `decode_residue()` (line 2057)

**Calls:** 
- Huffman decode functions

**Globals read:** Codebook data

**Globals mutated:** Target buffer, bit stream

**Side effects:** Populates residue buffer with decoded vectors.

**Notes:** Residue is final spectrum correction layer in Vorbis.

---

### `decode_residue` (stb_vorbis.cpp:2057-2397)

**Signature:**
```c
static void decode_residue(vorb *f, float *residue_buffers[], int ch, int n, int rn, uint8 *do_not_decode)
```

**Purpose:** Decodes all residue vectors for all channels in frame (main residue stage).

**Called by:** 
- `vorbis_decode_packet_rest()` (line 3171)

**Calls:** 
- Multiple codebook decoding functions
- Residue decode configuration dispatch

**Globals read:** Residue configuration, codebooks

**Globals mutated:** All residue buffers, bit stream

**Side effects:** Populates all channel residue buffers; major decoding stage.

**Notes:** Implements all three Vorbis residue types (0, 1, 2) with different partition strategies.

---

## IMDCT (Inverse Modified Discrete Cosine Transform) Functions

### `imdct_step3_iter0_loop` (stb_vorbis.cpp:2398-2442)

**Signature:**
```c
static void imdct_step3_iter0_loop(int n, float *e, int i_off, int k_off, float *A)
```

**Purpose:** First stage of 3-stage IMDCT butterfly operations (twiddle application).

**Called by:** 
- `inverse_mdct()` FFT stage (line 2620)

**Calls:** None (only arithmetic)

**Globals read:** None

**Globals mutated:** Buffer `e` (in-place FFT)

**Side effects:** Applies twiddle factors and butterflies to signal.

**Notes:** Core FFT butterfly computation; part of Cooley-Tukey FFT algorithm.

---

### `imdct_step3_inner_r_loop` (stb_vorbis.cpp:2443-2492)

**Signature:**
```c
static void imdct_step3_inner_r_loop(int lim, float *e, int d0, int k_off, float *A, int k1)
```

**Purpose:** Radix-2 butterfly loop for IMDCT (real-valued operations).

**Called by:** 
- FFT stages in `inverse_mdct()`

**Calls:** None (only arithmetic)

**Globals read:** None

**Globals mutated:** Buffer `e` (in-place computation)

**Side effects:** Applies radix-2 butterflies with twiddle factors.

**Notes:** Real FFT variant (not complex); part of IMDCT implementation.

---

### `imdct_step3_inner_s_loop` (stb_vorbis.cpp:2493-2543)

**Signature:**
```c
static void imdct_step3_inner_s_loop(int n, float *e, int i_off, int k_off, float *A, int a_off, int k0)
```

**Purpose:** Non-power-of-2 FFT stage with stride.

**Called by:** 
- FFT stages in `inverse_mdct()`

**Calls:** None (only arithmetic)

**Globals read:** None

**Globals mutated:** Buffer `e`

**Side effects:** Applies twiddle butterflies with stride support.

**Notes:** Handles arbitrary FFT sizes (not just power-of-2).

---

### `iter_54` (stb_vorbis.cpp:2544-2575)

**Signature:**
```c
static __forceinline void iter_54(float *z)
```

**Purpose:** Optimized 5-4 butterfly for radix-5 FFT stage.

**Called by:** 
- `imdct_step3_inner_s_loop_ld654()`

**Calls:** None (only arithmetic)

**Globals read:** None

**Globals mutated:** Buffer `z`

**Side effects:** In-place radix-5 butterfly computation.

**Notes:** Marked `__forceinline`; specialized computation for common FFT size (2^6 * 5).

---

### `imdct_step3_inner_s_loop_ld654` (stb_vorbis.cpp:2576-2619)

**Signature:**
```c
static void imdct_step3_inner_s_loop_ld654(int n, float *e, int i_off, float *A, int base_n)
```

**Purpose:** Specialized radix-2/radix-5 FFT stage for common block size (2048 samples).

**Called by:** 
- `inverse_mdct()` for optimized block size

**Calls:** 
- `iter_54()` (butterfly)

**Globals read:** None

**Globals mutated:** Buffer `e`

**Side effects:** Fast path FFT for 2048-sample blocks.

**Notes:** Performance optimization for most common MDCT size.

---

### `inverse_mdct` (stb_vorbis.cpp:2620-2923)

**Signature:**
```c
static void inverse_mdct(float *buffer, int n, vorb *f, int blocktype)
```

**Purpose:** Computes inverse Modified Discrete Cosine Transform (IMDCT) on input buffer.

**Called by:** 
- `vorbis_finish_frame()` (line 3446) to synthesize audio

**Calls:** 
- Multiple FFT stage functions (imdct_step3_*)
- `get_window()` (window function)

**Globals read:** Blocksize FFT tables, twiddle factors

**Globals mutated:** Buffer (in-place IMDCT computation)

**Side effects:** Transforms frequency-domain coefficients to time-domain samples.

**Notes:** Major computational kernel; implements FFT-based fast IMDCT per Vorbis spec.

---

## Frame Decoding

### `get_window` (stb_vorbis.cpp:3050-3063)

**Signature:**
```c
static float *get_window(vorb *f, int len)
```

**Purpose:** Returns window function for overlap-add frame processing.

**Called by:** 
- `vorbis_finish_frame()` for windowing
- IMDCT synthesis

**Calls:** None (table lookup only)

**Globals read:** `f->window_[blocktype]` precomputed windows

**Globals mutated:** None

**Side effects:** None; pure lookup.

**Notes:** Returns pointer to precomputed window coefficients.

---

### `do_floor` (stb_vorbis.cpp:3064-3114)

**Signature:**
```c
static int do_floor(vorb *f, Mapping *map, int i, int n, float *target, YTYPE *finalY, uint8 *step2_flag)
```

**Purpose:** Decodes floor (spectral envelope) for channel i and populates target buffer.

**Called by:** 
- `vorbis_decode_packet_rest()` (line 3171) for each channel

**Calls:** 
- Huffman decoding, floor interpolation
- `draw_line()`, `predict_point()`

**Globals read:** Floor configuration, codebooks

**Globals mutated:** Target buffer (spectral floor), bit stream

**Side effects:** Fills target buffer with floor curve values (masking curve).

**Notes:** Critical spectral shape stage; must complete before residue can be applied.

---

### `vorbis_decode_initial` (stb_vorbis.cpp:3115-3170)

**Signature:**
```c
static int vorbis_decode_initial(vorb *f, int *p_left_start, int *p_left_end, int *p_right_start, int *p_right_end, int *mode)
```

**Purpose:** Decodes frame header, mode, and channel window boundaries.

**Called by:** 
- `vorbis_decode_packet()` (line 3439) to start frame decode

**Calls:** 
- `get_bits()` (read frame header bits)
- Mode lookup

**Globals read:** Vorbis setup data (modes, mappings)

**Globals mutated:** Mode and window boundary parameters (outputs)

**Side effects:** Advances bit stream; returns 0 on error, 1 on success.

**Notes:** Initial stage of frame decoding; determines window shape and channel layout.

---

### `vorbis_decode_packet_rest` (stb_vorbis.cpp:3171-3438)

**Signature:**
```c
static int vorbis_decode_packet_rest(vorb *f, int *len, Mode *m, int left_start, int left_end, int right_start, int right_end, int *p_left)
```

**Purpose:** Decodes complete Vorbis frame given mode and window boundaries (floor + residue + coupling).

**Called by:** 
- `vorbis_decode_packet()` (line 3439)

**Calls:** 
- `do_floor()` (floor decoding)
- `decode_residue()` (residue decoding)
- Coupling operations
- `vorbis_finish_frame()` (synthesis)

**Globals read:** Mapping, coupling, and residue configurations

**Globals mutated:** Spectral buffers, bit stream, coupling state

**Side effects:** Completely decodes frame (floor + residue + coupling + synthesis); returns 0 on error, 1 on success.

**Notes:** Core frame decoding; implements floor + residue + IMDCT pipeline.

---

### `vorbis_decode_packet` (stb_vorbis.cpp:3439-3445)

**Signature:**
```c
static int vorbis_decode_packet(vorb *f, int *len, int *p_left, int *p_right)
```

**Purpose:** Wrapper to decode complete packet (header + frame data).

**Called by:** 
- Public frame decoding API (`stb_vorbis_get_frame_float()`, etc.)

**Calls:** 
- `vorbis_decode_initial()` (frame header)
- `vorbis_decode_packet_rest()` (frame data)

**Globals read:** None

**Globals mutated:** Output parameters, bit stream

**Side effects:** Decodes complete frame; returns number of samples or 0 on error.

**Notes:** Top-level frame decoding entry point.

---

### `vorbis_finish_frame` (stb_vorbis.cpp:3446-3497)

**Signature:**
```c
static int vorbis_finish_frame(stb_vorbis *f, int len, int left, int right)
```

**Purpose:** Applies IMDCT, windowing, and overlap-add to synthesize final PCM samples.

**Called by:** 
- `vorbis_decode_packet_rest()` after residue decode

**Calls:** 
- `get_window()` (window function)
- `inverse_mdct()` (IMDCT synthesis)
- Overlap-add operations

**Globals read:** Window and IMDCT tables

**Globals mutated:** `f->samples` buffer (output PCM), synthesis state

**Side effects:** Populates PCM output buffer; updates overlap buffer for next frame.

**Notes:** Final synthesis stage; converts spectral data to time-domain audio.

---

### `vorbis_pump_first_frame` (stb_vorbis.cpp:3498-3505)

**Signature:**
```c
static void vorbis_pump_first_frame(stb_vorbis *f)
```

**Purpose:** Decodes initial frame to populate output buffers after setup.

**Called by:** 
- `stb_vorbis_open_*()` functions after decoder initialization

**Calls:** 
- `vorbis_decode_packet()` (decode first frame)

**Globals read:** Decoder state

**Globals mutated:** Output buffers, synthesis state

**Side effects:** Fills initial PCM output; enables first call to frame reading functions.

**Notes:** Preparatory step for stream playback.

---

### `is_whole_packet_present` (stb_vorbis.cpp:3506-3598)

**Signature:**
```c
static int is_whole_packet_present(stb_vorbis *f, int end_page)
```

**Purpose:** Verifies that complete packet data is in buffer before attempting decode.

**Called by:** 
- Push-mode API functions for safety

**Calls:** 
- `start_page()`, page scanning functions

**Globals read:** Page and packet tables

**Globals mutated:** None

**Side effects:** None; pure validation.

**Notes:** Defensive check for push-data API; prevents buffer underrun during decode.

---

## Comments & Metadata

### `parse_comment` (stb_vorbis.cpp:3599-3749)

**Signature:**
```c
static void parse_comment(vorb *f, comment* com, const char* buf, int len)
```

**Purpose:** Parses Vorbis comment field (title, artist, etc.) from binary data.

**Called by:** 
- Setup packet parsing

**Calls:** 
- String parsing and allocation

**Globals read:** None

**Globals mutated:** `com` structure (populates comment metadata)

**Side effects:** Parses and stores comment data.

**Notes:** Handles UTF-8 comment format per Vorbis spec.

---

## Setup & Initialization

### `start_decoder` (stb_vorbis.cpp:3750-4384)

**Signature:**
```c
static int start_decoder(vorb *f)
```

**Purpose:** Complete decoder initialization from three Vorbis setup packets (identification, comments, setup).

**Called by:** 
- `stb_vorbis_open_*()` functions

**Calls:** 
- Extensive setup functions (codebook, floor, residue, mapping mode initialization)
- `crc32_init()`, codebook builders, FFT setup

**Globals read:** None

**Globals mutated:** All decoder tables and state

**Side effects:** Fully initializes decoder; returns 1 on success, 0 on error (sets error code).

**Notes:** Largest single function; implements complete Vorbis setup packet parsing. Must complete before any frame decoding.

---

### `vorbis_deinit` (stb_vorbis.cpp:4385-4447)

**Signature:**
```c
static void vorbis_deinit(stb_vorbis *p)
```

**Purpose:** Deallocates all decoder resources (codebooks, tables, buffers).

**Called by:** 
- `stb_vorbis_close()` (line 4448)
- Error cleanup in `stb_vorbis_open_*()` functions

**Calls:** 
- `setup_free()` (free allocated tables)

**Globals read:** None

**Globals mutated:** All decoder allocations freed

**Side effects:** Deallocates decoder memory; decoder becomes invalid after call.

**Notes:** Inverse of setup; safe to call multiple times (idempotent due to NULL checks).

---

### `vorbis_init` (stb_vorbis.cpp:4455-4473)

**Signature:**
```c
static void vorbis_init(stb_vorbis *p, const stb_vorbis_alloc *z)
```

**Purpose:** Initializes decoder structure to clean state (NULL fields, error codes).

**Called by:** 
- `stb_vorbis_open_*()` functions before setup

**Calls:** `memset()` (zero initialization)

**Globals read:** None

**Globals mutated:** All decoder fields initialized

**Side effects:** Zeros decoder structure; sets initial values for error state.

**Notes:** First step of decoder creation; enables error-safe cleanup if setup fails.

---

## Public API Functions

### `stb_vorbis_close` (stb_vorbis.cpp:4448-4453)

**Signature:**
```c
void stb_vorbis_close(stb_vorbis *p)
```

**Purpose:** Public API function to close/free Vorbis decoder instance.

**Called by:** 
- User code, `stb_vorbis_decode_*()` helpers

**Calls:** 
- `vorbis_deinit()` (deallocate internal state)
- `setup_free()` (free decoder structure itself)

**Globals read:** None

**Globals mutated:** All decoder memory freed

**Side effects:** Releases all resources; decoder pointer becomes invalid.

**Notes:** NULL-safe; can be called on NULL pointer without error.

---

### `stb_vorbis_get_sample_offset` (stb_vorbis.cpp:4475-4481)

**Signature:**
```c
int stb_vorbis_get_sample_offset(stb_vorbis *f)
```

**Purpose:** Public API to query current sample position in stream.

**Called by:** 
- User code to track playback position

**Calls:** None (field access only)

**Globals read:** `f->current_loc` (sample position counter)

**Globals mutated:** None

**Side effects:** None; query only.

**Notes:** Returns cumulative decoded sample count.

---

### `stb_vorbis_get_info` (stb_vorbis.cpp:4483-4493)

**Signature:**
```c
stb_vorbis_info stb_vorbis_get_info(stb_vorbis *f)
```

**Purpose:** Public API to retrieve stream metadata (channels, sample rate, block size, memory usage).

**Called by:** 
- User code to query stream properties

**Calls:** None (struct copy)

**Globals read:** `f->channels`, `f->sample_rate`, `f->blocksize_1`, memory counters

**Globals mutated:** None

**Side effects:** None; query only.

**Notes:** Returns copy of metadata structure.

---

### `stb_vorbis_get_error` (stb_vorbis.cpp:4495-4508)

**Signature:**
```c
int stb_vorbis_get_error(stb_vorbis *f)
```

**Purpose:** Public API to retrieve last error code from decoder.

**Called by:** 
- User code to check error status

**Calls:** None (field access)

**Globals read:** `f->error` (error code)

**Globals mutated:** `f->error` (cleared after read)

**Side effects:** Clears error code after returning.

**Notes:** Resets error state; idempotent subsequent calls return VORBIS__no_error.

---

### `stb_vorbis_flush_pushdata` (stb_vorbis.cpp:4510-4520)

**Signature:**
```c
void stb_vorbis_flush_pushdata(stb_vorbis *f)
```

**Purpose:** Public API to clear push-mode data buffer and reset decoder state.

**Called by:** 
- Push-mode users to reset on stream error

**Calls:** None (field resets)

**Globals read:** None

**Globals mutated:** `f->page_*`, `f->packet_*` buffers and state

**Side effects:** Clears buffered data; resets packet/page parsing state.

**Notes:** Used to recover from corrupted data in push mode.

---

### `vorbis_alloc` (stb_vorbis.cpp:4502-4508)

**Signature:**
```c
static stb_vorbis * vorbis_alloc(stb_vorbis *f)
```

**Purpose:** Allocates and returns new decoder instance (copies from temp instance).

**Called by:** 
- `stb_vorbis_open_*()` functions after setup completes

**Calls:** 
- `setup_malloc()` (allocate new decoder)

**Globals read:** None

**Globals mutated:** Allocates new decoder structure

**Side effects:** Copies setup state to newly allocated decoder; enables freeing temp instance.

**Notes:** Part of two-phase initialization (temp → allocated).

---

### `vorbis_search_for_page_pushdata` (stb_vorbis.cpp:4522-4712)

**Signature:**
```c
static int vorbis_search_for_page_pushdata(vorb *f, uint8 *data, int data_len)
```

**Purpose:** Searches for Ogg page boundary in push-mode data buffer.

**Called by:** 
- `stb_vorbis_decode_frame_pushdata()` (line 4613) for page sync

**Calls:** 
- Page parsing functions

**Globals read:** None

**Globals mutated:** Decoder page state, `f->page_crc_tests`

**Side effects:** Finds page boundary or buffers data for next call; returns 1 if page found, 0 if need more data.

**Notes:** Core of push-mode API; incremental page searching.

---

### `stb_vorbis_decode_frame_pushdata` (stb_vorbis.cpp:4613-4682)

**Signature:**
```c
int stb_vorbis_decode_frame_pushdata(
   stb_vorbis *f, const uint8 *datablock, int datablock_length_in_bytes,
   int *channels, float ***output, int *samples
)
```

**Purpose:** Public push-mode API to decode one frame from incrementally supplied data buffer.

**Called by:** 
- User code with streaming data

**Calls:** 
- `vorbis_search_for_page_pushdata()` (find pages)
- Frame decode functions

**Globals read:** None

**Globals mutated:** Decoder state, output buffers

**Side effects:** Processes supplied data; may decode frames or buffer for next call. Returns number of samples decoded (0 if buffering for more data).

**Notes:** Enables low-latency streaming without requiring complete file in memory.

---

### `stb_vorbis_open_pushdata` (stb_vorbis.cpp:4683-4711)

**Signature:**
```c
stb_vorbis *stb_vorbis_open_pushdata(
   const unsigned char *datablock, int datablock_length_in_bytes,
   int *datablock_memory_consumed_in_bytes, int *error,
   const stb_vorbis_alloc *alloc
)
```

**Purpose:** Public push-mode API to initialize decoder from initial Ogg data block.

**Called by:** 
- User code starting push-mode decoding

**Calls:** 
- `vorbis_init()`, `start_decoder()`, `vorbis_alloc()`

**Globals read:** None

**Globals mutated:** Allocates decoder structure

**Side effects:** Creates decoder from partial data; returns NULL on error. Sets `datablock_memory_consumed_in_bytes` to bytes used from input.

**Notes:** First call for push-mode API; subsequent calls use `stb_vorbis_decode_frame_pushdata()`.

---

### `stb_vorbis_get_file_offset` (stb_vorbis.cpp:4713-4728)

**Signature:**
```c
unsigned int stb_vorbis_get_file_offset(stb_vorbis *f)
```

**Purpose:** Public API to query current byte offset in file/stream.

**Called by:** 
- User code to track file position

**Calls:** None (field access or `ftell()`)

**Globals read:** `f->stream` or FILE position

**Globals mutated:** None

**Side effects:** None; query only.

**Notes:** Returns offset for use with seeking.

---

### `vorbis_find_page` (stb_vorbis.cpp:4729-4810)

**Signature:**
```c
static uint32 vorbis_find_page(stb_vorbis *f, uint32 *end, uint32 *last)
```

**Purpose:** Locates next Ogg page in stream and returns start/end offsets.

**Called by:** 
- Seeking functions

**Calls:** 
- Page header parsing

**Globals read:** Stream position

**Globals mutated:** Stream position (advances through pages)

**Side effects:** Scans stream for valid page; sets output offsets and flags.

**Notes:** Used for constructing seek table.

---

### `get_seek_page_info` (stb_vorbis.cpp:4811-4842)

**Signature:**
```c
static int get_seek_page_info(stb_vorbis *f, ProbedPage *z)
```

**Purpose:** Extracts sample count and timestamp from Ogg page for seeking.

**Called by:** 
- Seek table construction

**Calls:** 
- Page parsing, frame decoding (peek mode)

**Globals read:** Page data

**Globals mutated:** `ProbedPage` output structure

**Side effects:** Decodes page header to extract seek information.

**Notes:** Non-destructive page analysis for seek table building.

---

### `go_to_page_before` (stb_vorbis.cpp:4843-4867)

**Signature:**
```c
static int go_to_page_before(stb_vorbis *f, unsigned int limit_offset)
```

**Purpose:** Seeks backward to page before given file offset.

**Called by:** 
- Seeking functions for binary search

**Calls:** 
- Page finding, seeking

**Globals read:** Stream offset

**Globals mutated:** Stream position

**Side effects:** Repositions stream; returns 1 on success, 0 on failure.

**Notes:** Part of seeking algorithm; enables binary search for target sample.

---

### `seek_to_sample_coarse` (stb_vorbis.cpp:4868-5013)

**Signature:**
```c
static int seek_to_sample_coarse(stb_vorbis *f, uint32 sample_number)
```

**Purpose:** Performs coarse seek to approximate sample position by scanning pages.

**Called by:** 
- `stb_vorbis_seek()` and `stb_vorbis_seek_frame()`

**Calls:** 
- `vorbis_find_page()`, `go_to_page_before()`, page info extraction
- `peek_decode_initial()` (frame peeking without commitment)

**Globals read:** Stream pages, seek table

**Globals mutated:** Stream position, decoder state

**Side effects:** Repositions stream near target sample; may leave decoder mid-frame. Returns 1 on success, 0 on failure.

**Notes:** Approximate seeking; requires fine-tuning via frame decoding.

---

### `peek_decode_initial` (stb_vorbis.cpp:5014-5037)

**Signature:**
```c
static int peek_decode_initial(vorb *f, int *p_left_start, int *p_left_end, int *p_right_start, int *p_right_end, int *mode)
```

**Purpose:** Non-destructively reads frame header without consuming bits (for seeking).

**Called by:** 
- `seek_to_sample_coarse()` to peek at frame without commitment

**Calls:** 
- Bit reading (with state save/restore)

**Globals read:** Bit stream state

**Globals mutated:** Temporarily modifies bit state (then restores)

**Side effects:** None after return; bit position restored.

**Notes:** Enables seeking without corrupting decode state.

---

### `stb_vorbis_seek_frame` (stb_vorbis.cpp:5039-5077)

**Signature:**
```c
int stb_vorbis_seek_frame(stb_vorbis *f, unsigned int sample_number)
```

**Purpose:** Public API to seek to specified sample number (frame granularity).

**Called by:** 
- User code for random access

**Calls:** 
- `seek_to_sample_coarse()` (coarse seek)

**Globals read:** None

**Globals mutated:** Stream position, decoder state

**Side effects:** Repositions decoder to frame containing sample. Returns 1 on success, 0 on seek error.

**Notes:** Frame-level seek; may require fine-tuning with frame decoding for exact position.

---

### `stb_vorbis_seek` (stb_vorbis.cpp:5078-5094)

**Signature:**
```c
int stb_vorbis_seek(stb_vorbis *f, unsigned int sample_number)
```

**Purpose:** Public API to seek to specified sample number (sample granularity).

**Called by:** 
- User code for precise random access

**Calls:** 
- `stb_vorbis_seek_frame()` (frame seek)
- Frame decoding to fine-tune position

**Globals read:** None

**Globals mutated:** Stream position, decoder state, output buffers

**Side effects:** Repositions decoder and decodes frames until target sample reached. Returns 1 on success, 0 on error.

**Notes:** Provides sample-accurate seeking via frame decode fine-tuning.

---

### `stb_vorbis_seek_start` (stb_vorbis.cpp:5095-5104)

**Signature:**
```c
void stb_vorbis_seek_start(stb_vorbis *f)
```

**Purpose:** Public API to reset decoder to stream start.

**Called by:** 
- User code to restart playback

**Calls:** 
- `set_file_offset()` (seek to data start)
- Decoder reset

**Globals read:** `f->stream_start`, `f->first_audio_page_offset`

**Globals mutated:** Stream position, decoder state

**Side effects:** Repositions to beginning; next frame decode will start from first audio frame.

**Notes:** Useful for looping or restart functionality.

---

### `stb_vorbis_stream_length_in_samples` (stb_vorbis.cpp:5105-5179)

**Signature:**
```c
unsigned int stb_vorbis_stream_length_in_samples(stb_vorbis *f)
```

**Purpose:** Public API to query total samples in stream.

**Called by:** 
- User code for progress display, duration

**Calls:** 
- Seeking and page scanning to find stream end

**Globals read:** Stream offset, page tables

**Globals mutated:** May temporarily reposition stream (then restore)

**Side effects:** May perform seeking to scan stream end; restores decoder position.

**Notes:** Potentially expensive operation (scans to stream end); may cache result.

---

### `stb_vorbis_stream_length_in_seconds` (stb_vorbis.cpp:5180-5186)

**Signature:**
```c
float stb_vorbis_stream_length_in_seconds(stb_vorbis *f)
```

**Purpose:** Public API to query total duration in seconds.

**Called by:** 
- User code for duration display

**Calls:** 
- `stb_vorbis_stream_length_in_samples()` (get sample count)

**Globals read:** Sample rate

**Globals mutated:** None

**Side effects:** None directly; may cause seeking if samples function scans stream.

**Notes:** Converts sample count to seconds using stream sample rate.

---

### `stb_vorbis_get_frame_float` (stb_vorbis.cpp:5187-5210)

**Signature:**
```c
int stb_vorbis_get_frame_float(stb_vorbis *f, int *channels, float ***output)
```

**Purpose:** Public API to decode next frame and return float PCM samples.

**Called by:** 
- User code for pull-mode decoding to float

**Calls:** 
- `vorbis_decode_packet()` (frame decode)

**Globals read:** Decoder state

**Globals mutated:** Output buffers, stream position

**Side effects:** Decodes one frame; advances stream. Returns sample count or 0 on error. Sets `*output` to pointer to float arrays per channel.

**Notes:** Non-interleaved output; caller must manage pointers to channel data. Returns array of pointers where output[i] is float array for channel i.

---

### `stb_vorbis_open_file_section` (stb_vorbis.cpp:5211-5230)

**Signature:**
```c
stb_vorbis * stb_vorbis_open_file_section(FILE *file, int close_on_free, int *error, const stb_vorbis_alloc *alloc, unsigned int length)
```

**Purpose:** Public API to create decoder from FILE* for specified byte range.

**Called by:** 
- User code for file-based decoding with partial read

**Calls:** 
- `vorbis_init()`, `start_decoder()`, `vorbis_alloc()`

**Globals read:** None

**Globals mutated:** Allocates decoder structure

**Side effects:** Initializes decoder from file section. Returns decoder pointer or NULL on error. Sets `*error` code on failure.

**Notes:** Enables decoding embedded Vorbis streams in container formats.

---

### `stb_vorbis_open_file` (stb_vorbis.cpp:5232-5240)

**Signature:**
```c
stb_vorbis * stb_vorbis_open_file(FILE *file, int close_on_free, int *error, const stb_vorbis_alloc *alloc)
```

**Purpose:** Public API to create decoder from FILE*.

**Called by:** 
- User code for file-based decoding

**Calls:** 
- `stb_vorbis_open_file_section()` (delegates to section variant with full file length)

**Globals read:** None

**Globals mutated:** Allocates decoder structure

**Side effects:** Creates decoder from full file. Returns decoder or NULL on error.

**Notes:** Wrapper around `stb_vorbis_open_file_section()` for convenience.

---

### `stb_vorbis_open_filename` (stb_vorbis.cpp:5242-5251)

**Signature:**
```c
stb_vorbis * stb_vorbis_open_filename(const char *filename, int *error, const stb_vorbis_alloc *alloc)
```

**Purpose:** Public API to create decoder from filename.

**Called by:** 
- User code for file-path-based decoding

**Calls:** 
- `fopen()` (open file)
- `stb_vorbis_open_file()` with `close_on_free=TRUE`

**Globals read:** Filesystem

**Globals mutated:** Allocates decoder; opens file

**Side effects:** Opens file and creates decoder. File will be closed by `stb_vorbis_close()`. Returns decoder or NULL on error.

**Notes:** Most convenient entry point for file-based use.

---

### `stb_vorbis_open_memory` (stb_vorbis.cpp:5253-5280)

**Signature:**
```c
stb_vorbis * stb_vorbis_open_memory(const unsigned char *data, int len, int *error, const stb_vorbis_alloc *alloc)
```

**Purpose:** Public API to create decoder from in-memory buffer.

**Called by:** 
- User code for memory-based decoding

**Calls:** 
- `vorbis_init()`, `start_decoder()`, `vorbis_alloc()`, `vorbis_pump_first_frame()`

**Globals read:** None

**Globals mutated:** Allocates decoder structure

**Side effects:** Creates decoder from buffer data. Returns decoder or NULL on error. Sets `*error` code on failure.

**Notes:** Does not copy data; decoder maintains pointer to caller's buffer (must remain valid).

---

## Sample Format Conversion Functions

### `copy_samples` (stb_vorbis.cpp:5315-5327)

**Signature:**
```c
static void copy_samples(short *dest, float *src, int len)
```

**Purpose:** Converts float PCM samples to 16-bit signed integer with clipping.

**Called by:** 
- Short format output functions

**Calls:** 
- None (only arithmetic and type conversion)

**Globals read:** None

**Globals mutated:** Destination buffer filled

**Side effects:** Writes converted samples.

**Notes:** Implements clipping to ±32767 range for 16-bit output.

---

### `compute_samples` (stb_vorbis.cpp:5328-5352)

**Signature:**
```c
static void compute_samples(int mask, short *output, int num_c, float **data, int d_offset, int len)
```

**Purpose:** Generates mono or stereo 16-bit output with channel mixing per PLAYBACK_* flags.

**Called by:** 
- `stb_vorbis_get_frame_short()` (line 5408) for standard playback channels

**Calls:** 
- `copy_samples()` (convert floats)

**Globals read:** None

**Globals mutated:** Output buffer filled

**Side effects:** Mixes channels and converts to 16-bit output.

**Notes:** Handles mono upmix and stereo downmix based on channel mask.

---

### `compute_stereo_samples` (stb_vorbis.cpp:5353-5391)

**Signature:**
```c
static void compute_stereo_samples(short *output, int num_c, float **data, int d_offset, int len)
```

**Purpose:** Generates stereo 16-bit output from multi-channel float data with downmixing.

**Called by:** 
- `convert_channels_short_interleaved()` (line 5418) for stereo downmix

**Calls:** 
- `copy_samples()` (convert floats)

**Globals read:** `channel_position[7][6]` downmix matrix

**Globals mutated:** Output buffer filled

**Side effects:** Downmixes multi-channel to stereo 16-bit.

**Notes:** Implements ITU downmixing rules for multi-channel to stereo.

---

### `convert_samples_short` (stb_vorbis.cpp:5392-5407)

**Signature:**
```c
static void convert_samples_short(int buf_c, short **buffer, int b_offset, int data_c, float **data, int d_offset, int samples)
```

**Purpose:** Converts non-interleaved float to non-interleaved 16-bit PCM.

**Called by:** 
- `stb_vorbis_get_frame_short()` (line 5408)

**Calls:** 
- `copy_samples()` (per-channel conversion)

**Globals read:** None

**Globals mutated:** Output buffers filled

**Side effects:** Fills output buffer arrays with converted samples.

**Notes:** Handles arbitrary channel count and count mismatch (mono to stereo, etc.).

---

### `convert_channels_short_interleaved` (stb_vorbis.cpp:5418-5442)

**Signature:**
```c
static void convert_channels_short_interleaved(int buf_c, short *buffer, int data_c, float **data, int d_offset, int len)
```

**Purpose:** Converts float PCM to interleaved 16-bit output with channel downmixing.

**Called by:** 
- `stb_vorbis_get_frame_short_interleaved()` (line 5444)

**Calls:** 
- `compute_stereo_samples()` (channel downmixing)
- Sample conversion logic

**Globals read:** Channel downmix configuration

**Globals mutated:** Output buffer filled

**Side effects:** Writes interleaved 16-bit samples; handles downmixing.

**Notes:** Converts non-interleaved float to interleaved short with downmixing support.

---

## Public Short Format APIs

### `stb_vorbis_get_frame_short` (stb_vorbis.cpp:5408-5443)

**Signature:**
```c
int stb_vorbis_get_frame_short(stb_vorbis *f, int num_c, short **buffer, int num_samples)
```

**Purpose:** Public API to decode next frame to non-interleaved 16-bit PCM.

**Called by:** 
- User code for pull-mode short format decoding

**Calls:** 
- `stb_vorbis_get_frame_float()` (decode to float)
- `convert_samples_short()` (convert to short)

**Globals read:** None

**Globals mutated:** Output buffers, stream position

**Side effects:** Decodes one frame and converts to 16-bit. Returns sample count or 0 on error.

**Notes:** Non-interleaved output (one buffer array per channel).

---

### `stb_vorbis_get_frame_short_interleaved` (stb_vorbis.cpp:5444-5456)

**Signature:**
```c
int stb_vorbis_get_frame_short_interleaved(stb_vorbis *f, int num_c, short *buffer, int num_shorts)
```

**Purpose:** Public API to decode next frame to interleaved 16-bit PCM.

**Called by:** 
- User code for pull-mode short interleaved decoding

**Calls:** 
- `stb_vorbis_get_frame_float()` (decode to float)
- `convert_channels_short_interleaved()` (convert to interleaved short)

**Globals read:** None

**Globals mutated:** Output buffer, stream position

**Side effects:** Decodes one frame to interleaved 16-bit. Returns sample count or 0 on error.

**Notes:** Interleaved output (samples LRLRLR... for stereo).

---

### `stb_vorbis_get_samples_short_interleaved` (stb_vorbis.cpp:5457-5477)

**Signature:**
```c
int stb_vorbis_get_samples_short_interleaved(stb_vorbis *f, int channels, short *buffer, int num_shorts)
```

**Purpose:** Public API to decode multiple frames into interleaved 16-bit buffer.

**Called by:** 
- User code for buffered short decoding

**Calls:** 
- `stb_vorbis_get_frame_short_interleaved()` (decode frame by frame)

**Globals read:** None

**Globals mutated:** Output buffer, stream position

**Side effects:** Decodes until buffer filled or stream end. Returns total samples decoded.

**Notes:** Convenience wrapper decoding multiple frames into contiguous buffer.

---

### `stb_vorbis_get_samples_short` (stb_vorbis.cpp:5478-5497)

**Signature:**
```c
int stb_vorbis_get_samples_short(stb_vorbis *f, int channels, short **buffer, int len)
```

**Purpose:** Public API to decode multiple frames into non-interleaved 16-bit buffers.

**Called by:** 
- User code for buffered non-interleaved short decoding

**Calls:** 
- `stb_vorbis_get_frame_short()` (decode frame by frame)

**Globals read:** None

**Globals mutated:** Output buffers, stream position

**Side effects:** Decodes until buffers filled or stream end. Returns total samples decoded.

**Notes:** Convenience wrapper for non-interleaved short format.

---

### `stb_vorbis_decode_filename` (stb_vorbis.cpp:5498-5535)

**Signature:**
```c
int stb_vorbis_decode_filename(const char *filename, int *channels, int *sample_rate, short **output)
```

**Purpose:** Public all-in-one API to fully decode Vorbis file to 16-bit short array.

**Called by:** 
- User code for complete file decode

**Calls:** 
- `stb_vorbis_open_filename()` (open file)
- `stb_vorbis_get_info()` (get metadata)
- `stb_vorbis_stream_length_in_samples()` (allocate buffer)
- `stb_vorbis_get_samples_short_interleaved()` (decode all)
- `stb_vorbis_close()` (cleanup)
- `malloc()`, `realloc()` (memory management)

**Globals read:** Filesystem

**Globals mutated:** Allocates output buffer; opens/closes file

**Side effects:** Allocates large buffer for entire decoded stream. Returns total samples or error code. Sets `*channels` and `*sample_rate`.

**Notes:** Convenience function; not suitable for real-time or large files (buffers entire decoded stream in memory).

---

### `stb_vorbis_decode_memory` (stb_vorbis.cpp:5538-5576)

**Signature:**
```c
int stb_vorbis_decode_memory(const uint8 *mem, int len, int *channels, int *sample_rate, short **output)
```

**Purpose:** Public all-in-one API to fully decode Vorbis buffer to 16-bit short array.

**Called by:** 
- User code for complete buffer decode

**Calls:** 
- `stb_vorbis_open_memory()` (create decoder)
- `stb_vorbis_get_info()` (get metadata)
- `stb_vorbis_stream_length_in_samples()` (allocate buffer)
- `stb_vorbis_get_samples_short_interleaved()` (decode all)
- `stb_vorbis_close()` (cleanup)
- `malloc()`, `realloc()` (memory management)

**Globals read:** None

**Globals mutated:** Allocates output buffer

**Side effects:** Allocates large buffer for entire decoded stream. Returns total samples or error code. Sets `*channels` and `*sample_rate`.

**Notes:** Convenience function analogous to `stb_vorbis_decode_filename()` but for memory input.

---

## Public Float Format APIs

### `stb_vorbis_get_samples_float_interleaved` (stb_vorbis.cpp:5578-5604)

**Signature:**
```c
int stb_vorbis_get_samples_float_interleaved(stb_vorbis *f, int channels, float *buffer, int num_floats)
```

**Purpose:** Public API to decode multiple frames into interleaved float buffer.

**Called by:** 
- User code for float format decoding

**Calls:** 
- `stb_vorbis_get_frame_float()` (decode frame by frame)

**Globals read:** None

**Globals mutated:** Output buffer, stream position

**Side effects:** Decodes until buffer filled or stream end. Returns total samples decoded.

**Notes:** Convenience wrapper for interleaved float output; preserves full precision.

---

### `stb_vorbis_get_samples_float` (stb_vorbis.cpp:5605-5625)

**Signature:**
```c
int stb_vorbis_get_samples_float(stb_vorbis *f, int channels, float **buffer, int num_samples)
```

**Purpose:** Public API to decode multiple frames into non-interleaved float buffers.

**Called by:** 
- User code for non-interleaved float decoding

**Calls:** 
- `stb_vorbis_get_frame_float()` (decode frame by frame)

**Globals read:** None

**Globals mutated:** Output buffers, stream position

**Side effects:** Decodes until buffers filled or stream end. Returns total samples decoded.

**Notes:** Convenience wrapper for non-interleaved float output; preserves full precision.

---

## Summary

**Total Functions:** 111
- **Public API:** 25 (stb_vorbis_* functions)
- **Static Helpers:** 86 (internal implementation)

**Major Subsystems:**
1. **Bit I/O** — get_bits, get8, getn, bitreverse
2. **Huffman Decoding** — codebook_decode_scalar, codebook_decode, compute_sorted_huffman
3. **FFT/IMDCT** — inverse_mdct, imdct_step3_*, compute_twiddle_factors
4. **Frame Decoding** — vorbis_decode_packet, vorbis_decode_packet_rest, do_floor, decode_residue
5. **Setup/Initialization** — start_decoder, vorbis_init, init_blocksize
6. **Seeking** — stb_vorbis_seek, seek_to_sample_coarse, vorbis_find_page
7. **Format Conversion** — copy_samples, convert_samples_short, compute_stereo_samples
8. **Memory Management** — setup_malloc, setup_temp_malloc, vorbis_deinit

**Key Hot Paths:**
- `get_bits()` — called billions of times per stream
- `inverse_mdct()` — major computational kernel
- `decode_residue()` — spectrum correction
- `do_floor()` — masking curve synthesis

**Notable Design Patterns:**
- Two-phase initialization (temp instance → allocated)
- Push-mode and pull-mode APIs
- Dual memory allocation (pre-allocated vs. heap)
- Non-interleaved float internally, interleaved short for output
- Deferred seeking (coarse → fine-tuning via frame decode)

