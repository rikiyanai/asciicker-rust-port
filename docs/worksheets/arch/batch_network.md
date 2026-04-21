# Network System Batch Analysis

Cross-platform networking and threading primitives for multiplayer game server.
Covers: network.h (declarations), network.cpp (implementation), game_svr.cpp (server entry point).

---

## network.h - Protocol Definitions and API Declarations

### Protocol Structs (network.h:126-209)

**Purpose:** Binary wire format for client-server multiplayer protocol  
**Packing:** `#pragma pack(push,1)` ensures no padding (network-safe)  
**Token Convention:** Uppercase = client request, lowercase = server response  
**Endianness:** Little-endian assumed (x86/ARM on both sides, no conversion)

**Structs:**
- `STRUCT_REQ_JOIN` ('J'): Client join with 31-char name
- `STRUCT_RSP_JOIN` ('j'): Server assigns ID + maxcli count
- `STRUCT_BRC_JOIN` ('j'): Broadcast new player join (position, sprite, name)
- `STRUCT_BRC_EXIT` ('e'): Broadcast player exit (id only)
- `STRUCT_REQ_POSE` ('P'): Client pose update (anim, frame, pos, dir, sprite)
- `STRUCT_BRC_POSE` ('p'): Broadcast pose to all clients (includes sender id)
- `STRUCT_REQ_TALK` ('T'): Client chat (256-byte variable-length string)
- `STRUCT_BRC_TALK` ('t'): Broadcast chat with sender id
- `STRUCT_REQ_LAG` ('L'): Client ping (3-byte timestamp)
- `STRUCT_RSP_LAG` ('l'): Server pong (echoes timestamp)

**Token Collision Note:** STRUCT_RSP_JOIN and STRUCT_BRC_JOIN both use 'j', but RSP is sent synchronously before any broadcast.

---

## network.cpp - Platform Abstraction Implementation

### `TCP_INIT` (network.cpp:138-142 Windows, 337-340 POSIX)

**Signature:**
```cpp
int TCP_INIT()
```
**Purpose:** Initialize socket subsystem  
**Called by:** `ServerLoop` (game_svr.cpp:931)  
**Calls:** Windows: `WSAStartup(MAKEWORD(2,2), &wsaData)`, POSIX: no-op (returns 0)  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Windows: Initializes Winsock DLL  
**Notes:** Must be called before any socket operations on Windows. POSIX version is no-op stub.

---

### `TCP_CLOSE` (network.cpp:144-147 Windows, 342-345 POSIX)

**Signature:**
```cpp
int TCP_CLOSE(TCP_SOCKET s)
```
**Purpose:** Close socket and release OS handle  
**Called by:** `ServerLoop` (game_svr.cpp:1028,1050), `PlayerCon::Stop` (game_svr.cpp:258), `PlayerCon::Release` (game_svr.cpp:804), `PlayerCon::Recv` (game_svr.cpp:751), `BroadCast::Send` (game_svr.cpp:891)  
**Calls:** Windows: `closesocket(s)`, POSIX: `close(s)`  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Closes file descriptor/socket handle, terminates network connection  
**Notes:** Platform-specific close function names (closesocket vs close).

---

### `TCP_CLEANUP` (network.cpp:149-152 Windows, 347-350 POSIX)

**Signature:**
```cpp
int TCP_CLEANUP()
```
**Purpose:** Cleanup socket subsystem  
**Called by:** `ServerLoop` (game_svr.cpp:953,963,982,1061)  
**Calls:** Windows: `WSACleanup()`, POSIX: no-op (returns 0)  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Windows: Unloads Winsock DLL  
**Notes:** Symmetric with TCP_INIT. Must be called after all sockets closed on Windows.

---

### `TCP_WRITE` (network.cpp:488-500)

**Signature:**
```cpp
int TCP_WRITE(TCP_SOCKET s, const uint8_t* buf, int size)
```
**Purpose:** Blocking send with retry until all bytes written  
**Called by:** `WS_WRITE` (network.cpp:791,797), `PlayerCon::Recv` (game_svr.cpp:455)  
**Calls:** `send(s, (const char*)buf, l, 0)` in loop  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Sends data over network, blocks until complete  
**Notes:** Loops until all bytes sent, returns on error (w<=0) or completion. Common implementation for both Windows/POSIX.

---

### `TCP_READ` (network.cpp:502-514)

**Signature:**
```cpp
int TCP_READ(TCP_SOCKET s, uint8_t* buf, int size)
```
**Purpose:** Blocking recv with retry until all bytes read  
**Called by:** `HTTP_READ` (network.cpp:550), `WS_READ` (network.cpp:835,853,865,878,891,902,934,951,962)  
**Calls:** `recv(s, (char*)buf, l, 0)` in loop  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Reads data from network, blocks until complete  
**Notes:** Loops until all bytes received, returns on error (r<=0) or completion. Common implementation for both Windows/POSIX.

---

### `HTTP_READ` (network.cpp:532-691)

**Signature:**
```cpp
int HTTP_READ(TCP_SOCKET s, int(*cb)(const char* header, const char* value, void* param), void* param, char body_overread[2048])
```
**Purpose:** Parse HTTP headers via callback, return body overshoot size  
**Called by:** `PlayerCon::Recv` (game_svr.cpp:429) via Headers::cb  
**Calls:** `recv(s, (char*)buf, 2048, 0)`, callback for each header:value pair, `malloc/realloc/free` for dynamic header storage  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Reads from network, invokes callback, allocates/frees heap memory  
**Notes:** Callback-based to avoid large header dictionary allocation. Returns bytes read beyond headers (body overshoot) in 2KB buffer. Handles dynamic header/value growth up to 64KB limit.

**WHY callback:** Avoids allocating entire header dictionary, allows early rejection by returning negative from callback.

