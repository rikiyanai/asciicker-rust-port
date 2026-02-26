# Asciicker Network System Documentation

## Overview

The Asciicker network system (`network.h` and `network.cpp`) provides cross-platform multiplayer networking capabilities for the game. The system implements a client-server architecture using TCP sockets with WebSocket framing for browser compatibility. The codebase is designed to compile on both Windows and POSIX systems (Linux, macOS) using platform abstraction layers.

## 1. Network Architecture

### 1.1 Client-Server Model

The Asciicker network system operates as a **client-server architecture** rather than peer-to-peer. This design choice simplifies game state management, as the server acts as the authoritative source for game data. Clients connect to a central server, which then broadcasts relevant updates to all connected clients. This architecture provides several advantages for a real-time multiplayer game like Asciicker.

The server maintains the authoritative game state and distributes updates to all connected clients. When a client sends an update (such as a position change or chat message), the server processes it and broadcasts it to all other connected clients. This ensures consistency across all clients and prevents cheating by requiring server validation of game actions. The client-server model also simplifies NAT traversal since clients only need to connect to a single server rather than establishing peer-to-peer connections with every other player.

### 1.2 Transport Layer

The network system uses **TCP (Transmission Control Protocol)** as its transport layer. TCP provides reliable, ordered, error-checked delivery of data between the client and server. This reliability is important for game state synchronization, as lost or reordered packets could cause visual glitches or game logic errors. The use of TCP eliminates the need for the application layer to handle packet loss and reordering, simplifying the implementation.

While TCP introduces some latency compared to UDP, the Asciicker game appears to prioritize correctness over raw speed for its ASCII-based gameplay. The relatively low data rate of ASCII graphics (compared to 3D graphics) makes TCP's overhead acceptable. The implementation includes WebSocket framing on top of TCP, which enables browser-based clients to connect to the same server, expanding the game's accessibility.

### 1.3 Platform Abstraction

The network code implements extensive platform abstraction to support both Windows and POSIX systems from a single codebase. This abstraction is handled through conditional compilation with `#ifdef _WIN32` to select between Windows-specific and POSIX-specific implementations. The abstraction covers socket operations, threading, and synchronization primitives, allowing game code to use consistent APIs regardless of the target platform.

On Windows, the system uses Winsock for socket operations, CreateThread for threading, CRITICAL_SECTION for mutexes, and SRWLOCK for read-write locks. On POSIX systems, it uses standard BSD sockets, pthreads, pthread_mutex_t, and pthread_rwlock_t. This abstraction layer is defined in network.cpp and wrapped behind the unified types and functions declared in network.h. The game code uses types like TCP_SOCKET, THREAD_HANDLE, MUTEX_HANDLE, and RWLOCK_HANDLE without any platform-specific conditionals.

## 2. Protocol Design

### 2.1 Token-Based Message Protocol

The Asciicker network protocol is a **token-based binary protocol** where the first byte of every message identifies the message type. This design provides efficient message parsing since the receiver can immediately determine the message type and know exactly how to interpret the remaining bytes. The protocol uses a simple convention: uppercase tokens represent client requests, while lowercase tokens represent server responses or broadcasts.

The token-based design allows for very fast message parsing in the game loop, which is critical for maintaining smooth multiplayer performance. Instead of parsing text-based protocols (like HTTP or JSON), the binary structs are directly cast from network bytes to C structures, minimizing CPU overhead. The fixed-size nature of most message structs ensures predictable network framing, making it easy to determine where one message ends and the next begins.

### 2.2 Binary Packing

All protocol structures use `#pragma pack(push,1)` to eliminate padding between struct members. This ensures that the binary layout of structures matches exactly what is sent over the network, eliminating the need for serialization/deserialization logic. Without packed structures, compiler-inserted padding bytes would cause misinterpretation of messages on the receiving end.

The protocol assumes **little-endian byte order**, which is the native format for x86 and ARM processors (used in both client and server deployments). This assumption holds for the vast majority of deployments, but would require endian-swap logic for cross-platform compatibility with big-endian systems. The protocol documentation explicitly notes this assumption, indicating that any changes to the binary struct layout would break the network contract.

