# Audio System Architecture (`audio.cpp`)

Platform-agnostic audio subsystem with stb_vorbis Ogg decoding, sample library management, and 16-track mixing engine. Supports 5 audio backends: CoreAudio (macOS), PulseAudio (Linux), SDL (fallback), AudioWorklet (web modern), ScriptNode (web legacy).

## Global State

| Variable | Type | Purpose |
|----------|------|---------|
| `lib_sample_data[MAX_SAMPLES]` | `int16_t*[]` | Decoded PCM stereo buffers |
| `lib_sample_len[MAX_SAMPLES]` | `int[]` | Sample frame counts |
| `ply_track[PLY_TRACKS]` | `PlyTrack[]` | 16 active playback tracks |
| `sample_hash[HASH_MAKS+1]` | `SampleHash*[]` | Filename→sample_id hash table |
| `volume` | `int32_t` | Global volume (0-32768) |
| `ply_forest_id` | `int` | Background forest loop sample id |
| `call_head`, `call_tail` | `CallQueue*` | Desktop command queue (mutex-protected) |
| `call_mutex` | `std::mutex` | Desktop thread synchronization |
| `samples` | `int` | Current sample count (incremented on load) |
| `sample_ids[AUDIO_FILES]` | `int[]` | AUDIO_FILE enum→sample_id mapping |

---

### `AudioMute` (audio.cpp:166-170)

**Signature:** `void AudioMute(bool mute)`

**Purpose:** Toggle global audio mute by setting volume to 0 (mute=true) or 32768 (mute=false).

**Called by:** grep-verified callers:
- `game.cpp:637` — Game toggle mute from menu
- `game.cpp:10911` — Game mute hotkey handler
- `mainmenu.cpp:2156` — Main menu mute toggle

**Calls:** `CallAudio()` with 2-byte volume value

**Globals mutated:** None directly; queues command to `CallAudio()` which will eventually write `volume`

**Globals read:** None

**Side effects:** Enqueues audio command; blocks until queue lock acquired (desktop) or posts to worklet (web)

**Notes:** Wrapper that constructs 16-bit volume value (0 or 32768) and calls `CallAudio()` which either enqueues it (desktop) or forwards to worklet (web). On desktop, actual volume mutation happens in `DriverAudioCmd()` callback thread.

---

### `AudioWalk` (audio.cpp:178-204)

**Signature:** `void AudioWalk(int foot, int volume, const SpriteReq* req, int material)`

**Purpose:** Play material-based footstep sound effect with foot/material variation.

**Called by:** grep-verified callers:
- `physics.cpp:2030` — Player landing (foot=0, auto-alternate)
- `physics.cpp:2047` — Left footstep cycle
- `physics.cpp:2050` — Right footstep cycle

**Calls:** `GetSampleID(FOOTSTEPS)`, `CallAudio()`

**Globals mutated:** `track` (static, round-robin index for track selection)

**Globals read:** None (besides module-statics)

**Side effects:** Enqueues 16-byte audio command to backend; increments static track counter mod PLY_TRACKS

**Notes:** Accepts foot parameter (0=auto-alternate, 1=left, 2=right) and calculates `sample_chunk = 2*material+(foot&1)` for marker-based playback of material-specific sound variants. Auto-alternation uses static `land` flag. Command format: `[track_id, sample_id, volume, sample_chunk]`. Track index round-robins across 16 tracks to support simultaneous footsteps.

---

### `XOgg` (audio.cpp:218-346)

**Signature:** `extern "C" void XOgg(int index, const uint8_t* data, int ogg_size)`

**Purpose:** Decode Ogg Vorbis stream to 16-bit stereo PCM with marker extraction.

**Called by:** grep-verified callers:
- `audio.cpp:450` — `LoadSample()` post-file-load decoding
- `game_web.cpp` (via JS at line 1315/1323) — Web ScriptNode sample loading

**Calls:** `stb_vorbis_open_memory()`, `stb_vorbis_get_info()`, `stb_vorbis_stream_length_in_samples()`, `stb_vorbis_get_frame_float()`, `stb_vorbis_get_markers()`, `stb_vorbis_close()`

