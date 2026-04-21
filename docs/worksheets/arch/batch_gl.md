# OpenGL Wrapper Functions (Batch GL)

This document provides detailed reference for all OpenGL wrapper and helper functions used in the Asciicker engine.

## Overview

Two primary modules handle OpenGL operations:
- **gl45_emu.cpp**: OpenGL 4.5 DSA emulation layer for GL 3.3 compatibility
- **imgui_impl_opengl3.cpp**: ImGui OpenGL 3.x renderer implementation

---

## gl45_emu.cpp Functions

### `gl3CopyImageSubData` (gl45_emu.cpp:102-119)

**Signature:** `void gl3CopyImageSubData(GLuint srcName, GLenum srcTarget, GLint srcLevel, GLint srcX, GLint srcY, GLint srcZ, GLuint dstName, GLenum dstTarget, GLint dstLevel, GLint dstX, GLint dstY, GLint dstZ, GLsizei srcWidth, GLsizei srcHeight, GLsizei srcDepth)`

**Purpose:** Stub function for image-to-image copy (unimplemented, all calls ported to alternative methods)

**Called by:** No callers (all production calls have been ported away)

**Calls:** `glCopyImageSubData` (in passthrough mode only, USE_GL3=0), `printf` (warning)

**Globals read:** None

**Globals mutated:** `warn_once` (static local, prevents repeated warnings)

**Side effects:** Prints warning to stdout on first call in GL 3.3 mode

**Notes:** All glCopyImageSubData calls were ported to CPU-side copy methods (glGetTexImage + glTexSubImage2D). Logs once-per-session warning if accidentally invoked.

---

### `gl3GetTextureSubImage` (gl45_emu.cpp:125-142)

**Signature:** `void gl3GetTextureSubImage(GLuint texture, GLint level, GLint xoffset, GLint yoffset, GLint zoffset, GLsizei width, GLsizei height, GLsizei depth, GLenum format, GLenum type, GLsizei bufSize, void *pixels)`

**Purpose:** Stub function for texture pixel readback (unimplemented, desktop tools only)

**Called by:**
- `asciiid.cpp:1639` (font editor pixel reading)
- `asciiid.cpp:7644` (glyph coverage calculator)
- `game_app.cpp:900` (font editor)

**Calls:** `glGetTextureSubImage` (in passthrough mode only, USE_GL3=0), `printf` (warning)

**Globals read:** None

**Globals mutated:** `warn_once` (static local, prevents repeated warnings)

**Side effects:** Prints warning to stdout on first call in GL 3.3 mode

**Notes:** Only used by font editor and glyph coverage calculator (desktop-only development tools). GL 3.3 emulation could use glReadPixels + framebuffer if needed later.

---

### `gl3TextureStorage2D` (gl45_emu.cpp:157-184)

**Signature:** `void gl3TextureStorage2D(GLuint tex, GLint levels, GLenum ifmt, GLsizei w, GLsizei h)`

**Purpose:** Allocate immutable 2D texture storage with mipmap levels

**Called by:**
- `asciiid.cpp:1152` (shade texture)
- `asciiid.cpp:1587` (font texture)
- `asciiid.cpp:1973` (ANSI terminal buffer)
- `game_app.cpp:848` (font texture)

**Calls:** `glGetIntegerv`, `glBindTexture`, `glTexImage2D`, `glTexParameteri`

**Globals read:** None

**Globals mutated:** GL_TEXTURE_BINDING_2D (saved/restored)

**Side effects:** Allocates GPU texture memory, modifies GL texture binding state (restored)

**Notes:** Loops through mipmap levels, halving dimensions each level (min 1×1). Integer formats (GL_RGBA8UI, GL_R16UI) require GL_RGBA_INTEGER/GL_RED_INTEGER format parameter. Sets GL_TEXTURE_MAX_LEVEL explicitly.

---

### `gl3TextureStorage3D` (gl45_emu.cpp:186-210)

**Signature:** `void gl3TextureStorage3D(GLuint tex, GLint levels, GLenum ifmt, GLsizei w, GLsizei h, GLsizei d)`

