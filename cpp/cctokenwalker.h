/* FILE: ./cpp/cctokenwalker.h */

#ifdef __cplusplus
extern "C" {
#endif

typedef struct CCTokenWalker CCTokenWalker;

/* Create a new walker over a UTF‑8 buffer. */
CCTokenWalker *cctw_new(const char *code_ptr, unsigned long code_len);

/* Free a walker. Must be called exactly once per cctw_new on success. */
void cctw_free(CCTokenWalker *walker);

/* Collect declaration symbols (for CCCRATE / metrics). */
unsigned long cctw_collect_symbols(
    CCTokenWalker      *walker,
    const char        **out_names,
    unsigned long       max_names
);

/* Collect import/module names (for CCSOV / import graph). */
unsigned long cctw_collect_imports(
    CCTokenWalker      *walker,
    const char        **out_imports,
    unsigned long       max_imports
);

/* Run blacklist scan on the current buffer. Returns 1 if any marker is found, 0 otherwise. */
int cctw_scan_blacklist(
    CCTokenWalker *walker,
    const char   **out_marker,
    unsigned long *out_offset
);

#ifdef __cplusplus
}
#endif