**Globals mutated:** `lib_sample_data[index]`, `lib_sample_len[index]`

**Globals read:** None (pure decode, no side effects beyond output arrays)

**Side effects:** Allocates heap buffer (`malloc`) for PCM + marker data; clears on decode failure by setting `lib_sample_data[index]=0`

**Notes:** [DEPENDENCY:STB_VORBIS] Mono files duplicated to stereo (both L/R channels). Marker format: tab-separated float pairs (start\tend timestamps) extracted and converted to sample indices via `(int)floor(a*freq+0.5)`. Marker data stored after PCM: `[marker_count, marker_0_start, marker_0_end, ...]`. Frame loop clips output to prevent overflow; overflow count calculated via `2*size` byte limit. Returns silently if `index >= MAX_SAMPLES` (out of bounds).

---

### `FindSample` (audio.cpp:363-387)

**Signature:** `static int FindSample(const char* name, uint32_t* h, int* l)`

**Purpose:** Look up sample by filename using djb2 hash; output optional hash and length.

**Called by:** grep-verified callers:
- `audio.cpp:394` — `Sample()` (web) duplicate check
- `audio.cpp:418` — `LoadSample()` cache lookup

**Calls:** `strcmp()` for hash collision verification

**Globals mutated:** None (read-only query)

**Globals read:** `sample_hash[]`

**Side effects:** None

**Notes:** djb2 hash algorithm (`hash = ((hash << 5) + hash) + c`). Returns sample_id on hit (-1 on miss). Output parameters `h` and `l` are optional (checked against null); `l` returns `(n-name)` byte length for later malloc. Hash collision resolution via linked-list chaining (`buck->next`). Time complexity O(1) average, O(n) worst-case (collision chain).

---

### `LoadSample` (audio.cpp:413-467)

**Signature:** `static int LoadSample(const char* name)`

**Purpose:** Load Ogg file from `samples/` directory, decode via `XOgg()`, register in hash table.

**Called by:** grep-verified callers:
- `audio.cpp:484` — `LoadAllSamples()` batch loader

**Calls:** `FindSample()`, `fopen()`, `fseek()`, `ftell()`, `fread()`, `fclose()`, `malloc()`, `free()`, `XOgg()`, `strcmp()`

**Globals mutated:** `sample_ids[]` (via `samples` counter increment), `samples`, `sample_hash[]` (hash table insertion)

**Globals read:** `base_path` (extern, set by platform startup), `MAX_SAMPLES`, `HASH_MAKS`

**Side effects:** Allocates file buffer (`malloc`), reads file from disk, deallocates buffer after decode

**Notes:** [DEPENDENCY:OGG] Files must be Ogg Vorbis format. Returns cached sample_id if already loaded (hash hit). On miss, constructs path `{base_path}samples/{name}`, opens file, reads to memory, calls `XOgg()`. Fails silently if file not found, sample array full, or decode fails. On Emscripten, returns -1 if not in hash (files pre-loaded by JS loader). `strcpy()` used for name storage in hash table (vulnerable if name > allocated space, but size is `sizeof(SampleHash)+len` where `len = (n-name)` from `FindSample()`).

---

### `LoadAllSamples` (audio.cpp:481-488)

**Signature:** `static void LoadAllSamples()`

**Purpose:** Pre-load all .ogg files into memory from `sample_names[]` array.

**Called by:** grep-verified callers:
- `audio.cpp:783` — `InitAudio()` (CoreAudio)
- `audio.cpp:896` — `InitAudio()` (PulseAudio)
- `audio.cpp:1060` — `InitAudio()` (SDL)
- `audio.cpp:1336` — `InitAudio()` (Emscripten)

**Calls:** `LoadSample()`, `GetSampleID()`, `CallAudio()`

**Globals mutated:** `lib_sample_data[]`, `lib_sample_len[]`, `sample_ids[]`, `sample_hash[]`, `samples` (via `LoadSample()`)

**Globals read:** `sample_names[]`

**Side effects:** Disk I/O for all samples; queues forest sample id to audio backend