**WHY 2KB body_overread:** Last recv() during header parse  captures partial body (first few bytes), buffer captures this overshoot.

---

### `WS_WRITE` (network.cpp:706-805)

**Signature:**
```cpp
int WS_WRITE(TCP_SOCKET s, const uint8_t* buf, int size, int split, int type)
```
**Purpose:** WebSocket frame encoder (RFC 6455)  
**Called by:** `PlayerCon::Recv` (game_svr.cpp:511,531,645,687), `WS_READ` (network.cpp:926,943) for control frames  
**Calls:** `TCP_WRITE(s, frame, len)`, `TCP_WRITE(s, buf+offs, payload)`  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Sends WebSocket-framed data over network  
**Notes:** Supports text/binary/close/ping/pong frame types (type param: 0x1=text, 0x2=bin, 0x8=close, 0x9=ping, 0xA=pong). Handles payload length encoding (7-bit, 16-bit, 64-bit). Splits large messages into multiple frames if split>0. FIN bit set only on last frame. Server sends UNMASKED frames (client-to-server must be masked per RFC 6455).

**WHY masking required:** RFC 6455 mandates client→server frames masked (security: cache poisoning prevention), server→client unmasked.

**WHY split parameter:** Prevents large frames causing latency spikes by splitting into equal-sized chunks.

---

### `WS_READ` (network.cpp:823-979)

**Signature:**
```cpp
int WS_READ(TCP_SOCKET s, uint8_t* buf, int size, int* type)
```
**Purpose:** WebSocket frame decoder (unmask, handle control frames)  
**Called by:** `PlayerCon::Recv` (game_svr.cpp:475)  
**Calls:** `TCP_READ(s, frame, N)` for frame header and payload, `WS_WRITE(s, ping, payload, 0, 0xA)` for pong response, `WS_WRITE(s, 0, 0, 0, 0x8)` for close ack  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Reads from network, auto-responds to control frames (close/ping), returns data frames to caller  
**Notes:** Unmasks client-to-server frames (XOR with 4-byte mask key). Handles multi-frame messages (FIN bit). Auto-responds to ping (send pong), auto-acks close (send close), ignores pong. Returns -1 on error, close, or buffer too small. Returns tot_data bytes for data frames (0x0=continuation, 0x1=text, 0x2=binary).

**WHY unmasking:** RFC 6455 requires client-to-server frames masked, server unmasks via XOR: `payload[i] ^= mask[i & 3]`.

**WHY control frame dispatch:** Close/ping/pong must be handled immediately (protocol requirement), data frames returned to caller.

---

### `THREAD_CREATE` (network.cpp:192-208 Windows, 361-371 POSIX)

**Signature:**
```cpp
THREAD_HANDLE* THREAD_CREATE(void* (*entry)(void*), void* arg)
```
**Purpose:** Spawn joinable thread with portable signature  
**Called by:** No callers found via grep  
**Calls:** Windows: `malloc`, `CreateThread(0, 0, THREAD_HANDLE::wrap, t, 0, &id)`, `free` on failure, POSIX: `pthread_create(&th, 0, entry, arg)`, `malloc`  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Allocates wrapper struct, spawns OS thread  
**Notes:** Windows version uses trampoline (THREAD_HANDLE::wrap) to convert DWORD WINAPI signature to void*(*)(void*) portable signature. POSIX version directly uses pthread with portable signature. Returns THREAD_HANDLE* for cross-platform join, NULL on failure.

**WHY wrapper struct:** Windows CreateThread expects `DWORD WINAPI (*)(LPVOID)` signature, game code uses POSIX-style `void* (*)(void*)`. Wrapper stores entry/arg and provides trampoline.

---

### `THREAD_JOIN` (network.cpp:215-222 Windows, 377-383 POSIX)

**Signature:**
```cpp
void* THREAD_JOIN(THREAD_HANDLE* thread)
```
**Purpose:** Wait for thread exit, retrieve return value  
**Called by:** Not found in grep results  
**Calls:** Windows: `WaitForSingleObject(thread->th, INFINITE)`, `CloseHandle(thread->th)`, `free(thread)`, POSIX: `pthread_join(thread->th, &ret)`, `free(thread)`  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Blocks until thread exits, frees wrapper struct  
**Notes:** Windows version retrieves return value from wrapper->arg (populated by trampoline). POSIX retrieves via pthread_join out-param. Both paths free wrapper and return void*.

---

### `THREAD_CREATE_DETACHED` (network.cpp:225-239 Windows, 385-393 POSIX)

**Signature:**
```cpp
bool THREAD_CREATE_DETACHED(void* (*entry)(void*), void* arg)
```
**Purpose:** Fire-and-forget thread (no join)  
**Called by:** `PlayerCon::Start` (game_svr.cpp:251)  
**Calls:** Windows: `malloc`, `CreateThread(0, 0, THREAD_HANDLE::wrap_detached, t, 0, &id)`, `CloseHandle(th)`, `free` on failure, POSIX: `pthread_create(&th, 0, entry, arg)`, `pthread_detach(th)`  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Spawns detached OS thread, wrapper auto-freed (Windows: in wrap_detached, POSIX: no wrapper)  
**Notes:** Windows wrap_detached copies entry/arg to stack, frees wrapper immediately, then invokes entry (wrapper leaks prevented). POSIX directly detaches thread. Returns bool (success/failure), caller cannot join.

**WHY wrap_detached frees wrapper:** Detached threads have no join, so wrapper would leak if not freed immediately.

---

### `THREAD_SLEEP` (network.cpp:241-244 Windows, 395-398 POSIX)

**Signature:**
```cpp
void THREAD_SLEEP(int ms)
```
**Purpose:** Cross-platform sleep  
**Called by:** Not found in grep results  
**Calls:** Windows: `Sleep(ms)`, POSIX: `usleep(ms*1000)`  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Yields CPU, blocks calling thread for ms milliseconds  
**Notes:** Windows Sleep takes milliseconds, POSIX usleep takes microseconds (multiply by 1000).