**Purpose:** Allocate immutable 3D texture storage with mipmap levels

**Called by:**
- `asciiid.cpp:10948` (palette 3D lookup texture 256×256×256)

**Calls:** `glGetIntegerv`, `glBindTexture`, `glTexImage3D`, `glTexParameteri`

**Globals read:** None

**Globals mutated:** GL_TEXTURE_BINDING_3D (saved/restored)

**Side effects:** Allocates GPU texture memory, modifies GL texture binding state (restored)

**Notes:** BUG at line 205: calls `glTexParameteri(GL_TEXTURE_2D, ...)` instead of GL_TEXTURE_3D. Should be GL_TEXTURE_3D for 3D textures.

---

### `gl3CreateTextures` (gl45_emu.cpp:213-220)

**Signature:** `void gl3CreateTextures(GLenum target, GLsizei num, GLuint* arr)`

**Purpose:** Generate texture object names

**Called by:**
- `asciiid.cpp:1150, 1586, 1972, 10947` (shade, font, ANSI, palette textures)
- `game_app.cpp:847` (font texture)

**Calls:** `glGenTextures` (GL 3.3), `glCreateTextures` (GL 4.5 passthrough)

**Globals read:** None

**Globals mutated:** None (writes to output array parameter)

**Side effects:** Allocates texture object names in OpenGL context

**Notes:** Simple wrapper; GL 3.3 mode uses glGenTextures, GL 4.5 uses native DSA glCreateTextures.

---

### `gl3TextureSubImage2D` (gl45_emu.cpp:222-233)

**Signature:** `void gl3TextureSubImage2D(GLuint tex, GLint level, GLint x, GLint y, GLsizei w, GLsizei h, GLenum fmt, GLenum type, const void *pix)`

**Purpose:** Upload pixel data to region of 2D texture

**Called by:**
- `asciiid.cpp:1155, 1170, 1590, 1633, 3345, 3350, 7014` (material, font, ANSI updates)
- `game_app.cpp:851, 894` (font updates)

**Calls:** `glGetIntegerv`, `glBindTexture`, `glTexSubImage2D`

**Globals read:** None

**Globals mutated:** GL_TEXTURE_BINDING_2D (saved/restored)

**Side effects:** Uploads pixel data to GPU, modifies GL texture binding state (restored)

**Notes:** Primary texture upload path for dynamic content (ANSI terminal, font glyphs). Bind/unbind pattern preserves caller's texture state.

---

### `gl3BindTextureUnit2D` (gl45_emu.cpp:235-248)

**Signature:** `void gl3BindTextureUnit2D(GLuint unit, GLuint tex)`

**Purpose:** Bind 2D texture to texture unit for shader sampling

**Called by:**
- `asciiid.cpp:3210, 3211, 3495, 3496, 3586, 3587, 3637, 3671, 3688, 7036, 7039, 7055, 7056, 7062, 7063, 7317, 7318, 7343, 7344` (shader texture bindings)

**Calls:** `glGetIntegerv`, `glActiveTexture`, `glBindTexture`

**Globals read:** None

**Globals mutated:** GL_ACTIVE_TEXTURE (saved/restored), GL_TEXTURE_BINDING_2D

**Side effects:** Binds texture to unit for shader access, modifies active texture unit state (restored)

**Notes:** If tex==0, also unbinds GL_TEXTURE_3D to prevent conflicts. Restores original active texture unit after binding.

---

### `gl3BindTextureUnit3D` (gl45_emu.cpp:250-263)

**Signature:** `void gl3BindTextureUnit3D(GLuint unit, GLuint tex)`

**Purpose:** Bind 3D texture to texture unit for shader sampling

**Called by:**
- `asciiid.cpp:3212, 3497, 3588, 7064, 7319, 7345` (palette 3D texture bindings)

**Calls:** `glGetIntegerv`, `glActiveTexture`, `glBindTexture`

**Globals read:** None

**Globals mutated:** GL_ACTIVE_TEXTURE (saved/restored), GL_TEXTURE_BINDING_3D