**Notes:** Iterates `sample_names[]` (defined at line 469-474: "forest.ogg", "footsteps.ogg", NULL terminator). Calls `LoadSample()` for each, stores result in `sample_ids[i]`. After loading, retrieves FOREST sample id via `GetSampleID()` and enqueues 4-byte command (forest_id) to backend via `CallAudio()`. Purpose: cache forest ambience loop and signal backend to start background playback.

---

### `GetSampleID` (audio.cpp:490-495)

**Signature:** `int GetSampleID(AUDIO_FILE af)`

**Purpose:** Map AUDIO_FILE enum to loaded sample id (bounds-checked).

**Called by:** grep-verified callers:
- `audio.cpp:194` — `AudioWalk()` fetch FOOTSTEPS sample
- `audio.cpp:486` — `LoadAllSamples()` fetch FOREST sample

**Calls:** None

**Globals mutated:** None

**Globals read:** `sample_ids[]`, `AUDIO_FILES` (enum max)

**Side effects:** None

**Notes:** Simple array lookup with bounds check (`af < 0 || af >= AUDIO_FILES`). Returns -1 on invalid index. Enum defined in `audio.h` with `AUDIO_FILES` as count. Maps from public AUDIO_FILE enum to runtime sample id slot.

---

### `DriverAudioCmd` (audio.cpp:518-637)

**Signature:** `void DriverAudioCmd(void* userdata, const uint8_t* data, int size)`

**Purpose:** Process audio playback commands from game code (track, sample, volume, markers).

**Called by:** grep-verified callers:
- `audio.cpp:828` — `coreaudio_cb()` dequeue commands
- `audio.cpp:995` — `stream_write_cb()` (PulseAudio) dequeue commands
- `audio.cpp:1046` — `SDLAudioCB()` dequeue commands
- `audio.cpp:1112` — `Call()` (web AudioWorklet) synchronous call
- `audio.cpp:1164` — `CallAudio()` (web ScriptNode) synchronous call

**Calls:** None (data-only processing)

**Globals mutated:** `ply_forest_id`, `volume`, `ply_track[].sample_id`, `ply_track[].sample_vol`, `ply_track[].sample_pos`, `ply_track[].sample_end`

**Globals read:** `lib_sample_data[]`, `lib_sample_len[]`, `PLY_TRACKS`, `MAX_SAMPLES`

**Side effects:** Modifies track playback state; performs marker lookup into `lib_sample_data` to extract chunk boundaries

**Notes:** Dispatches by command size: 4 bytes = set forest sample id, 2 bytes = set global volume, 12+ bytes = configure track. Track command format: `[track_idx, sample_id, volume, optional_chunk_idx]`. For 16+ byte commands with valid chunk index, looks up marker array stored after PCM data: `marker_ptr = (int32_t*)(lib_sample_data[sample_id] + 2*sample_end); marker_ptr[2*chunk_idx + 0/1]` gives start/end positions. Chunk boundaries enable sub-region playback (material/foot variation). Extensive comment block (lines 568-637) documents future planned commands (pan, rot, loop, pause, events, scripting) but not yet implemented. Currently only sizes 2, 4, 12+ are processed; other sizes silently ignored.

---

### `DriverAudioCB` (audio.cpp:645-703)

**Signature:** `void DriverAudioCB(void* userdata, int16_t buffer[], int frames)`

**Purpose:** Mix all 16 active tracks + forest loop to output buffer with saturation.

**Called by:** grep-verified callers:
- `audio.cpp:835` — `coreaudio_cb()` fill buffer
- `audio.cpp:1005` — `stream_write_cb()` (PulseAudio) fill buffer
- `audio.cpp:1055` — `SDLAudioCB()` fill buffer
- `audio.cpp:1106` — `Proc()` (web AudioWorklet) fill buffer
- `audio.cpp:1145` — `Audio()` (web ScriptNode) fill buffer

**Calls:** `memset()`

**Globals mutated:** `forest_pos` (static, sample playback position), `ply_track[].sample_id`, `ply_track[].sample_pos`

**Globals read:** `ply_forest_id`, `lib_sample_data[]`, `lib_sample_len[]`, `ply_track[]`, `volume`

