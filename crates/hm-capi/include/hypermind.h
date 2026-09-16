#ifndef HYPERMIND_H
#define HYPERMIND_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Wire-visible ABI version. The minor component is raised whenever a symbol is
 * added; the major component is raised whenever a symbol is removed or a
 * signature changes. Enumerator values below are append-only.
 */
#define HM_ABI_VERSION_MAJOR 1
#define HM_ABI_VERSION_MINOR 0

/*
 * Boundary status tier. Values 0 through 6 are faults raised at this boundary.
 * HM_STATUS_KERNEL means the kernel itself returned an error; the numeric
 * kernel error code and its stable name travel in the call surface's own
 * out-parameters rather than in this value.
 */
typedef enum HmStatus {
  HM_STATUS_OK = 0,
  HM_STATUS_NULL_POINTER = 1,
  HM_STATUS_INVALID_UTF8 = 2,
  HM_STATUS_INVALID_ARGUMENT = 3,
  HM_STATUS_HANDLE_CLOSED = 4,
  HM_STATUS_RUNTIME = 5,
  HM_STATUS_PANIC = 6,
  HM_STATUS_KERNEL = 7
} HmStatus;

/*
 * Opaque handle over one embedded actor and its runtime. Ownership rules, which
 * every function below honours:
 *
 *   1. hm_engine_open and hm_engine_clone each hand out one handle that the
 *      caller releases exactly once with hm_engine_free.
 *   2. hm_engine_close is separate from freeing. It shuts the actor down for
 *      every handle sharing the same state and is idempotent.
 *   3. A const char* handed to the caller by this library is borrowed unless a
 *      function says otherwise; a char* out-parameter is owned by the caller and
 *      released with hm_string_free.
 *   4. This library never takes ownership of a caller-supplied const char*.
 */
typedef struct HmEngine HmEngine;

/*
 * Returns the ABI version packed as (major << 16) | minor.
 */
uint32_t hm_abi_version(void);

/*
 * Returns the last boundary error recorded on the calling thread, or NULL when
 * none was recorded. The string is borrowed and stays valid only until the next
 * call into this library on the same thread.
 */
const char *hm_last_error_message(void);

/*
 * Drops the last boundary error recorded on the calling thread. Any pointer
 * previously returned by hm_last_error_message is dangling afterwards.
 */
void hm_last_error_clear(void);

/*
 * Releases a string this library handed out as an owned char*. Passing NULL is
 * a no-op. Never pass a borrowed pointer to this function.
 */
void hm_string_free(char *value);

/*
 * Opens an embedded actor and writes a fresh handle through out_engine.
 *
 * configuration_json is a NUL-terminated UTF-8 JSON object with the keys:
 *
 *   path                       state directory; the actor owns <path>/<actor>
 *   actor                      nonzero 16-bit actor identifier
 *   user_hex                   32 hexadecimal characters
 *   kek_hex                    64 hexadecimal characters
 *   projection_map_bytes       optional, defaults to 268435456
 *   providers_from_environment optional, defaults to false
 *
 * Unknown keys are rejected. The string is borrowed for the duration of the
 * call. Returns HM_STATUS_OK on success, HM_STATUS_NULL_POINTER for a NULL
 * argument, HM_STATUS_INVALID_UTF8 for a non-UTF-8 configuration,
 * HM_STATUS_INVALID_ARGUMENT for a malformed or out-of-range configuration,
 * HM_STATUS_RUNTIME when called from inside an async runtime thread or when a
 * runtime cannot be started, and HM_STATUS_KERNEL when the kernel refused to
 * open. On every failure out_engine is left untouched and the reason is
 * available from hm_last_error_message on the calling thread.
 */
HmStatus hm_engine_open(const char *configuration_json, HmEngine **out_engine);

/*
 * Returns a second handle over the same shared state, or NULL when engine is
 * NULL. The returned pointer is distinct from engine and is released with its
 * own hm_engine_free; releasing one handle leaves the others usable.
 */
HmEngine *hm_engine_clone(const HmEngine *engine);

/*
 * Shuts the actor down for every handle over the same shared state and releases
 * the actor directory, so the same path can be opened again in this process.
 * The call blocks and is idempotent: a second call also returns HM_STATUS_OK.
 * Returns HM_STATUS_NULL_POINTER for a NULL handle and HM_STATUS_RUNTIME when
 * called from inside an async runtime thread.
 */
HmStatus hm_engine_close(const HmEngine *engine);

/*
 * Releases exactly one handle; passing NULL is a no-op. The shared state, its
 * runtime included, is dropped once the last handle over it is freed. No call
 * may be in flight on the handle, and freeing must not happen from inside an
 * async runtime thread.
 */
void hm_engine_free(HmEngine *engine);

#ifdef __cplusplus
}
#endif

#endif
