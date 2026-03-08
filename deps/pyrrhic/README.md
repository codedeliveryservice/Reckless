Pyrrhic is a partial cleanup of the [Fathom](https://github.com/jdart1/Fathom) library for probing up-tp 7-man Syzygy Tablebases. Pyrrhic attempts to reduce the burden on the global namespace introduced by Fathom, as well as provide a more robust API for decoding the results of function calls. Pyrrhic is kept fairly up-to date with changes as they are made to Stockfish's implementation, or the Fathom repository itself. 

**Integration with Pyrrhic**

To make use of Pyrrhic in your engine, you'll want to copy the contents of this repository into a directory for your engine's source. Once done, you will need to update the definitions in `tbconfig.h`. This file allows Pyrrhic to make use of your engine's utilities. Namely, you'll be defining macros for `popcnt`, `lsb`, and `poplsb`, which are all common in engines. You'll also be providing a basic interface for Bitboad attack generation for each piece type. 

**Compatibility with Pyrrhic**

Pyrrhic has a miniature chess implementation via `tbconfig.h`. Each chess program picks its own conventions. Pyrrhic uses the most common Bitboard orientation, where A1 = 0, and H8 = 63. If your engine uses a different layout, you'll need to do a form of translation when crafting calls to the Pyrrhic endpoint, as well as when reading the results back. 

**Building Alongside Pyrrhic**

The only source file that needs to be compiled is `tbprobe.c`. Any other `.c` files in the Pyrrhic project should not be explicitly compiled. The only `.h` file that should be included by a project is `tbprobe.h`. The `api.h` file is simply reference material, and should never be explicitly included. 

**Initializing Pyrrhic**

`tb_init(const char* path)` is used to initialize the tablebases from a directory. Multiple file paths at once should be seperated by a semicolon in Windows systems, and by a colon on Unix-based systems. When finished, tb_free() is called to cleanup any remaining memory. The tablebases may be initialized once again if desired. Lastly, if the path is an empty string, or `<empty>`, then tb_init() will return without doing anything. tb_init() will also set `TB_LARGEST`, `TB_NUM_WDL`, `TB_NUM_DTM`, `TB_NUM_DTZ` to pass back information about what was loaded. Those variables have `extern` definitions, and can be referenced through `tbprobe.h`

**Performing Tablebase Probes**

Refer to the [Wiki](https://github.com/AndyGrant/Pyrrhic/wiki), which lays out the 4 possible Tablebase Probing functions