**Side effects:** Writes 16-bit stereo PCM to output buffer; loops forest sample; marks finished tracks as inactive

**Notes:** [DATA-CONTRACT:PCM] Buffer is stereo int16_t (2 channels per frame, 4 bytes per frame). Forest loop: static `forest_pos` cycles through `[0, lib_sample_len[ply_forest_id])` with volume attenuation `(data[pos*2] * volume) >> 15`. Per-track mixing: `int32_t` accumulator sums all active tracks, saturates to `[-32767, +32767]` to prevent clipping. Track volume combines track `sample_vol` and global `volume`: `vol = (tr->sample_vol * volume) >> 15`. Division by 65535 used for per-sample mixing ( be legacy; potential precision issue). Tracks that reach `sample_end` are marked inactive (`sample_id = -1`) and skipped on next frame. Forest loop always plays if `ply_forest_id >= 0`.

---

### `CallAudio` (audio.cpp:740-755, 1155-1197)

**Signature:** `void CallAudio(const uint8_t* data, int size)` (desktop/web variants)

**Purpose:** Enqueue or execute audio command depending on backend (mutex queue on desktop, direct call/postMessage on web).

**Called by:** grep-verified callers:
- `audio.cpp:169` — `AudioMute()` enqueue volume command
- `audio.cpp:203` — `AudioWalk()` enqueue track command
- `audio.cpp:487` — `LoadAllSamples()` enqueue forest id

**Calls:** `malloc()`, `memcpy()`, `std::lock_guard<std::mutex>`, `EM_ASM()` (web)

**Globals mutated:** `call_head`, `call_tail` (desktop), `audio_mode` (web)

**Globals read:** `call_mutex` (desktop), `audio_mode` (web)

**Side effects:** Desktop: allocates heap `CallQueue` node, acquires mutex, appends to queue. Web: EM_ASM inline assembly posts to worklet or calls directly.

**Notes:** Two implementations: Desktop (non-EMSCRIPTEN) uses mutex-protected linked queue (`CallQueue` struct). Web (EMSCRIPTEN) checks `audio_mode`: if >0 (ScriptNode), calls `DriverAudioCmd()` synchronously; if <0 (AudioWorklet), uses EM_ASM to postMessage to worklet thread. Web also caches first volume and first audio call (lines 1180-1193) in case worklet not yet ready. Desktop: `CallQueue` struct is variable-length (flexible array member `data[1]`), sized as `sizeof(CallQueue)+size-1` bytes.

---

### `OnAudioCall` (audio.cpp:727-738)

**Signature:** `static CallQueue* OnAudioCall()`

**Purpose:** Dequeue all pending audio commands from mutex-protected queue (desktop only).

**Called by:** grep-verified callers:
- `audio.cpp:825` — `coreaudio_cb()` dequeue in callback
- `audio.cpp:992` — `stream_write_cb()` (PulseAudio) dequeue
- `audio.cpp:1043` — `SDLAudioCB()` dequeue

**Calls:** `std::lock_guard<std::mutex>`

**Globals mutated:** `call_head`, `call_tail` (reset to null under lock)

**Globals read:** `call_mutex`

**Side effects:** Acquires mutex, atomically drains queue, releases lock

**Notes:** Atomic batch dequeue: returns head of entire queue chain and clears head/tail pointers under lock. Caller is responsible for traversing chain and freeing nodes. Only used on desktop (CoreAudio, PulseAudio, SDL); web AudioWorklet uses EM_ASM postMessage; web ScriptNode uses synchronous `CallAudio()` → `DriverAudioCmd()`.

---

### `InitAudio` (audio.cpp:781-818, 894-972, 1058-1075, 1211-1338, 1348, 1368-1371)

**Signature:** `bool InitAudio()`

**Purpose:** Platform-specific audio backend initialization; load samples; start playback thread.

**Called by:** grep-verified callers:
- `game_app.cpp:1899` — Main application startup
- `game_web.cpp:651` — Web (commented out for debugging)

**Calls:** `LoadAllSamples()`, platform-specific APIs (AudioQueue, PulseAudio, SDL, Emscripten EM_ASM)

