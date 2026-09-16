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
#define HM_ABI_VERSION_MINOR 1

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
 * Receives the outcome of exactly one hm_engine_call. kernel_code carries the
 * numeric kernel error discriminant when status is HM_STATUS_KERNEL and -1
 * otherwise. result_json is the serialized tool envelope on success and NULL on
 * failure; error_message is the stable kernel error name on a kernel failure and
 * NULL on success. Both strings are borrowed and are valid only until this
 * callback returns; copy anything that must outlive it.
 */
typedef void (*HmResultCallback)(HmStatus status,
                                 int32_t kernel_code,
                                 const char *result_json,
                                 const char *error_message,
                                 void *user_data);

/*
 * Single-use rendezvous that turns one hm_engine_call into a blocking call for
 * embedders that cannot host a callback. A waiter carries exactly one result and
 * is released with hm_waiter_free.
 */
typedef struct HmWaiter HmWaiter;

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

/*
 * Runs one tool verb against the handle's actor and reports the result through
 * callback. verb is one of the tool names the kernel exposes and arguments_json
 * is that verb's arguments as a JSON document; both are borrowed for the
 * duration of this call and are never retained. The call returns immediately:
 * the work runs on the handle's own runtime and the callback fires on one of
 * that runtime's worker threads, never on the calling thread.
 *
 * HM_STATUS_OK is returned if and only if the callback will fire exactly once.
 * Every other return value means the callback was never invoked and never will
 * be: HM_STATUS_NULL_POINTER for a NULL engine, verb, arguments_json or
 * callback, HM_STATUS_INVALID_UTF8 for a non-UTF-8 verb or arguments, and
 * HM_STATUS_HANDLE_CLOSED for a handle already passed to hm_engine_close. The
 * reason is available from hm_last_error_message on the calling thread.
 *
 * engine and user_data must both stay valid until the callback has returned.
 */
HmStatus hm_engine_call(const HmEngine *engine, const char *verb, const char *arguments_json, HmResultCallback callback, void *user_data);

/*
 * Allocates a waiter that carries exactly one call result, or NULL when the
 * allocation fails. The caller releases it exactly once with hm_waiter_free and
 * must keep it alive until the call it is paired with has reported.
 */
HmWaiter *hm_waiter_new(void);

/*
 * Records one call result into the waiter passed as user_data and wakes the
 * thread blocked in hm_waiter_wait. The signature matches HmResultCallback
 * exactly, so it is handed to hm_engine_call as the callback with the waiter as
 * its user data. Both strings are copied into storage the waiter owns, so the
 * borrowed callback strings need not outlive the callback.
 */
void hm_waiter_callback(HmStatus status, int32_t kernel_code, const char *result_json, const char *error_message, void *user_data);

/*
 * Blocks the calling thread until the paired call reports, then hands the result
 * over and returns that call's status.
 *
 * out_kernel_code receives the numeric kernel error discriminant when the
 * returned status is HM_STATUS_KERNEL and -1 otherwise. out_result_json receives
 * an owned char* that the caller releases with hm_string_free, or NULL when the
 * call produced no envelope. Both out-parameters are written before any refusal,
 * so a refused wait leaves NULL behind.
 *
 * A waiter delivers exactly one result: a second wait returns
 * HM_STATUS_INVALID_ARGUMENT. A wait issued from inside an async runtime thread
 * returns HM_STATUS_RUNTIME instead of blocking that thread, and a NULL argument
 * returns HM_STATUS_NULL_POINTER.
 */
HmStatus hm_waiter_wait(HmWaiter *waiter, int32_t *out_kernel_code, char **out_result_json);

/*
 * Releases a waiter; passing NULL is a no-op. No call given this waiter may
 * still be in flight.
 */
void hm_waiter_free(HmWaiter *waiter);

#ifdef __cplusplus
}
#endif

#endif