**Side effects:** Binds 3D texture to unit for shader access, modifies active texture unit state (restored)

**Notes:** If tex==0, also unbinds GL_TEXTURE_2D to prevent conflicts. Restores original active texture unit after binding.

---

### `gl3TextureParameteri2D` (gl45_emu.cpp:265-276)

**Signature:** `void gl3TextureParameteri2D(GLuint tex, GLenum param, GLint val)`

**Purpose:** Set integer texture parameter (filtering, wrapping)

**Called by:**
- `asciiid.cpp:1158-1161, 1595-1598, 1974-1977` (texture filter/wrap modes)
- `game_app.cpp:856-859` (font texture parameters)

**Calls:** `glGetIntegerv`, `glBindTexture`, `glTexParameteri`

**Globals read:** None

**Globals mutated:** GL_TEXTURE_BINDING_2D (saved/restored)

**Side effects:** Modifies texture sampling behavior, modifies GL texture binding state (restored)

**Notes:** Common parameters: GL_TEXTURE_MIN_FILTER, GL_TEXTURE_MAG_FILTER, GL_TEXTURE_WRAP_S/T, GL_CLAMP_TO_EDGE/BORDER.

---

### `gl3TextureParameteri3D` (gl45_emu.cpp:278-289)

**Signature:** `void gl3TextureParameteri3D(GLuint tex, GLenum param, GLint val)`

**Purpose:** Set integer texture parameter for 3D textures

**Called by:**
- `asciiid.cpp:10949-10953` (palette 3D texture filter/wrap modes)

**Calls:** `glGetIntegerv`, `glBindTexture`, `glTexParameteri`

**Globals read:** None

**Globals mutated:** GL_TEXTURE_BINDING_3D (saved/restored)

**Side effects:** Modifies 3D texture sampling behavior, modifies GL texture binding state (restored)

**Notes:** Sets nearest filtering and clamp-to-edge for palette lookup textures.

---

### `gl3TextureParameterfv2D` (gl45_emu.cpp:291-302)

**Signature:** `void gl3TextureParameterfv2D(GLuint tex, GLenum param, GLfloat* val)`

**Purpose:** Set float vector texture parameter (border color, LOD bias)

**Called by:**
- `asciiid.cpp:1600` (border color for font texture)
- `game_app.cpp:861` (border color)

**Calls:** `glGetIntegerv`, `glBindTexture`, `glTexParameterfv`

**Globals read:** None

**Globals mutated:** GL_TEXTURE_BINDING_2D (saved/restored)

**Side effects:** Modifies texture sampling behavior, modifies GL texture binding state (restored)

**Notes:** Primarily used for GL_TEXTURE_BORDER_COLOR (white_transp for font atlases).

---

### `gl3TextureParameterfv3D` (gl45_emu.cpp:304-315)

**Signature:** `void gl3TextureParameterfv3D(GLuint tex, GLenum param, GLfloat* val)`

**Purpose:** Set float vector texture parameter for 3D textures

**Called by:** None (no current usage in codebase)

**Calls:** `glGetIntegerv`, `glBindTexture`, `glTexParameterfv`

**Globals read:** None

**Globals mutated:** GL_TEXTURE_BINDING_3D (saved/restored)

**Side effects:** Modifies 3D texture sampling behavior, modifies GL texture binding state (restored)

**Notes:** Unused currently; provided for API completeness.

---

### `gl3CreateBuffers` (gl45_emu.cpp:317-324)

**Signature:** `void gl3CreateBuffers(GLsizei num, GLuint* arr)`

**Purpose:** Generate buffer object names

**Called by:**
- `asciiid.cpp:1979, 1992, 2289, 2303, 5254` (VBOs for ANSI, mesh, terrain, ghost)

**Calls:** `glGenBuffers` (GL 3.3), `glCreateBuffers` (GL 4.5 passthrough)

**Globals read:** None

**Globals mutated:** None (writes to output array parameter)

**Side effects:** Allocates buffer object names in OpenGL context

**Notes:** Simple wrapper; GL 3.3 mode uses glGenBuffers, GL 4.5 uses native DSA glCreateBuffers.