---

### `RWLOCK_CREATE` (network.cpp:251-256 Windows, 405-410 POSIX)

**Signature:**
```cpp
RWLOCK_HANDLE* RWLOCK_CREATE()
```
**Purpose:** Allocate and initialize read-write lock  
**Called by:** `PlayerCon::Start` (game_svr.cpp:250), `ServerLoop` (game_svr.cpp:1002)  
**Calls:** Windows: `malloc`, `InitializeSRWLock(&rwl->rw)`, POSIX: `malloc`, `pthread_rwlock_init(&rwl->rw, 0)`  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Allocates heap memory, initializes OS lock primitive  
**Notes:** Returns RWLOCK_HANDLE* wrapper struct. Windows uses SRWLOCK (Slim Read-Write Lock), POSIX uses pthread_rwlock_t.

---

### `RWLOCK_DELETE` (network.cpp:258-261 Windows, 412-416 POSIX)

**Signature:**
```cpp
void RWLOCK_DELETE(RWLOCK_HANDLE* rwl)
```
**Purpose:** Destroy read-write lock and free memory  
**Called by:** `PlayerCon::Release` (game_svr.cpp:843), `ServerLoop` (game_svr.cpp:1059)  
**Calls:** Windows: `free(rwl)`, POSIX: `pthread_rwlock_destroy(&rwl->rw)`, `free(rwl)`  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Frees heap memory, POSIX destroys lock primitive  
**Notes:** Windows SRWLOCK has no destroy function (no-op). POSIX requires pthread_rwlock_destroy.

---

### `RWLOCK_READ_LOCK` (network.cpp:263-266 Windows, 418-421 POSIX)

**Signature:**
```cpp
void RWLOCK_READ_LOCK(RWLOCK_HANDLE* rwl)
```
**Purpose:** Acquire read lock (shared, multiple readers allowed)  
**Called by:** `PlayerCon::Recv` (game_svr.cpp:624,672), `BroadCast::Send` (game_svr.cpp:860)  
**Calls:** Windows: `AcquireSRWLockShared(&rwl->rw)`, POSIX: `pthread_rwlock_rdlock(&rwl->rw)`  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Blocks until read lock acquired, multiple threads can hold read lock simultaneously  
**Notes:** Use for read-only access to shared data. Must be paired with RWLOCK_READ_UNLOCK.

---

### `RWLOCK_READ_UNLOCK` (network.cpp:268-271 Windows, 423-426 POSIX)

**Signature:**
```cpp
void RWLOCK_READ_UNLOCK(RWLOCK_HANDLE* rwl)
```
**Purpose:** Release read lock  
**Called by:** `PlayerCon::Recv` (game_svr.cpp:648,682,690,696), `BroadCast::Send` (game_svr.cpp:915)  
**Calls:** Windows: `ReleaseSRWLockShared(&rwl->rw)`, POSIX: `pthread_rwlock_unlock(&rwl->rw)`  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Releases shared lock, may wake waiting writers  
**Notes:** Must match prior RWLOCK_READ_LOCK.

---

### `RWLOCK_WRITE_LOCK` (network.cpp:273-276 Windows, 428-431 POSIX)

**Signature:**
```cpp
void RWLOCK_WRITE_LOCK(RWLOCK_HANDLE* rwl)
```
**Purpose:** Acquire write lock (exclusive, no other readers/writers)  
**Called by:** `PlayerCon::Recv` (game_svr.cpp:523,626), `PlayerCon::Aquire` (game_svr.cpp:785), `PlayerCon::Release` (game_svr.cpp:800), `BroadCast::Send` (game_svr.cpp:878)  
**Calls:** Windows: `AcquireSRWLockExclusive(&rwl->rw)`, POSIX: `pthread_rwlock_wrlock(&rwl->rw)`  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Blocks until exclusive write lock acquired  
**Notes:** Use for write access to shared data. Blocks readers and other writers. Must be paired with RWLOCK_WRITE_UNLOCK.

---

### `RWLOCK_WRITE_UNLOCK` (network.cpp:278-281 Windows, 433-436 POSIX)

**Signature:**
```cpp
void RWLOCK_WRITE_UNLOCK(RWLOCK_HANDLE* rwl)
```
**Purpose:** Release write lock  
**Called by:** `PlayerCon::Recv` (game_svr.cpp:534,558,584,605,638), `PlayerCon::Aquire` (game_svr.cpp:793), `PlayerCon::Release` (game_svr.cpp:846), `BroadCast::Send` (game_svr.cpp:882,905)  
**Calls:** Windows: `ReleaseSRWLockExclusive(&rwl->rw)`, POSIX: `pthread_rwlock_unlock(&rwl->rw)`  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Releases exclusive lock, may wake waiting readers/writers  
**Notes:** Must match prior RWLOCK_WRITE_LOCK.

---

### `MUTEX_CREATE` (network.cpp:309-314 Windows, 443-448 POSIX)

**Signature:**
```cpp
MUTEX_HANDLE* MUTEX_CREATE()
```
**Purpose:** Allocate and initialize mutex (exclusive lock)  
**Called by:** Not found in grep results  
**Calls:** Windows: `malloc`, `InitializeCriticalSection(&m->mu)`, POSIX: `malloc`, `pthread_mutex_init(&m->mu, 0)`  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Allocates heap memory, initializes OS mutex primitive  
**Notes:** Returns MUTEX_HANDLE* wrapper. Windows uses CRITICAL_SECTION, POSIX uses pthread_mutex_t.

---

### `MUTEX_DELETE` (network.cpp:316-320 Windows, 450-454 POSIX)

