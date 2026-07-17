#ifndef SPANNERPLAN_H
#define SPANNERPLAN_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
/*
 * Prerelease, version-pinned ABI: use this header only with the native
 * library from the same release archive. ABI compatibility is not promised
 * between alpha tags.
 */


/**
 * Renders a query plan from protobuf wire bytes.
 *
 * Returns a NUL-terminated UTF-8 string that must be freed with
 * [`spannerplan_string_free`]. On render error, `*out_is_error` is set to 1
 * and the returned string holds the message. Returns NULL only when
 * `out_is_error` is NULL.
 *
 * # Safety
 *
 * `plan_wire` must point to `plan_wire_len` valid bytes when non-null.
 * `mode`, `format`, and `config_json` (if non-null) must be valid NUL-terminated
 * UTF-8. `out_is_error` must be non-null.
 */
char *spannerplan_render_tree_table_wire(const uint8_t *plan_wire,
                                         uintptr_t plan_wire_len,
                                         const char *mode,
                                         const char *format,
                                         const char *config_json,
                                         int *out_is_error);

/**
 * Renders a query plan from JSON/YAML text (QueryPlan, ResultSetStats, or
 * ResultSet shapes).
 *
 * Returns a NUL-terminated UTF-8 string that must be freed with
 * [`spannerplan_string_free`]. On render error, `*out_is_error` is set to 1
 * and the returned string holds the message. Returns NULL only when
 * `out_is_error` is NULL.
 *
 * # Safety
 *
 * `plan_json`, `mode`, and `format` must be valid NUL-terminated UTF-8.
 * `config_json`, if non-null, must likewise be valid UTF-8. `out_is_error`
 * must be non-null.
 */
char *spannerplan_render_tree_table_json(const char *plan_json,
                                         const char *mode,
                                         const char *format,
                                         const char *config_json,
                                         int *out_is_error);

/**
 * Frees a string returned by the render entry points.
 *
 * # Safety
 *
 * `s` must be NULL or a pointer previously returned by a render entry point
 * and not yet freed.
 */
void spannerplan_string_free(char *s);

#endif  /* SPANNERPLAN_H */