---

### `gl3NamedBufferStorage` (gl45_emu.cpp:335-346)

**Signature:** `void gl3NamedBufferStorage(GLuint buffer, GLsizeiptr size, const void *data, GLbitfield flags)`

**Purpose:** Allocate immutable buffer storage (VBO/UBO)

**Called by:**
- `asciiid.cpp:1981, 1994, 2290, 2304, 5256` (VBO allocation for rendering pipelines)

**Calls:** `glBindBuffer`, `glBufferData`

**Globals read:** None

**Globals mutated:** GL_COPY_WRITE_BUFFER binding (implicit)

**Side effects:** Allocates GPU buffer memory

**Notes:** Uses GL_COPY_WRITE_BUFFER target to avoid disturbing GL_ARRAY_BUFFER/GL_ELEMENT_ARRAY_BUFFER. BUG at line 342: always uses GL_DYNAMIC_DRAW regardless of flags (should check GL_DYNAMIC_STORAGE_BIT).

---

### `gl3NamedBufferSubData` (gl45_emu.cpp:348-356)

**Signature:** `void gl3NamedBufferSubData(GLuint buffer, GLintptr offset, GLsizeiptr size, const void *data)`

**Purpose:** Update region of buffer storage

**Called by:**
- `asciiid.cpp:1982, 3091, 3101, 3640, 3675` (VBO updates for dynamic geometry)

**Calls:** `glBindBuffer`, `glBufferSubData`

**Globals read:** None

**Globals mutated:** GL_COPY_WRITE_BUFFER binding (implicit)

**Side effects:** Updates GPU buffer contents

**Notes:** Uses GL_COPY_WRITE_BUFFER target to avoid state conflicts. Primary path for dynamic geometry updates (terrain ghosts, instance buffers).

---

### `gl3CreateVertexArrays` (gl45_emu.cpp:358-365)

**Signature:** `void gl3CreateVertexArrays(GLsizei num, GLuint* arr)`

**Purpose:** Generate vertex array object names

**Called by:**
- `asciiid.cpp:1984, 1996, 2292, 2306, 5259` (VAOs for ANSI, mesh, terrain, ghost rendering)

**Calls:** `glGenVertexArrays` (GL 3.3), `glCreateVertexArrays` (GL 4.5 passthrough)

**Globals read:** None

**Globals mutated:** None (writes to output array parameter)

**Side effects:** Allocates VAO names in OpenGL context

**Notes:** Simple wrapper; GL 3.3 mode uses glGenVertexArrays, GL 4.5 uses native DSA glCreateVertexArrays.

---

## imgui_impl_opengl3.cpp Functions

### `ImGui_ImplOpenGL3_Init` (imgui_impl_opengl3.cpp:111-132)

**Signature:** `bool ImGui_ImplOpenGL3_Init(const char* glsl_version)`

**Purpose:** Initialize ImGui OpenGL renderer with GLSL version string

**Called by:**
- `asciiid.cpp:11017` (editor initialization with "#version 330")

**Calls:** `ImGui::GetIO`, `strcpy`, `strcat`

**Globals read:** None

**Globals mutated:** `g_GlslVersionString` (copies GLSL version + newline)

**Side effects:** Sets `io.BackendRendererName` to "imgui_impl_opengl3"

**Notes:** Defaults GLSL version to #version 100 (ES2), #version 300 es (ES3), or #version 130 (desktop) if NULL. Asserts version string fits in 32-byte buffer.

---

### `ImGui_ImplOpenGL3_Shutdown` (imgui_impl_opengl3.cpp:134-137)

**Signature:** `void ImGui_ImplOpenGL3_Shutdown()`

**Purpose:** Cleanup ImGui OpenGL renderer resources

**Called by:**
- `asciiid.cpp:11259` (editor shutdown)

**Calls:** `ImGui_ImplOpenGL3_DestroyDeviceObjects`

**Globals read:** None

**Globals mutated:** None (delegates to DestroyDeviceObjects)

**Side effects:** Deletes shaders, buffers, textures, VAOs