**Signature:**
```cpp
void MUTEX_DELETE(MUTEX_HANDLE* mutex)
```
**Purpose:** Destroy mutex and free memory  
**Called by:** Not found in grep results  
**Calls:** Windows: `DeleteCriticalSection(&mutex->mu)`, `free(mutex)`, POSIX: `pthread_mutex_destroy(&mutex->mu)`, `free(mutex)`  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Destroys OS mutex primitive, frees heap memory  
**Notes:** Both Windows and POSIX require explicit destroy call.

---

### `MUTEX_LOCK` (network.cpp:322-325 Windows, 456-459 POSIX)

**Signature:**
```cpp
void MUTEX_LOCK(MUTEX_HANDLE* mutex)
```
**Purpose:** Acquire exclusive mutex lock  
**Called by:** Not found in grep results  
**Calls:** Windows: `EnterCriticalSection(&mutex->mu)`, POSIX: `pthread_mutex_lock(&mutex->mu)`  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Blocks until mutex acquired  
**Notes:** Must be paired with MUTEX_UNLOCK. Deadlock if same thread locks twice (non-recursive).

---

### `MUTEX_UNLOCK` (network.cpp:327-330 Windows, 461-464 POSIX)

**Signature:**
```cpp
void MUTEX_UNLOCK(MUTEX_HANDLE* mutex)
```
**Purpose:** Release exclusive mutex lock  
**Called by:** Not found in grep results  
**Calls:** Windows: `LeaveCriticalSection(&mutex->mu)`, POSIX: `pthread_mutex_unlock(&mutex->mu)`  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Releases mutex, may wake waiting threads  
**Notes:** Must match prior MUTEX_LOCK.

---

### `INTERLOCKED_DEC` (network.cpp:283-286 Windows, 466-469 POSIX)

**Signature:**
```cpp
unsigned int INTERLOCKED_DEC(volatile unsigned int* ptr)
```
**Purpose:** Atomic decrement (lock-free)  
**Called by:** `PlayerCon::Recv` (game_svr.cpp:540), `PlayerCon::Release` (game_svr.cpp:836), `BroadCast::Send` (game_svr.cpp:909)  
**Calls:** Windows: `InterlockedDecrement(ptr)`, POSIX: `__sync_fetch_and_sub(ptr, 1) - 1`  
**Globals read:** None  
**Globals mutated:** `*ptr` (atomic)  
**Side effects:** Atomically decrements value, returns new value  
**Notes:** POSIX adjusts __sync_fetch_and_sub return (old value) to match Windows (new value). Used for reference counting (free when reaches 0).

---

### `INTERLOCKED_INC` (network.cpp:288-291 Windows, 471-474 POSIX)

**Signature:**
```cpp
unsigned int INTERLOCKED_INC(volatile unsigned int* ptr)
```
**Purpose:** Atomic increment (lock-free)  
**Called by:** `BroadCast::Send` (game_svr.cpp:903)  
**Calls:** Windows: `InterlockedIncrement(ptr)`, POSIX: `__sync_fetch_and_add(ptr, 1) + 1`  
**Globals read:** None  
**Globals mutated:** `*ptr` (atomic)  
**Side effects:** Atomically increments value, returns new value  
**Notes:** POSIX adjusts __sync_fetch_and_add return (old value) to match Windows (new value). Used for reference counting.

---

### `INTERLOCKED_SUB` (network.cpp:293-296 Windows, 476-479 POSIX)

**Signature:**
```cpp
unsigned int INTERLOCKED_SUB(volatile unsigned int* ptr, unsigned int sub)
```
**Purpose:** Atomic subtraction (lock-free)  
**Called by:** Not found in grep results  
**Calls:** Windows: `(unsigned int)InterlockedAdd((volatile LONG*)ptr, -(LONG)sub)`, POSIX: `__sync_fetch_and_sub(ptr, sub) - sub`  
**Globals read:** None  
**Globals mutated:** `*ptr` (atomic)  
**Side effects:** Atomically subtracts value, returns new value  
**Notes:** Windows uses InterlockedAdd with negative operand (cast to LONG). POSIX adjusts return value.

---

### `INTERLOCKED_ADD` (network.cpp:298-301 Windows, 481-484 POSIX)

**Signature:**
```cpp
unsigned int INTERLOCKED_ADD(volatile unsigned int* ptr, unsigned int add)
```
**Purpose:** Atomic addition (lock-free)  
**Called by:** Not found in grep results  
**Calls:** Windows: `(unsigned int)InterlockedAdd((volatile LONG*)ptr, (LONG)add)`, POSIX: `__sync_fetch_and_add(ptr, add) + add`  
**Globals read:** None  
**Globals mutated:** `*ptr` (atomic)  
**Side effects:** Atomically adds value, returns new value  
**Notes:** Windows uses InterlockedAdd (cast to LONG). POSIX adjusts __sync_fetch_and_add return (old value) to match Windows (new value).

---

### `THREAD_HANDLE::wrap` (network.cpp:161-166 Windows only)

**Signature:**
```cpp
static DWORD WINAPI wrap(LPVOID p)
```
**Purpose:** Trampoline to convert Windows thread signature to portable signature  
**Called by:** Windows CreateThread via THREAD_CREATE  
**Calls:** `t->entry(t->arg)` (portable entry function)  
**Globals read:** None  
**Globals mutated:** `t->arg` (stores return value)  
**Side effects:** Invokes user entry function, stores return value in wrapper  
**Notes:** Converts `DWORD WINAPI (*)(LPVOID)` Windows signature to `void* (*)(void*)` portable signature. Stores return value in `t->arg` for THREAD_JOIN retrieval.

---

### `THREAD_HANDLE::wrap_detached` (network.cpp:168-176 Windows only)

