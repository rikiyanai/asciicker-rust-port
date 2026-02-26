# Audit: Unknown Audio Details

**Source file:** `/Users/r/Downloads/asciicker-Y9-2/audio.cpp`  
**Analysis date:** 2026-02-20  
**Total lines:** 1374

---

## Summary

This audit documents the audio system in asciicker, covering stb_vorbis integration, sample management, marker formats, and a discovered memory leak bug.

---

## 1. stb_vorbis Integration Details

### Overview

The audio system uses **stb_vorbis** (a public domain Ogg Vorbis decoding library) for streaming audio decode. The main decode function is `XOgg()` at **line 218-346**.

### API Calls Used

| Function | Purpose | Line Reference |
|----------|---------|-----------------|
| `stb_vorbis_open_memory()` | Open Ogg stream from memory buffer | Line 225 |
| `stb_vorbis_get_info()` | Get sample rate and channel count | Line 238 |
| `stb_vorbis_stream_length_in_samples()` | Get total sample count | Line 234 |
| `stb_vorbis_get_frame_float()` | Stream decode next frame | Line 284 |
| `stb_vorbis_get_markers()` | Custom extension for embedded timing markers | Line 241 |
| `stb_vorbis_close()` | Free decoder resources | Line 342 |

### Stereo Handling

Mono Ogg files are duplicated to both L/R channels for uniform stereo output. This is handled in the decode loop at **lines 286-307**:

```cpp
if (chn==1)
{
    // mono -> L=0, R=0
    for (int i=0; i<len; i++)
    {
        // ... conversion logic ...
        dec[offs++] = m;  // Left channel
        dec[offs++] = m;  // Right channel (duplicate)
    }
}
```

### Output Format

- **Sample rate:** 44100 Hz (hardcoded in all backends)
- **Bit depth:** 16-bit signed integer
- **Channels:** 2 (stereo)

---

## 2. Sample Loading/Unloading

### Data Structures

```cpp
#define MAX_SAMPLES 64
static int16_t* lib_sample_data[MAX_SAMPLES] = {0};  // Decoded PCM buffers
static int lib_sample_len[MAX_SAMPLES] = {0};         // Sample count per buffer

// Hash table for filename → sample_id lookup
#define HASH_MAKS (MAX_SAMPLES-1)
static SampleHash* sample_hash[HASH_MAKS+1] = {0};
```

### Loading Flow

1. **`LoadAllSamples()` (line 481-488)** - Pre-loads all .ogg files at init time:
   - Scans `samples/` directory
   - Calls `LoadSample()` for each file
   - Pre-loading ensures zero-latency playback (audio callbacks run on separate thread)

2. **`LoadSample()` (line 413-467)** - Loads a single sample:
   - Checks hash table for existing entry
   - Opens `./samples/<name>` file
   - Allocates buffer and reads file
   - Calls `XOgg()` to decode Ogg to PCM
   - Adds to hash table with djb2 hash

3. **`FindSample()` (line 363-387)** - O(1) lookup using djb2 hash:
   ```cpp
   uint32_t hash = 5381;
   const char* n = name;
   while (unsigned int c = *n++)
       hash = ((hash << 5) + hash) + c;
   ```

4. **`GetSampleID()` (line 490-495)** - Maps `AUDIO_FILE` enum to runtime sample index:
   ```cpp
   static const char* sample_names[] = // IN ORDER OF enum AUDIO_FILE
   {
       "forest.ogg",
       "footsteps.ogg",
       0
   };
   ```

### Known Sample Files

Only two audio files are loaded:
- `forest.ogg` - Ambient forest sound (loops continuously)
- `footsteps.ogg` - Footstep sounds with material markers

### **CRITICAL BUG: No Sample Unload Function**

**There is NO function to unload/free individual samples!**

- Memory is allocated in `XOgg()` via `malloc()` (line 257)
- `FreeAudio()` functions only free audio backend resources:
  - CoreAudio: `AudioQueueDispose()` (line 777)
  - PulseAudio: `pa_stream_unref()`, `pa_context_unref()` (lines 881-890)
  - SDL: `SDL_CloseAudio()` (line 1038)
  - Emscripten: `audio_ctx.close()` (line 1203)
- **No code ever frees `lib_sample_data[]` buffers**
- This is a **memory leak** - all decoded samples remain in memory for the entire program lifetime

---

## 3. Marker Format

### Purpose

Embedded timing markers enable footstep variation by material and foot (left/right). Markers are stored after PCM data for chunk-based playback.

### Format Specification