**Notes:** Simple wrapper around device object destruction.

---

### `ImGui_ImplOpenGL3_NewFrame` (imgui_impl_opengl3.cpp:139-143)

**Signature:** `void ImGui_ImplOpenGL3_NewFrame()`

**Purpose:** Prepare renderer for new ImGui frame (lazy device object creation)

**Called by:**
- `asciiid.cpp:6853` (per-frame editor rendering)

**Calls:** `ImGui_ImplOpenGL3_CreateDeviceObjects`

**Globals read:** `g_FontTexture`

**Globals mutated:** None (delegates to CreateDeviceObjects if needed)

**Side effects:** Creates shaders/buffers/textures on first call (lazy init)

**Notes:** Checks if font texture exists; creates device objects if missing (handles GL context loss/recreation).

---

### `ImGui_ImplOpenGL3_RenderDrawData` (imgui_impl_opengl3.cpp:148-312)

**Signature:** `void ImGui_ImplOpenGL3_RenderDrawData(ImDrawData* draw_data)`

**Purpose:** Render ImGui draw data to OpenGL framebuffer

**Called by:**
- `asciiid.cpp:10740` (editor main render loop)

**Calls:** `glGetIntegerv` (×16), `glIsEnabled` (×4), `glActiveTexture`, `glUseProgram`, `glUniform1i`, `glUniformMatrix4fv`, `glBindSampler`, `glGenVertexArrays`, `glBindVertexArray`, `glBindBuffer`, `glEnableVertexAttribArray`, `glVertexAttribPointer`, `glBufferData`, `glScissor`, `glBindTexture`, `glDrawElements`, `glDeleteVertexArrays`, (×20 state restore calls)

**Globals read:** `g_ShaderHandle`, `g_AttribLocationTex`, `g_AttribLocationProjMtx`, `g_AttribLocationPosition`, `g_AttribLocationUV`, `g_AttribLocationColor`, `g_VboHandle`, `g_ElementsHandle`

**Globals mutated:** None (saves/restores all GL state)

**Side effects:** Draws ImGui UI to active framebuffer, modifies GL state (saved/restored)

**Notes:** Saves 16+ GL state variables, sets up orthographic projection, recreates VAO each frame (for multi-context safety), renders all command lists, restores all state. Handles retina displays via FramebufferScale. Supports GL_CLIP_ORIGIN for GL 4.5 (upper-left NDC).

---

### `ImGui_ImplOpenGL3_CreateFontsTexture` (imgui_impl_opengl3.cpp:314-341)

**Signature:** `bool ImGui_ImplOpenGL3_CreateFontsTexture()`

**Purpose:** Upload ImGui font atlas to GPU texture

**Called by:**
- `ImGui_ImplOpenGL3_CreateDeviceObjects:558` (device object creation)

**Calls:** `ImGui::GetIO`, `io.Fonts->GetTexDataAsRGBA32`, `glGetIntegerv`, `glGenTextures`, `glBindTexture`, `glTexParameteri`, `glPixelStorei`, `glTexImage2D`

**Globals read:** None

**Globals mutated:** `g_FontTexture`, GL_TEXTURE_BINDING_2D (saved/restored)

**Side effects:** Allocates GPU texture, sets `io.Fonts->TexID`, modifies GL texture binding (restored)

**Notes:** Uses RGBA32 format (75% memory overhead but shader-compatible). Sets linear filtering. Saves/restores GL_TEXTURE_BINDING_2D.

---

### `ImGui_ImplOpenGL3_DestroyFontsTexture` (imgui_impl_opengl3.cpp:343-352)

**Signature:** `void ImGui_ImplOpenGL3_DestroyFontsTexture()`

**Purpose:** Delete ImGui font atlas GPU texture

**Called by:**
- `ImGui_ImplOpenGL3_DestroyDeviceObjects:587` (cleanup)

**Calls:** `ImGui::GetIO`, `glDeleteTextures`

**Globals read:** `g_FontTexture`

**Globals mutated:** `g_FontTexture` (cleared to 0), `io.Fonts->TexID` (cleared to 0)