**Signature:**
```cpp
static DWORD WINAPI wrap_detached(LPVOID p)
```
**Purpose:** Trampoline for detached threads, frees wrapper immediately  
**Called by:** Windows CreateThread via THREAD_CREATE_DETACHED  
**Calls:** `free(t)`, `entry(arg)` (portable entry function)  
**Globals read:** None  
**Globals mutated:** None (frees wrapper)  
**Side effects:** Copies entry/arg to stack, frees wrapper, invokes user entry  
**Notes:** Prevents wrapper leak for detached threads (no join → no cleanup opportunity). Copies entry/arg before free, then invokes entry.

---

## game_svr.cpp - Server Entry Point and Client Connection Handling

### `akAPI_Exec` (game_svr.cpp:118-120)

**Signature:**
```cpp
void akAPI_Exec(const char* str, int len, bool root)
```
**Purpose:** JavaScript execution stub (no V8 on server)  
**Called by:** `game.cpp` via extern (5 call sites for chat/scripting)  
**Calls:** None (stub)  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** None (no-op)  
**Notes:** Headless server has no JavaScript engine. Game logic runs C++ only. NPC scripting not available on server.

---

### `exit_handler` (game_svr.cpp:124-129)

**Signature:**
```cpp
void exit_handler(int sig)
```
**Purpose:** Signal handler for graceful shutdown (SIGINT/SIGTERM)  
**Called by:** OS signal delivery (registered at game_svr.cpp:1102-1103)  
**Calls:** `printf`  
**Globals read:** None  
**Globals mutated:** `isRunning` (set to false)  
**Side effects:** Sets global shutdown flag, prints message  
**Notes:** SIGINT (Ctrl+C) triggers graceful shutdown. Sets isRunning=false to exit ServerLoop accept loop.

---

### `Buzz` (game_svr.cpp:133-135)

**Signature:**
```cpp
void Buzz()
```
**Purpose:** Haptic feedback stub (no haptics on server)  
**Called by:** `game.cpp` (2 call sites for UI feedback)  
**Calls:** None (stub)  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** None (no-op)  
**Notes:** Headless server has no haptic output devices.

---

### `SyncConf` (game_svr.cpp:138-140)

**Signature:**
```cpp
void SyncConf()
```
**Purpose:** Configuration sync stub (no IndexedDB sync needed)  
**Called by:** `game.cpp:SaveConf()` (2 call sites)  
**Calls:** None (stub)  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** None (no-op)  
**Notes:** Server writes config synchronously to native filesystem (no async IndexedDB like web build).

---

### `GetConfPath` (game_svr.cpp:145-148)

**Signature:**
```cpp
const char* GetConfPath()
```
**Purpose:** Return server config file path  
**Called by:** `game.cpp:LoadConf(), SaveConf()` (3 call sites)  
**Calls:** None  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** None  
**Notes:** Returns "asciicker.cfg" in current directory (no user home, no virtual filesystem).

---

### `Server::Send` (game_svr.cpp:153-156)

**Signature:**
```cpp
bool Server::Send(const uint8_t* data, int size)
```
**Purpose:** Send network packet to client (STUB)  
**Called by:** No callers found via grep  
**Calls:** None (stub)  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** None (returns false, send failed)  
**Notes:** Server networking not yet implemented. Future: Send via BSD sockets API.

---

### `Server::Proc` (game_svr.cpp:159-162)

**Signature:**
```cpp
void Server::Proc()
```
**Purpose:** Process server tick (STUB)  
**Called by:** No callers found via grep  
**Calls:** None (stub)  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** None (no-op)  
**Notes:** Server loop not yet implemented. Future: Process network messages, update physics.

---

### `Server::Log` (game_svr.cpp:165-172)

**Signature:**
```cpp
void Server::Log(const char* str)
```
**Purpose:** Log server message with timestamp prefix  
**Called by:** No callers found via grep  
**Calls:** `time(NULL)`, `localtime(&now)`, `strftime`, `printf`  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Prints timestamped message to stdout  
**Notes:** Format: `[YYYY-MM-DD HH:MM:SS] message`. Uses localtime (not thread-safe on some platforms).

---

### `Base64Encode` (game_svr.cpp:176-225)

**Signature:**
```cpp
int Base64Encode(unsigned char* data, int len, char* base64)
```
**Purpose:** Encode binary data to Base64 string  
**Called by:** `PlayerCon::Recv` (game_svr.cpp:442) for WebSocket handshake  
**Calls:** None (pure computation)  
**Globals read:** None (uses static const char chr[] table)  
**Globals mutated:** None  
**Side effects:** Writes to output buffer `base64`  
**Notes:** RFC 4648 Base64 encoding. Pads with '=' for incomplete 3-byte chunks. Returns encoded length (not null-terminated by this function, caller adds '\0').

---

### `BroadCast::Send` (game_svr.cpp:854-916)