**Globals mutated:** Backend-specific state (`coreaudio_queue`, `mainloop`, `context`, `stream`, `audio_mode`, etc.)

**Globals read:** None directly from audio.cpp globals; uses extern `base_path` for file I/O

**Side effects:** Initializes platform audio API; creates callback thread; opens audio device; returns false on failure

**Notes:** Five implementations selected at compile time via `#ifdef`:

1. **CoreAudio (macOS, line 781-818)**: Creates AudioQueue with 2 pre-allocated buffers, starts playback thread. Calls `coreaudio_cb()` to fill buffers.

2. **PulseAudio (Linux, line 894-972)**: Creates `pa_threaded_mainloop` with locked context/stream initialization. Uses `stream_write_cb()` for data requests. Complex locking protocol to wait for context/stream ready states.

3. **SDL (fallback, line 1058-1075)**: Simple `SDL_OpenAudio()` with `SDLAudioCB()` callback. Portable but higher latency.

4. **Emscripten web (line 1211-1338)**: EM_ASM block loads samples from Emscripten filesystem (`FS.root.contents["samples"]`), calls `Sample()` for each, attempts to create AudioWorklet (if supported) or falls back to ScriptNode. Returns bitwise NOT of sample rate on AudioWorklet success, plain sample rate on ScriptNode, 0 on failure. Sets `audio_mode` (caller responsibility) to determine `CallAudio()` path.

5. **Stub (line 1348, 1368-1371)**: Returns false when no backend configured (NO_AUDIO flag or unknown platform).

---

### `FreeAudio` (audio.cpp:774-779, 873-892, 1036-1039, 1199-1209, 1347, 1364-1366)

**Signature:** `void FreeAudio()`

**Purpose:** Platform-specific cleanup; stop playback thread; free resources.

**Called by:** grep-verified callers:
- `game_app.cpp:541` — Shutdown sequence
- `game_app.cpp:2194` — Error exit
- `game_app.cpp:2248` — Normal exit
- `game_app.cpp:3521` — Final cleanup

**Calls:** Platform-specific cleanup APIs

**Globals mutated:** Backend-specific state (nullified)

**Globals read:** Backend-specific state

**Side effects:** Stops audio thread; closes audio device; deallocates platform resources

**Notes:** Five implementations (one per backend):

1. **CoreAudio**: `AudioQueueStop()`, `AudioQueueDispose()` (frees buffers automatically).

2. **PulseAudio**: `pa_threaded_mainloop_stop()`, `pa_context_disconnect()`, `pa_context_unref()`, `pa_stream_unref()`, `pa_threaded_mainloop_free()`.

3. **SDL**: `SDL_CloseAudio()`.

4. **Emscripten**: EM_ASM block closes `audio_ctx`, nullifies JavaScript objects (`audio_cb`, `audio_node`, etc.).

5. **Stubs**: Empty (line 1347, 1364-1366).

---

### `coreaudio_cb` (audio.cpp:820-838)

**Signature:** `void coreaudio_cb(void* userdata, AudioQueueRef queue, AudioQueueBufferRef buffer)`

**Purpose:** CoreAudio callback; dequeue commands, mix tracks, enqueue buffer for playback.

**Called by:** CoreAudio framework (async callback thread)

**Calls:** `OnAudioCall()`, `DriverAudioCmd()`, `DriverAudioCB()`, `AudioQueueEnqueueBuffer()`

**Globals mutated:** (via `OnAudioCall()` and callbacks)

**Globals read:** `coreaudio_queue`

**Side effects:** I/O through audio hardware; memory allocation/deallocation via command queue drain

**Notes:** Ring buffer pattern: framework fills buffer via callback, app enqueues buffer back. Loop processes all queued commands (is multiple per callback), then generates audio, then returns buffer to framework. Buffer size: `BUFFER_SIZE = 2048` bytes (512 stereo frames at 44.1 kHz).

---

### `stream_write_cb` (audio.cpp:984-1011)

**Signature:** `void stream_write_cb(pa_stream *stream, size_t requested_bytes, void *userdata)`