### 2.3 WebSocket Framing

The network system implements **WebSocket framing (RFC 6455)** on top of TCP connections. WebSocket provides several advantages for browser-based gaming: it uses HTTP for the initial handshake, allowing it to work through most HTTP proxies and NAT configurations. After the handshake, the connection upgrades to a bidirectional binary/frame protocol that is more efficient than HTTP for real-time communication.

The WebSocket implementation in network.cpp includes both encoding (WS_WRITE) and decoding (WS_READ) functions. The encoding function handles frame construction with proper FIN bit handling, opcode selection (text, binary, close, ping, pong), and payload length encoding. The decoding function handles frame parsing, mask removal (required for client-to-server frames per RFC 6455), and automatic response to control frames like ping and close.

WebSocket framing also supports **message splitting** through the `split` parameter in WS_WRITE. This feature allows large messages to be divided into multiple frames, preventing a single large message from causing latency spikes for other messages in the queue. The split parameter specifies the maximum size of each frame, and the implementation calculates equal-sized frames with the last frame potentially being smaller.

## 3. Message Types

The Asciicker protocol defines seven distinct message types, each serving a specific purpose in multiplayer communication. These messages cover the essential multiplayer functionality: joining the game, leaving the game, position updates, chat communication, and latency measurement. The following sections detail each message type and its purpose.

### 3.1 Join Request (STRUCT_REQ_JOIN)

**Token:** 'J' (uppercase, client request)

The join request is sent by a client when connecting to the server. It contains the player's display name, which is limited to 31 characters plus a null terminator. Upon receiving this request, the server validates the name, assigns a unique player ID, and responds with the join response message. The name is likely sanitized by the server to prevent injection of control characters or inappropriate content.

```c
struct STRUCT_REQ_JOIN
{
    uint8_t token;  // 'J'
    char name[31];  // null-terminated player name
};
```

The join request represents the first message in the multiplayer handshake. The server must process this message before any other client messages, as the client is not considered connected until it receives a valid join response with its assigned player ID.

### 3.2 Join Response (STRUCT_RSP_JOIN)

**Token:** 'j' (lowercase, server response)

The join response is sent by the server in reply to a successful join request. It assigns the client a unique player ID and informs the client of the maximum number of clients allowed on the server. The player ID is used in all subsequent messages to identify the client, and the maxcli value allows the client to display server capacity information.

```c
struct STRUCT_RSP_JOIN
{
    uint8_t token;   // 'j'
    uint8_t maxcli;  // maximum number of clients allowed
    uint16_t id;     // assigned player ID for this client
};
```

### 3.3 Broadcast Join (STRUCT_BRC_JOIN)

**Token:** 'j' (note: token collision with response)

When a new player successfully joins, the server broadcasts a join message to all existing clients. This informs them of the new player's presence, including their position, animation state, sprite, name, and assigned ID. The broadcast is identical in structure to the join response but contains the new player's complete initial state.

```c
struct STRUCT_BRC_JOIN
{
    uint8_t token;   // 'j'
    uint8_t anim;    // animation state
    uint8_t frame;  // animation frame
    uint8_t am;     // action/mount state
    float pos[3];   // position (x, y, z)
    float dir;      // direction
    uint16_t id;    // player ID
    uint16_t sprite;// sprite identifier
    char name[32];  // player name
};
```

The protocol notes that while there is a token collision between the join response and broadcast join messages, this does not cause issues because the response is sent synchronously before any broadcasts occur. Clients expect exactly one 'j' token as a direct response to their join request, followed by any number of broadcast 'j' tokens from other players joining.

### 3.4 Broadcast Exit (STRUCT_BRC_EXIT)

**Token:** 'e' (lowercase, server broadcast)

When a player disconnects from the game, the server broadcasts an exit message to all remaining clients. This message contains only the player ID, allowing clients to remove the player from their local representation of the game world. The exit message is minimal to reduce bandwidth usage, as disconnection notifications must be reliable but carry minimal information.