**Side effects:** Frees GPU texture memory

**Notes:** Checks if g_FontTexture is non-zero before deletion.

---

### `CheckShader` (imgui_impl_opengl3.cpp:355-370)

**Signature:** `static bool CheckShader(GLuint handle, const char* desc)`

**Purpose:** Validate shader compilation and log errors

**Called by:**
- `ImGui_ImplOpenGL3_CreateDeviceObjects:534, 540` (vertex/fragment shader validation)

**Calls:** `glGetShaderiv`, `fprintf`, `glGetShaderInfoLog`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Prints compilation errors to stderr

**Notes:** Static helper function. Returns true if compilation succeeded, false otherwise. Logs compilation errors even if shader compiled (warnings).

---

### `CheckProgram` (imgui_impl_opengl3.cpp:373-388)

**Signature:** `static bool CheckProgram(GLuint handle, const char* desc)`

**Purpose:** Validate shader program linking and log errors

**Called by:**
- `ImGui_ImplOpenGL3_CreateDeviceObjects:546` (program link validation)

**Calls:** `glGetProgramiv`, `fprintf`, `glGetProgramInfoLog`

**Globals read:** `g_GlslVersionString`

**Globals mutated:** None

**Side effects:** Prints linking errors to stderr

**Notes:** Static helper function. Returns true if linking succeeded, false otherwise. Includes GLSL version string in error messages.

---

### `ImGui_ImplOpenGL3_CreateDeviceObjects` (imgui_impl_opengl3.cpp:390-568)

**Signature:** `bool ImGui_ImplOpenGL3_CreateDeviceObjects()`

**Purpose:** Create shaders, buffers, VAO, and font texture for ImGui rendering

**Called by:**
- `ImGui_ImplOpenGL3_NewFrame:142` (lazy initialization)

**Calls:** `glGetIntegerv`, `sscanf`, `glCreateShader`, `glShaderSource`, `glCompileShader`, `CheckShader`, `glCreateProgram`, `glAttachShader`, `glLinkProgram`, `CheckProgram`, `glGetUniformLocation`, `glGetAttribLocation`, `glGenBuffers`, `ImGui_ImplOpenGL3_CreateFontsTexture`, `glBindTexture`, `glBindBuffer`, `glBindVertexArray`

**Globals read:** `g_GlslVersionString`

**Globals mutated:** `g_VertHandle`, `g_FragHandle`, `g_ShaderHandle`, `g_AttribLocationTex`, `g_AttribLocationProjMtx`, `g_AttribLocationPosition`, `g_AttribLocationUV`, `g_AttribLocationColor`, `g_VboHandle`, `g_ElementsHandle`, GL_TEXTURE_BINDING_2D/GL_ARRAY_BUFFER_BINDING/GL_VERTEX_ARRAY_BINDING (saved/restored)

**Side effects:** Compiles shaders, links program, allocates buffers, creates font texture

**Notes:** Selects shader source based on GLSL version (120, 130, 300es, 410core). Saves/restores GL state. Creates vertex/element buffers but does not allocate storage (dynamic allocation per frame). Returns true on success.

---

### `ImGui_ImplOpenGL3_DestroyDeviceObjects` (imgui_impl_opengl3.cpp:570-588)

**Signature:** `void ImGui_ImplOpenGL3_DestroyDeviceObjects()`

**Purpose:** Delete shaders, buffers, VAO, and font texture

**Called by:**
- `ImGui_ImplOpenGL3_Shutdown:136` (cleanup)

**Calls:** `glDeleteBuffers`, `glDetachShader`, `glDeleteShader`, `glDeleteProgram`, `ImGui_ImplOpenGL3_DestroyFontsTexture`

**Globals read:** `g_VboHandle`, `g_ElementsHandle`, `g_ShaderHandle`, `g_VertHandle`, `g_FragHandle`

**Globals mutated:** All global handles (cleared to 0)

**Side effects:** Frees all GPU resources (shaders, buffers, textures)

**Notes:** Checks if handles are non-zero before deletion. Detaches shaders before deleting them.