**Purpose:** PulseAudio callback; fill requested bytes with mixed audio data.

**Called by:** PulseAudio event loop (async, separate thread)

**Calls:** `OnAudioCall()`, `DriverAudioCmd()`, `pa_stream_begin_write()`, `DriverAudioCB()`, `pa_stream_write()`

**Globals mutated:** (via callbacks)

**Globals read:** (via callbacks)

**Side effects:** I/O to PulseAudio stream

**Notes:** is called multiple times until `bytes_remaining <= 0`. Per iteration: gets writable buffer via `pa_stream_begin_write()`, processes queued commands, generates audio, writes to stream. Loop continues while bytes remain (in case fragmented requests).

---

### `SDLAudioCB` (audio.cpp:1041-1056)

**Signature:** `void SDLAudioCB(void* userdata, Uint8* stream, int len)`

**Purpose:** SDL audio callback; dequeue commands, mix tracks, fill output buffer.

**Called by:** SDL audio thread

**Calls:** `OnAudioCall()`, `DriverAudioCmd()`, `DriverAudioCB()`

**Globals mutated:** (via callbacks)

**Globals read:** (via callbacks)

**Side effects:** Modifies output buffer; may allocate/deallocate via command queue

**Notes:** Simple callback: drain queue, generate audio. SDL handles threading and buffer management.

---

### `context_state_cb` (audio.cpp:974-977)

**Signature:** `void context_state_cb(pa_context* context, void* mainloop)`

**Purpose:** PulseAudio context state callback; signal mainloop waiting thread.

**Called by:** PulseAudio event loop

**Calls:** `pa_threaded_mainloop_signal()`

**Globals mutated:** None (event signaling only)

**Globals read:** None directly

**Side effects:** Signals waiting thread that context state changed

**Notes:** Used during `InitAudio()` to wait for PA_CONTEXT_READY state (lines 916-923).

---

### `stream_state_cb` (audio.cpp:979-982)

**Signature:** `void stream_state_cb(pa_stream *s, void *mainloop)`

**Purpose:** PulseAudio stream state callback; signal mainloop waiting thread.

**Called by:** PulseAudio event loop

**Calls:** `pa_threaded_mainloop_signal()`

**Globals mutated:** None (event signaling only)

**Globals read:** None directly

**Side effects:** Signals waiting thread that stream state changed

**Notes:** Used during `InitAudio()` to wait for PA_STREAM_READY state (lines 957-964).

---

### `stream_success_cb` (audio.cpp:1013-1016)

**Signature:** `void stream_success_cb(pa_stream *stream, int success, void *userdata)`

**Purpose:** PulseAudio operation success callback (unused stub).

**Called by:** PulseAudio event loop (after stream cork at line 969)

**Calls:** None

**Globals mutated:** None

**Globals read:** None

**Side effects:** None

**Notes:** Required for `pa_stream_cork()` call but no action needed. Early return (line 1015).

---

### `Sample` (audio.cpp:390-410)

**Signature:** `extern "C" void Sample(const char* name)`

**Purpose:** Web callback to register sample filename in hash table before decoding (AudioWorklet only).

**Called by:** JavaScript loader in `InitAudio()` EM_ASM block (line 1239)

**Calls:** `FindSample()`, `malloc()`, `strcpy()`

**Globals mutated:** `sample_hash[]`, `samples`

**Globals read:** `HASH_MAKS`

**Side effects:** Allocates hash table nodes; increments sample counter

**Notes:** Used only on Emscripten (guarded by `#ifdef EMSCRIPTEN`, line 389-411). Pre-registers samples in hash before `LoadAllSamples()` is called, so `LoadSample()` will find them in hash and skip file I/O. If collision detected (name already registered), silently logs comment (line 406). Returns without action if sample already in hash (line 394-407 check).

---

### `Proc` (audio.cpp:1104-1108)

**Signature:** `extern "C" int16_t* Proc()`

**Purpose:** AudioWorklet processor entry point; generate 128 frames of audio data.

**Called by:** AudioWorklet JavaScript wrapper (on worklet thread)