```c
struct STRUCT_BRC_EXIT
{
    uint8_t token;  // 'e'
    uint8_t pad;    // padding byte for alignment
    uint16_t id;    // ID of player who exited
};
```

### 3.5 Pose Update Request (STRUCT_REQ_POSE)

**Token:** 'P' (uppercase, client request)

Clients send pose updates to communicate their current position, animation state, and orientation to the server. This is likely the most frequent message type in the protocol, as it carries real-time movement data. The message includes animation state (anim, frame), position in 3D space (pos[3]), facing direction (dir), and sprite identifier.

```c
struct STRUCT_REQ_POSE
{
    uint8_t token;   // 'P'
    uint8_t anim;    // animation identifier
    uint8_t frame;   // animation frame number
    uint8_t am;      // action/mount state
    float pos[3];   // x, y, z position
    float dir;      // facing direction
    uint16_t sprite; // sprite identifier
};
```

The pose update is sent by clients to report their current state to the server. The server then broadcasts this information to all other clients, who update their local representation of that player. This creates the illusion of real-time multiplayer movement.

### 3.6 Broadcast Pose Update (STRUCT_BRC_POSE)

**Token:** 'p' (lowercase, server broadcast)

The broadcast pose message is identical to the pose request but includes the sender's player ID. This allows clients to distinguish between their own pose (which they know internally) and other players' poses received from the server.

```c
struct STRUCT_BRC_POSE
{
    uint8_t token;   // 'p'
    uint8_t anim;    // animation identifier
    uint8_t frame;   // animation frame number
    uint8_t am;      // action/mount state
    float pos[3];   // x, y, z position
    float dir;      // facing direction
    uint16_t sprite; // sprite identifier
    uint16_t id;     // sender's player ID
};
```

### 3.7 Talk Request (STRUCT_REQ_TALK)

**Token:** 'T' (uppercase, client request)

Clients send talk messages to communicate text chat to other players. The message includes a length byte indicating the actual message length (allowing the client to avoid sending the full 256-byte buffer) followed by the message string. The server broadcasts this message to all connected clients.

```c
struct STRUCT_REQ_TALK
{
    uint8_t token;   // 'T'
    uint8_t len;     // actual message length
    uint8_t str[256]; // message string (trim to actual size)
};
```

### 3.8 Broadcast Talk (STRUCT_BRC_TALK)

**Token:** 't' (lowercase, server broadcast)

The broadcast talk message includes the sender's player ID along with the chat message. This allows clients to display the sender's name (looked up by ID from the join messages) alongside their message.

```c
struct STRUCT_BRC_TALK
{
    uint8_t token;   // 't'
    uint8_t len;     // actual message length
    uint16_t id;     // sender's player ID
    uint8_t str[256]; // message string
};
```

### 3.9 Lag Test Request (STRUCT_REQ_LAG)

**Token:** 'L' (uppercase, client request)

Clients send lag test messages to measure network latency. The message includes a 3-byte timestamp that the server echoes back in its response. By comparing the echoed timestamp with the current time, the client can calculate the round-trip time. The timestamp is only 3 bytes to minimize bandwidth usage for what is essentially a keepalive message.

```c
struct STRUCT_REQ_LAG
{
    uint8_t token;     // 'L'
    uint8_t stamp[3];  // 3-byte timestamp
};
```

### 3.10 Lag Test Response (STRUCT_RSP_LAG)

**Token:** 'l' (lowercase, server response)

The server echoes back the lag test timestamp with no modification. This allows the client to calculate the round-trip time and estimate one-way latency (assuming symmetric network paths).

```c
struct STRUCT_RSP_LAG
{
    uint8_t token;     // 'l'
    uint8_t stamp[3];  // echoed 3-byte timestamp
};
```

## 4. Connection Handling

### 4.1 Socket Abstraction