**Signature:**
```cpp
void BroadCast::Send(int id_from, bool cs_already_locked = false)
```
**Purpose:** Broadcast message to all connected clients (except sender)  
**Called by:** `PlayerCon::Recv` (game_svr.cpp:602,720,742,818) for pose/join/talk/exit broadcasts  
**Calls:** `RWLOCK_READ_LOCK(PlayerCon::cs)`, `RWLOCK_READ_UNLOCK(PlayerCon::cs)`, `RWLOCK_WRITE_LOCK(con->rwlock)`, `RWLOCK_WRITE_UNLOCK(con->rwlock)`, `INTERLOCKED_INC(&refs)`, `INTERLOCKED_DEC(&refs)`, `TCP_CLOSE(con->client_socket)`, `free(this)`  
**Globals read:** `PlayerCon::clients`, `PlayerCon::client_id[]`, `PlayerCon::players[]`  
**Globals mutated:** `con->broadcasts`, `con->head`, `con->tail`, `refs` (atomic), `con->client_socket` (set to INVALID on overload)  
**Side effects:** Enqueues broadcast to each client's pending queue, auto-frees when all refs released, closes socket if client can't keep up (>5000 broadcasts pending)  
**Notes:** Reference-counted broadcast (refs initialized to 1, incremented per recipient, decremented when sent or client released). Skips non-joined clients. Drops pose broadcasts if client has >500 pending (overload protection). Force-closes socket if >5000 pending (client can't keep up).

**WHY reference counting:** Single broadcast message shared across multiple client queues. Auto-freed when last reference released (INTERLOCKED_DEC==0).

**WHY broadcast fuse:** Prevents memory exhaustion from slow/dead clients. Pose broadcasts skipped at 500, socket closed at 5000.

---

### `PlayerCon::Start` (game_svr.cpp:247-252)

**Signature:**
```cpp
bool Start(TCP_SOCKET socket)
```
**Purpose:** Initialize player connection and spawn receive thread  
**Called by:** `ServerLoop` (game_svr.cpp:1030)  
**Calls:** `RWLOCK_CREATE()`, `THREAD_CREATE_DETACHED(Recv, this)`  
**Globals read:** None  
**Globals mutated:** `client_socket`, `rwlock`  
**Side effects:** Spawns detached thread for connection handling  
**Notes:** Detached thread calls PlayerCon::Recv. Returns false if thread spawn fails.

---

### `PlayerCon::Stop` (game_svr.cpp:254-259)

**Signature:**
```cpp
void Stop()
```
**Purpose:** Close client socket (atomic)  
**Called by:** `ServerLoop` (game_svr.cpp:1056) during shutdown  
**Calls:** `TCP_CLOSE(s)`  
**Globals read:** None  
**Globals mutated:** `client_socket` (set to INVALID_TCP_SOCKET)  
**Side effects:** Closes socket, terminates network connection  
**Notes:** Atomic socket swap (read-modify-write). Sets INVALID_TCP_SOCKET before close to prevent double-close.

---

### `PlayerCon::Recv` (game_svr.cpp:285-760)

**Signature:**
```cpp
void Recv()
```
**Purpose:** WebSocket handshake + message receive loop (runs in detached thread)  
**Called by:** Detached thread spawned by `PlayerCon::Start`  
**Calls:** `HTTP_READ` (via Headers::cb), `SHA1`, `Base64Encode`, `TCP_WRITE`, `setsockopt`, `WS_READ`, `WS_WRITE`, `RWLOCK_WRITE_LOCK/UNLOCK`, `RWLOCK_READ_LOCK/UNLOCK`, `INTERLOCKED_DEC`, `TCP_CLOSE`, `Release`, `malloc`, `strcpy`, `strcmp`, `strncmp`, `memcpy`, `printf`  
**Globals read:** `PlayerCon::cs`, `PlayerCon::clients`, `PlayerCon::client_id[]`, `PlayerCon::players[]`, `max_players`  
**Globals mutated:** `player_state`, `player_name`, `joined`, `has_state`, `head`, `tail`, `broadcasts`, `client_socket`  
**Side effects:** Network I/O (read/write), allocates broadcasts, updates player state, broadcasts to other clients, closes socket on error  
**Notes:** 470+ line function handling full client lifecycle:
1. WebSocket handshake (HTTP upgrade, validate headers, SHA1+Base64 accept key)
2. Message loop: 'L' lag, 'P' pose, 'J' join, 'T' talk
3. Auto-release on error (socket close, protocol violation)

**WebSocket handshake:**
- Validates HTTP GET `/ws/y8/`, Sec-WebSocket-Version=13, Upgrade=WebSocket, Connection=Upgrade
- Computes Sec-WebSocket-Accept: SHA1(key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11") → Base64
- Sends 101 Switching Protocols response
- Sets 300s idle timeout post-handshake

**Message dispatch:**
- 'L' (lag): Echo timestamp back (ping/pong)
- 'P' (pose): Process broadcasts, update player_state if changed, broadcast to others
- 'J' (join): Assign ID, send all existing players, broadcast join to others
- 'T' (talk): Broadcast chat message

**Error handling:** Any protocol violation → `Release()` → socket close, broadcast exit, free resources

---

### `PlayerCon::Recv` (static trampoline) (game_svr.cpp:762-767)

**Signature:**
```cpp
static void* Recv(void* p)
```
**Purpose:** Thread entry trampoline (unwraps PlayerCon* and calls member Recv)  
**Called by:** Detached thread spawned by `THREAD_CREATE_DETACHED`  
**Calls:** `con->Recv()` (member function)  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Delegates to member Recv  
**Notes:** Static function matching `void*(*)(void*)` signature required by pthread. Casts void* back to PlayerCon* and calls member function.

---

### `PlayerCon::Aquire` (game_svr.cpp:781-795)

**Signature:**
```cpp
static PlayerCon* Aquire()
```
**Purpose:** Allocate player connection slot (synchronized)  
**Called by:** `ServerLoop` (game_svr.cpp:1026)  
**Calls:** `RWLOCK_WRITE_LOCK(cs)`, `RWLOCK_WRITE_UNLOCK(cs)`  
**Globals read:** `max_players`, `clients`, `client_id[]`, `players[]`  
**Globals mutated:** `clients` (incremented), `con->release_index`  
**Side effects:** Allocates next available player slot, increments client count  
**Notes:** Thread-safe allocation under write lock. Returns NULL if server full (clients >= cap). Sets release_index for O(1) removal. Cap is min(max_players, MAX_CLIENTS).

---

### `PlayerCon::Release` (game_svr.cpp:797-849)

**Signature:**
```cpp
void Release()
```
**Purpose:** Free player connection slot, broadcast exit, cleanup resources (synchronized)  
**Called by:** `PlayerCon::Recv` (8 call sites on error paths)  
**Calls:** `RWLOCK_WRITE_LOCK(cs)`, `RWLOCK_WRITE_UNLOCK(cs)`, `TCP_CLOSE(client_socket)`, `malloc` (exit broadcast), `BroadCast::Send`, `INTERLOCKED_DEC`, `free`, `RWLOCK_DELETE(rwlock)`, `printf`  
**Globals read:** `clients`, `client_id[]`, `players[]`  
**Globals mutated:** `clients` (decremented), `client_id[]` (swap), `players[mov_id].release_index`, `joined`, `has_state`, `head`, `tail`, `broadcasts`, `rwlock`, `client_socket`  
**Side effects:** Closes socket, broadcasts exit, frees pending broadcasts, destroys rwlock, updates client list  
**Notes:** Thread-safe under write lock. Broadcasts exit if joined. Frees pending broadcasts (decrement refs). Swaps client_id array to maintain dense packing (O(1) removal). Sets rwlock to 0xDEADBEEF sentinel (debug aid). Prints "DISCONNECTED ID: N".

**WHY swap trick:** `client_id[release_index] = client_id[--clients]` moves last active client into freed slot, maintains dense array.

---

### `Headers::cb` (game_svr.cpp:314-420)

**Signature:**
```cpp
static int cb(const char* header, const char* value, void* param)
```
**Purpose:** HTTP header validation callback for WebSocket handshake  
**Called by:** `HTTP_READ` (invoked for each header:value pair)  
**Calls:** `strcmp`, `strncmp`, `strlen`, `strcpy`  
**Globals read:** None  
**Globals mutated:** `h->parsed` (bitmask), `h->key`, `h->keylen`  
**Side effects:** Validates headers, copies Sec-WebSocket-Key  
**Notes:** Returns 0 on valid header, -3 on protocol violation. Validates:
- First line: `GET /ws/y8/ HTTP/1.1` (header=NULL)
- Sec-WebSocket-Version: must be "13"
- Sec-WebSocket-Key: copied to h->key (max 63 chars)
- Upgrade: must be "WebSocket" or "websocket"
- Connection: must contain "Upgrade" (comma-delimited list)
- Content-Length: must be "0" (if present)

**parsed bitmask:** Ensures each header appears exactly once (duplicate = protocol violation).

---

### `ServerLoop` (game_svr.cpp:926-1064)

**Signature:**
```cpp
int ServerLoop(const char* port)
```
**Purpose:** Main server loop (bind socket, accept connections, spawn handlers)  
**Called by:** `main` (game_svr.cpp:1229)  
**Calls:** `TCP_INIT`, `TCP_CLEANUP`, `getaddrinfo`, `freeaddrinfo`, `socket`, `setsockopt`, `bind`, `listen`, `accept`, `TCP_CLOSE`, `memset`, `RWLOCK_CREATE`, `RWLOCK_DELETE`, `PlayerCon::Aquire`, `PlayerCon::Start`, `PlayerCon::Stop`, `printf`  
**Globals read:** `isRunning`, `max_players`, `PlayerCon::clients`, `PlayerCon::client_id[]`, `PlayerCon::players[]`  
**Globals mutated:** `PlayerCon::cs`, `PlayerCon::clients`, `PlayerCon::client_id[]`  
**Side effects:** Binds socket, listens for connections, spawns client threads, blocks until shutdown  
**Notes:** Initialize Winsock → resolve address → bind → listen → accept loop → cleanup.

**Accept loop:**
1. `accept()` blocks until client connects
2. Set SO_KEEPALIVE + TCP_NODELAY
3. `PlayerCon::Aquire()` → get free slot or reject (server full)
4. `PlayerCon::Start()` → spawn detached thread
5. Repeat until `isRunning==false`

**Shutdown:**
1. Close listen socket
2. Stop all active clients (close sockets)
3. Delete global rwlock
4. Cleanup Winsock

**Platform-specific:** `SO_REUSEPORT` set on POSIX only (Windows doesn't support).

---

### `main` (game_svr.cpp:1075-1236)

**Signature:**
```cpp
int main(int argc, char* argv[])
```
**Purpose:** Server entry point (parse args, load world, run server loop)  
**Called by:** OS program loader  
**Calls:** `strcmp`, `atoi`, `printf`, `signal`, `realpath`, `strrchr`, `strcpy`, `memcpy`, `strstr`, `sprintf`, `LoadSprites`, `fopen`, `LoadTerrain`, `fread`, `LoadWorld`, `GetFirstMesh`, `GetNextMesh`, `GetMeshName`, `UpdateMesh`, `RebuildWorld`, `fclose`, `ServerLoop`, `DeleteWorld`, `DeleteTerrain`, `FreeSprites`  
**Globals read:** None  
**Globals mutated:** `max_players`, `base_path`, `terrain`, `world`, `mat[]`  
**Side effects:** Parses CLI args, registers signal handlers, loads world data, runs server loop, cleanup on exit  
**Notes:** Command-line args:
- `--port N`: Listen port (default: 8080)
- `--max-players N`: Max concurrent players (default: 8, max: 50)
- `--help`: Print usage

**Initialization:**
1. Parse args
2. Register SIGINT/SIGTERM handlers (POSIX only)
3. Resolve executable path to `base_path` (strips `/.run/` suffix)
4. Load sprites (shared code)
5. Load terrain from `a3d/game_map_y8.a3d`
6. Load materials (256 entries)
7. Load world (BSP tree, instances)
8. Update meshes from `meshes/` directory
9. Rebuild world BSP with updated instance bboxes

**Cleanup:**
1. `DeleteWorld(world)`
2. `DeleteTerrain(terrain)`
3. `FreeSprites()`

**Platform-specific:** `realpath()` on POSIX, `GetFullPathNameA()` on Windows. `/.run/` stripping for build artifact directory detection.

---

## Static Data Structures

### `PlayerCon::cs` (game_svr.cpp:919)

**Type:** `RWLOCK_HANDLE*` (static)  
**Purpose:** Global read-write lock protecting client list  
**Initialized by:** `ServerLoop` (game_svr.cpp:1002) via `RWLOCK_CREATE()`  
**Destroyed by:** `ServerLoop` (game_svr.cpp:1059) via `RWLOCK_DELETE()`  
**Protected data:** `PlayerCon::clients`, `PlayerCon::client_id[]`, iteration over `PlayerCon::players[]`  
**Notes:** Write lock for Aquire/Release, read lock for iteration during broadcast/join.

---

### `PlayerCon::clients` (game_svr.cpp:920)

**Type:** `int` (static)  
**Purpose:** Count of active player connections (dense)  
**Initialized by:** `ServerLoop` (game_svr.cpp:998) to 0  
**Modified by:** `PlayerCon::Aquire` (++clients), `PlayerCon::Release` (--clients)  
**Protected by:** `PlayerCon::cs` write lock  
**Notes:** Always in range [0, min(max_players, MAX_CLIENTS)]. Indices into client_id[].

---

### `PlayerCon::client_id[]` (game_svr.cpp:921)

**Type:** `int[MAX_CLIENTS]` (static)  
**Purpose:** Dense array of active player IDs (0-49)  
**Initialized by:** `ServerLoop` (game_svr.cpp:999-1000) to [0,1,2,...,49]  
**Modified by:** `PlayerCon::Release` (swap to maintain dense packing)  
**Protected by:** `PlayerCon::cs` write lock  
**Notes:** First `clients` entries are active, remainder are free. Swapped on release to avoid holes.

---

### `PlayerCon::players[]` (game_svr.cpp:922)

**Type:** `PlayerCon[MAX_CLIENTS]` (static)  
**Purpose:** Pool of 50 player connection instances  
**Initialized by:** `ServerLoop` (game_svr.cpp:997) via `memset` to zero  
**Indexed by:** `client_id[i]` for active clients  
**Protected by:** Per-instance `rwlock` (PlayerCon::rwlock)  
**Notes:** Fixed-size pool, never reallocated. Each instance has independent rwlock.

---

### `isRunning` (game_svr.cpp:924)

**Type:** `volatile bool` (global)  
**Purpose:** Server shutdown flag  
**Initialized by:** Static initialization to `true`  
**Modified by:** `exit_handler` (set to false on SIGINT/SIGTERM)  
**Read by:** `ServerLoop` (game_svr.cpp:1006,1039)  
**Notes:** Volatile ensures signal handler writes visible to main loop. No lock (single writer).

---

### `server` (game_svr.cpp:113)

**Type:** `Server*` (global)  
**Purpose:** Fulfill extern declaration in game.cpp (currently NULL)  
**Initialized by:** Static initialization to 0  
**Modified by:** Never (stub implementation)  
**Notes:** game.cpp expects this global, but server build doesn't use Server class.

---

### `max_players` (game_svr.cpp:112)

**Type:** `int` (static)  
**Purpose:** Runtime cap on concurrent connections  
**Initialized by:** Static initialization to 8  
**Modified by:** `main` (game_svr.cpp:1087) via --max-players arg  
**Read by:** `PlayerCon::Aquire`, `PlayerCon::Recv` (join response)  
**Notes:** Clamped to [1, MAX_CLIENTS=50]. Independent of MAX_CLIENTS constant.

---

### `base_path` (game_svr.cpp:109)

**Type:** `char[1024]` (global)  
**Purpose:** Server executable directory (for asset loading)  
**Initialized by:** `main` (game_svr.cpp:1108-1162) via path resolution  
**Read by:** `main` (game_svr.cpp:1172,1198,1218) for a3d/meshes paths  
**Notes:** Stripped of `/.run/` suffix if present. Ends with `/`.

---

## Summary Statistics

**Files analyzed:** 3 (network.h, network.cpp, game_svr.cpp)  
**Functions documented:** 49  
- network.cpp: 28 (14 socket/protocol, 14 threading/sync platform pairs)  
- game_svr.cpp: 13 (1 main, 1 loop, 6 PlayerCon, 3 utility, 2 stubs)  
- Trampolines/callbacks: 3 (THREAD_HANDLE::wrap/wrap_detached, Headers::cb)  
- Static data structures: 8

**Platform abstraction coverage:**
- Socket: TCP_INIT/CLOSE/CLEANUP/WRITE/READ (5 functions)  
- HTTP/WebSocket: HTTP_READ, WS_WRITE, WS_READ (3 functions)  
- Threading: THREAD_CREATE/JOIN/CREATE_DETACHED/SLEEP (4 functions)  
- RWLock: RWLOCK_CREATE/DELETE/READ_LOCK/READ_UNLOCK/WRITE_LOCK/WRITE_UNLOCK (6 functions)  
- Mutex: MUTEX_CREATE/DELETE/LOCK/UNLOCK (4 functions)  
- Interlocked: INTERLOCKED_DEC/INC/SUB/ADD (4 functions)  
- Trampolines: THREAD_HANDLE::wrap/wrap_detached (2 Windows-only)

**Critical patterns:**
- Reference counting: BroadCast uses INTERLOCKED_INC/DEC, auto-frees at refs==0
- Dense client list: client_id[] swap trick in Release maintains O(1) remove
- Per-connection locks: Each PlayerCon has rwlock, global cs protects list
- Detached threads: PlayerCon::Start spawns non-joinable thread (auto-cleanup)
- WebSocket handshake: HTTP header validation → SHA1+Base64 accept key → 101 response
- Broadcast backpressure: Drops pose at 500 pending, closes socket at 5000

**Implementation status:** Server core is functional (WebSocket handshake, client management, broadcast). Game.cpp integration stubs (Server::Send/Proc, akAPI_Exec) not yet implemented.