**Calls:** `DriverAudioCB()`

**Globals mutated:** (via `DriverAudioCB()`)

**Globals read:** (via `DriverAudioCB()`)

**Side effects:** Fills static buffer with PCM data

**Notes:** Web AudioWorklet only (guarded by `#ifdef WORKLET`, line 1088). Generates fixed 128 frames; buffer declared static `proc_buffer[2*128]`. Returns pointer to buffer so JavaScript can read PCM data. No command processing here; `Call()` function (line 1110-1113) handles that separately (called from worklet message handler).

---

### `Init` (audio.cpp:1098-1102)

**Signature:** `extern "C" uint8_t* Init(int num)`

**Purpose:** AudioWorklet initialization callback (unused stub).

**Called by:** AudioWorklet setup (during worklet instantiation)

**Calls:** None

**Globals mutated:** None

**Globals read:** None

**Side effects:** None

**Notes:** Web AudioWorklet only (line 1088). Parameter `num` describes expected frame size but unused. Returns `call_buffer` (4096 bytes) for command queueing. Not actively used by current code.

---

### `Call` (audio.cpp:1110-1113)

**Signature:** `extern "C" void Call(uint8_t* data, int size)`

**Purpose:** AudioWorklet command handler; process audio commands from main thread.

**Called by:** AudioWorklet message handler (on worklet thread)

**Calls:** `DriverAudioCmd()`

**Globals mutated:** (via `DriverAudioCmd()`)

**Globals read:** (via `DriverAudioCmd()`)

**Side effects:** Updates playback state

**Notes:** Web AudioWorklet only (line 1088). Called when main thread posts message via `audio_port.postMessage()` (line 1176 in `CallAudio()`). Bridges from JavaScript message event to C++ audio command processing.

---

### `Audio` (audio.cpp:1125-1148)

**Signature:** `extern "C" const int16_t* Audio(int frames)`

**Purpose:** ScriptNode entry point; allocate/resize buffer, generate PCM data.

**Called by:** ScriptNode JavaScript callback (main thread)

**Calls:** `malloc()`, `free()`, `DriverAudioCB()`

**Globals mutated:** Static buffer pointers `buffer`, `buflen`

**Globals read:** None

**Side effects:** May allocate/deallocate heap buffer on first call or size change

**Notes:** Web ScriptNode only (line 1116). Dynamic buffer sizing: if no buffer or requested frames exceed allocated size, reallocates. Allocates `2*frames` samples (stereo). Handles variable frame sizes from browser. Returns pointer for JavaScript to read PCM data (copied into OutputBuffer by caller). Synchronous: all processing on main thread (higher latency than AudioWorklet).

---


**Signature:** `struct PlyTrack`

**Purpose:** 16-track mixer state for simultaneous playback.

**Called by:** See usage in `DriverAudioCB()` and `DriverAudioCmd()`

**Calls:** None (data structure only)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None (data structure only)

**Notes:** Fields: `sample_id` (Sample index, -1 if inactive), `sample_vol` (Track volume 0-65535), `sample_pos` (Current playback position in frames), `sample_end` (Stop position: marker end or sample end).

---

### `struct SampleHash` (audio.cpp:351-357)

**Signature:** `struct SampleHash`

**Purpose:** Hash table node for filename→sample_id lookup.

**Called by:** See usage in `FindSample()` and `LoadSample()`

**Calls:** None (data structure only)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None (data structure only)

**Notes:** Fields: `next` (Collision chain link), `hash` (djb2 hash of filename), `id` (Sample id index into lib_sample_data[]), `name[1]` (Flexible array member; actual size allocated per-node).

---

### `struct CallQueue` (audio.cpp:718-722)

**Signature:** `struct CallQueue`

**Purpose:** Linked-list command queue node (desktop only).

**Called by:** See usage in `CallAudio()` and `OnAudioCall()`

**Calls:** None (data structure only)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None (data structure only)

**Notes:** Fields: `next` (Queue chain link), `size` (Command payload size in bytes), `data[1]` (Flexible array; actual size allocated per-node).

**Called by:** AudioWorklet message handler (on worklet thread)