The network system provides a cross-platform socket abstraction layer. On Windows, sockets are represented by the SOCKET type (which is equivalent to a HANDLE), while on POSIX systems, they are represented by standard file descriptors (int). The abstraction defines INVALID_TCP_SOCKET as INVALID_SOCKET on Windows and -1 on POSIX.

Socket initialization (TCP_INIT) calls WSAStartup on Windows to initialize the Winsock library, while being a no-op on POSIX. Similarly, TCP_CLEANUP calls WSACleanup on Windows but does nothing on POSIX. The TCP_CLOSE function maps to closesocket() on Windows and close() on POSIX, handling the platform difference in socket closure.

### 4.2 Blocking I/O

The TCP_WRITE and TCP_READ functions implement blocking I/O with retry logic. TCP_WRITE continues sending until all requested bytes are written (or an error occurs), handling partial sends gracefully. TCP_READ similarly continues receiving until all requested bytes are read. This blocking behavior simplifies the protocol implementation, as callers can assume that when these functions return successfully, exactly the requested number of bytes have been transferred.

```c
int TCP_WRITE(TCP_SOCKET s, const uint8_t* buf, int size)
{
    int l = size;
    while (l > 0)
    {
        int w = send(s, (const char*)buf, l, 0);
        if (w <= 0)
            return w;
        l -= w;
        buf += w;
    }
    return size;
}
```

### 4.3 HTTP Handshake Support

The HTTP_READ function provides support for parsing HTTP headers, which is necessary for WebSocket upgrade handshakes. It reads HTTP headers line by line, invoking a callback for each header-value pair. The function handles chunked transfer encoding and returns any bytes read beyond the headers (body overread) so the caller can process them.

The callback-based design avoids allocating large header dictionaries in memory. Instead, each header is processed immediately as it is parsed, and the caller can reject headers early by returning a negative value from the callback. This is memory-efficient and allows early rejection of unwanted connections.

### 4.4 WebSocket Control Frames

The WebSocket implementation automatically handles control frames (close, ping, pong) according to RFC 6455. When receiving a close frame (opcode 0x8), the implementation responds with a close frame and returns -1 to signal connection termination. When receiving a ping frame (opcode 0x9), the implementation automatically responds with a pong frame containing the same payload. Pong frames (opcode 0xA) are silently ignored, as they are responses to the server's own pings.

This automatic handling ensures that WebSocket connections remain healthy according to the RFC specification without requiring explicit application-layer handling of these control messages.

### 4.5 Threading Model

The network system provides three threading primitives for different use cases. THREAD_CREATE spawns a joinable thread that can be waited on using THREAD_JOIN, which blocks until the thread exits and returns the thread's exit value. This is useful for threads that need to communicate results back to the caller.

THREAD_CREATE_DETACHED spawns a fire-and-forget thread that cannot be joined. This is useful for background tasks that do not need to communicate results, such as periodic cleanup operations. The thread is detached immediately upon creation, avoiding resource leaks.

The Windows implementation uses CreateThread with a wrapper struct to convert between the Windows DWORD WINAPI signature and the portable void* (*)(void*) signature. The POSIX implementation uses pthread_create directly, as the signatures are compatible. Both implementations provide consistent behavior through the THREAD_HANDLE abstraction.

### 4.6 Synchronization Primitives

The network system provides three synchronization primitives. MUTEX_HANDLE provides exclusive locking using CRITICAL_SECTION on Windows and pthread_mutex_t on POSIX. This is suitable for protecting a single shared resource, such as a message queue.

RWLOCK_HANDLE provides read-write locking using SRWLOCK on Windows and pthread_rwlock_t on POSIX. Multiple readers can hold the lock simultaneously, but only one writer can hold it at a time. This is suitable for protecting data that is read frequently but written rarely, such as world or terrain data.

INTERLOCKED functions provide atomic operations for lock-free synchronization. These include increment, decrement, add, and subtract operations. On Windows, they map to InterlockedIncrement, InterlockedDecrement, and InterlockedAdd. On POSIX, they map to GCC built-in atomic operations (__sync_fetch_and_add, __sync_fetch_and_sub). These are used for reference counting and other scenarios requiring atomic updates without the overhead of locks.

