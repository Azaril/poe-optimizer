/* Independent Windows calibration host for PoB's bundled Lua runtime.
 * Build with MSVC. No optimizer Rust or adapter source is linked here.
 */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct lua_State lua_State;
typedef lua_State *(__cdecl *newstate_fn)(void);
typedef void (__cdecl *openlibs_fn)(lua_State *);
typedef int (__cdecl *loadfile_fn)(lua_State *, const char *);
typedef int (__cdecl *pcall_fn)(lua_State *, int, int, int);
typedef const char *(__cdecl *tolstring_fn)(lua_State *, int, size_t *);
typedef void (__cdecl *close_fn)(lua_State *);
typedef void (__cdecl *createtable_fn)(lua_State *, int, int);
typedef void (__cdecl *pushstring_fn)(lua_State *, const char *);
typedef void (__cdecl *rawseti_fn)(lua_State *, int, int);
typedef void (__cdecl *setfield_fn)(lua_State *, int, const char *);

static FARPROC symbol(HMODULE module, const char *name) {
    FARPROC address = GetProcAddress(module, name);
    if (!address) { fprintf(stderr, "Missing Lua API: %s\n", name); exit(2); }
    return address;
}

int main(int argc, char **argv) {
    if (argc != 7) {
        fprintf(stderr, "Usage: reference-driver lua51.dll harness.lua source-dir scratch-dir input.xml output.json\n");
        return 2;
    }
    /* Native dependencies resolve beside the explicitly selected runtime DLL. */
    SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_DEFAULT_DIRS);
    HMODULE module = LoadLibraryExA(argv[1], NULL,
        LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS);
    if (!module) { fprintf(stderr, "Cannot load Lua DLL (%lu)\n", GetLastError()); return 2; }
    newstate_fn newstate = (newstate_fn)symbol(module, "luaL_newstate");
    openlibs_fn openlibs = (openlibs_fn)symbol(module, "luaL_openlibs");
    loadfile_fn loadfile = (loadfile_fn)symbol(module, "luaL_loadfile");
    pcall_fn pcall = (pcall_fn)symbol(module, "lua_pcall");
    tolstring_fn tolstring = (tolstring_fn)symbol(module, "lua_tolstring");
    close_fn close = (close_fn)symbol(module, "lua_close");
    createtable_fn createtable = (createtable_fn)symbol(module, "lua_createtable");
    pushstring_fn pushstring = (pushstring_fn)symbol(module, "lua_pushstring");
    rawseti_fn rawseti = (rawseti_fn)symbol(module, "lua_rawseti");
    setfield_fn setfield = (setfield_fn)symbol(module, "lua_setfield");
    lua_State *state = newstate();
    if (!state) { fprintf(stderr, "Cannot create Lua state\n"); return 2; }
    openlibs(state);
    createtable(state, 4, 0);
    for (int index = 3; index < argc; ++index) {
        pushstring(state, argv[index]);
        rawseti(state, -2, index - 2);
    }
    setfield(state, -10002, "reference_args"); /* Lua 5.1 LUA_GLOBALSINDEX */
    if (!SetCurrentDirectoryA(argv[3])) {
        fprintf(stderr, "Cannot enter source directory (%lu)\n", GetLastError());
        close(state);
        return 2;
    }
    int result = loadfile(state, argv[2]);
    if (result == 0) { result = pcall(state, 0, 0, 0); }
    if (result != 0) { fprintf(stderr, "%s\n", tolstring(state, -1, NULL)); }
    close(state);
    FreeLibrary(module);
    return result == 0 ? 0 : 1;
}