**Calls:** `DriverAudioCmd()`

**Globals mutated:** (via `DriverAudioCmd()`)

**Globals read:** (via `DriverAudioCmd()`)

**Side effects:** Updates playback state

**Notes:** Web AudioWorklet only (line 1088). Called when main thread posts message via `audio_port.postMessage()` (line 1176 in `CallAudio()`). Bridges from JavaScript message event to C++ audio command processing.

---

### `Audio` (audio.cpp:1125-1148)

**Signature:** `extern "C" const int16_t* Audio(int frames)`

**Purpose:** ScriptNode entry point; allocate/resize buffer, generate PCM data.

**Called by:** ScriptNode JavaScript callback (main thread)

**Calls:** `malloc()`, `free()`, `DriverAudioCB()`

**Globals mutated:** Static buffer pointers `buffer`, `buflen`

**Globals read:** None

**Side effects:** May allocate/deallocate heap buffer on first call or size change

**Notes:** Web ScriptNode only (line 1116). Dynamic buffer sizing: if no buffer or requested frames exceed allocated size, reallocates. Allocates `2*frames` samples (stereo). Handles variable frame sizes from browser. Returns pointer for JavaScript to read PCM data (copied into OutputBuffer by caller). Synchronous: all processing on main thread (higher latency than AudioWorklet).

---

## `GlobalSDL` (sdl.cpp:106-127)

| Field | Type | Purpose |

---

## Command Protocol

## Size-Based Dispatch (DriverAudioCmd)

| Size | Format | Purpose |
|------|--------|---------|
| 2 | `uint16_t volume` | Set global volume (0=mute, 32768=full) |
| 4 | `int32_t forest_id` | Set background forest loop sample id |
| 12+ | `[int32_t track, int32_t sample, int32_t vol, optional int32_t chunk]` | Configure track; chunk enables marker-based playback |

## Marker Layout (XOgg output)

Stored after PCM data in `lib_sample_data[]`:

```
[int32_t marker_count, int32_t marker_0_start, int32_t marker_0_end, ...]
```

Accessed via: `int32_t* marker_ptr = (int32_t*)(lib_sample_data[sample_id] + 2*sample_end) + 1;`

Chunk index `msg[3]` selects marker pair: `marker_ptr[2*chunk_idx + 0/1]` (start/end samples).

---

## Platform Coverage Matrix

| Backend | Platform | Threading | API | Latency | Status |
|---------|----------|-----------|-----|---------|--------|
| CoreAudio | macOS, iOS | Separate callback thread | AudioQueue | Low | Implemented |
| PulseAudio | Linux desktop | Threaded mainloop | pa_stream + lock/signal | Low | Implemented |
| SDL | Cross-platform fallback | Separate callback thread | SDL2 audio | Medium | Implemented |
| AudioWorklet | Modern web (Chrome 66+, Firefox 76+) | Dedicated worklet thread | postMessage | Low | Implemented |
| ScriptNode | Legacy web (all browsers) | Main thread (sync) | Deprecated API | High | Implemented (fallback) |

---

## Caveats & TODOs

- **Line 193, `AudioWalk()`:** Commented-out material-to-sample mapping (`GetSampleID((AUDIO_FILE)(2*material+(foot&1)))`) currently unused; hardcoded to FOOTSTEPS sample. Per-material sample bank not implemented.

- **Line 553-563, `DriverAudioCmd()`:** Marker lookup does not bounds-check marker count or chunk index; out-of-range chunk may read beyond allocated marker array.

- **Line 684, `DriverAudioCB()`:** Division by 65535 instead of proper gain normalization (legacy code, potential precision loss).

- **Line 568-637, `DriverAudioCmd()`:** Extensive comment block documents future commands (pan, rotation, looping, pause, events, scripting) not yet implemented. Current dispatch only handles 2, 4, 12+ byte sizes.

- **No sample unloading:** Once loaded, samples remain in memory indefinitely (no free per-sample). Only freed at engine shutdown.

- **Forest loop always plays:** If `ply_forest_id >= 0`, background forest ambience continuously loops in `DriverAudioCB()` regardless of game state.