## 5. Latency Compensation

### 5.1 Round-Trip Time Measurement

The Asciicker network system includes built-in latency measurement through the LAG message pair. Clients can send a STRUCT_REQ_LAG message containing a 3-byte timestamp. The server responds with STRUCT_RSP_LAG containing the same timestamp. By comparing the echoed timestamp with the current time, the client can calculate the round-trip time (RTT).

The timestamp uses only 3 bytes (24 bits) to minimize bandwidth overhead, likely sufficient for millisecond-level timestamps within some time period (likely wrapping every ~4.6 hours based on millisecond resolution). This is a simple but effective mechanism for basic latency estimation.

### 5.2 WebSocket Ping/Pong

Beyond the application-level LAG messages, the WebSocket protocol itself supports ping/pong frames as keepalive mechanisms. The WS_READ function automatically handles incoming ping frames by responding with pong frames containing the same payload. This helps keep connections alive through NAT mappings and firewalls that may close idle connections.

While the current implementation doesn't explicitly send WebSocket pings from the server, the infrastructure is in place to support this. The ability to send ping frames would allow for more frequent latency checks without adding application-level message overhead.

### 5.3 Message Splitting for Latency Reduction

The WS_WRITE function's split parameter provides a mechanism for preventing large messages from causing latency spikes. When enabled, large messages are divided into multiple smaller frames. This allows other messages to be interleaved between frames, preventing a single large message from blocking the entire network pipe.

This is particularly important for the ASCII game context where chat messages (up to 256 bytes) or bulk updates might otherwise cause delays for more time-sensitive pose updates. By splitting large messages, the implementation ensures that critical game state updates can be delivered more promptly.

### 5.4 Client-Side Prediction (Not in Network Layer)

The network layer provides the foundation for latency measurement, but client-side latency compensation (such as client-side prediction and interpolation) would be implemented at a higher layer. The current network layer provides raw position updates through the pose messages, but does not implement any prediction or smoothing algorithms. A Rust port would likely need to implement client-side prediction in the game logic layer to provide smooth gameplay on high-latency connections.

## 6. Protocol Summary Table

| Token | Direction | Type | Purpose |
|-------|-----------|------|---------|
| 'J' | Client -> Server | Request | Join request with player name |
| 'j' | Server -> Client | Response | Join response with player ID and max clients |
| 'j' | Server -> Client | Broadcast | Broadcast new player join to all clients |
| 'e' | Server -> Client | Broadcast | Broadcast player exit/disconnection |
| 'P' | Client -> Server | Request | Client pose update (position, animation) |
| 'p' | Server -> Client | Broadcast | Broadcast pose update to all clients |
| 'T' | Client -> Server | Request | Client chat message |
| 't' | Server -> Client | Broadcast | Broadcast chat message to all clients |
| 'L' | Client -> Server | Request | Latency test request with timestamp |
| 'l' | Server -> Client | Response | Latency test response echoing timestamp |

## 7. Implementation Files

- **network.h** (210 lines): Header file containing protocol structure definitions, cross-platform type definitions, and function declarations.
- **network.cpp** (980 lines): Implementation file containing platform-specific implementations for Windows and POSIX systems.

## 8. Rust Port Considerations

When porting this network system to Rust, several considerations apply. First, the binary protocol structures can be represented using Rust structs with #[repr(C, packed)] to match the C layout exactly. Second, the platform abstraction layer can be simplified using Rust's cross-platform crates like socket2 or mio for I/O, and std::thread or crossbeam for threading. Third, the WebSocket implementation could potentially be replaced with an existing Rust crate (like tokio-tungstenite) rather than implementing RFC 6455 manually. Fourth, the synchronization primitives can use Rust's std::sync::Mutex, RwLock, and atomic types. Fifth, the token-based protocol parsing is well-suited to Rust's pattern matching, which could provide a more idiomatic implementation than the C switch-on-token approach.