- **Encoding:** Tab-separated float pairs (start/end timestamps in seconds)
- **Example:** `"0.0\t0.5\n0.5\t1.0"` = 2 markers at 0-0.5s, 0.5-1.0s
- **Storage:** After PCM data in the same allocation

### Storage Layout

```
lib_sample_data[index] points to:
+------------------------------------------+
| PCM data (int16_t stereo samples)        |
+------------------------------------------+
| Marker count (int32_t at mrk[-1])        |
+------------------------------------------+
| Marker pairs (int32_t[2] per marker)     |
|   [start_0, end_0, start_1, end_1, ...]  |
+------------------------------------------+
```

### Parsing Code (lines 241-277)

```cpp
// Get embedded markers (custom extension)
const char* markers = stb_vorbis_get_markers(ogg);

// Parse markers
if (markers)
{
    for (int i=0; i<num_markers; i++)
    {
        float a=0, b=0;
        sscanf(ptr, "%f\t%f", &a, &b);
        
        // Convert from seconds to sample positions
        mrk[2*i+0] = (int)floor(a*freq+0.5);  // start
        mrk[2*i+1] = (int)floor(b*freq+0.5);  // end
    }
}
```

### Usage in Playback (lines 554-562)

```cpp
if (size>=16 && msg[3]>=0)
{
    int32_t* marker = (int32_t*)(lib_sample_data[tr->sample_id] + 2*tr->sample_end);
    if (*marker > msg[3])
    {
        marker = marker + 1 + 2 * msg[3];
        tr->sample_pos = marker[0];  // marker start
        tr->sample_end = marker[1];  // marker end
    }
}
```

### Chunk Indexing Formula

The chunk index is calculated as: `2*material + (foot & 1)`

- `material * 2` = base offset for material (each material has 2 chunks: left/right)
- `+ (foot & 1)` = add 0 for left foot, 1 for right foot

**Example:** material=3 (grass), foot=1 (right) → chunk=7

---

## 4. Sample Unload Bug Details

### Bug Description

**Type:** Memory Leak  
**Severity:** Medium (samples accumulate, never freed)  
**Location:** No unload function exists

### Root Cause

1. `LoadSample()` → `XOgg()` allocates PCM buffer with `malloc()` at line 257:
   ```cpp
   int16_t* dec = (int16_t*)malloc(sizeof(int16_t)*2*size + sizeof(int32_t)*(num_markers*2+1));
   ```

2. `lib_sample_data[index] = dec;` stores the pointer (line 344)

3. `FreeAudio()` does NOT free these buffers - it only frees backend resources

### Affected Backends

All five audio backends have this issue:
- CoreAudio (macOS) - line 774-779
- PulseAudio (Linux) - line 873-892
- SDL Audio - line 1036-1039
- Emscripten AudioWorklet - line 1199-1209
- Emscripten ScriptNode - line 1199-1209

### Recommended Fix

Add a function to free all sample data:

```cpp
static void FreeAllSamples()
{
    for (int i = 0; i < MAX_SAMPLES; i++)
    {
        if (lib_sample_data[i])
        {
            free(lib_sample_data[i]);
            lib_sample_data[i] = nullptr;
        }
    }
}
```

Then call `FreeAllSamples()` at the start of each platform's `FreeAudio()` implementation.

---

## Additional Findings

### Threading Model

- **Desktop (CoreAudio/PulseAudio/SDL):** Separate audio callback thread with mutex-protected command queue
- **Web AudioWorklet:** Dedicated worklet thread with postMessage
- **Web ScriptNode:** Main thread (synchronous)

### Mixing Engine

- 16-track mixer (`PlyTrack[PLY_TRACKS]`)
- Sum with int32_t accumulator for headroom
- Saturate to [-32767, +32767] to prevent clipping

### Audio Backends

| Backend | Platform | API |
|---------|----------|-----|
| CoreAudio | macOS/iOS | AudioQueue |
| PulseAudio | Linux | pa_threaded_mainloop |
| SDL Audio | Cross-platform fallback | SDL2 callback |
| AudioWorklet | Modern web browsers | AudioWorkletProcessor |
| ScriptNode | Legacy web browsers | ScriptProcessorNode |

---

## References

- Line 43-62: stb_vorbis integration comments
- Line 63-77: Sample library comments
- Line 218-346: `XOgg()` function - Ogg decoding
- Line 413-467: `LoadSample()` function
- Line 518-637: `DriverAudioCmd()` - command processing
- Line 645-703: `DriverAudioCB()` - audio mixing callback
