#include <hypermind.h>

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(condition, message)                                              \
  do {                                                                         \
    if (!(condition)) {                                                        \
      fprintf(stderr, "hm_abi_smoke: %s\n", (message));                        \
      exit(1);                                                                 \
    }                                                                          \
  } while (0)

static const char CONFIGURATION_TEMPLATE[] =
    "{\"path\":\"%s\","
    "\"actor\":7,"
    "\"user_hex\":\"11111111111111111111111111111111\","
    "\"kek_hex\":\"22222222222222222222222222222222"
    "22222222222222222222222222222222\","
    "\"projection_map_bytes\":16777216}";

static const char REMEMBER_ARGUMENTS[] =
    "{\"conversation\":\"abi-c-consumer\",\"kind\":\"user\","
    "\"content\":\"The C consumer records the release region eu-central-1.\","
    "\"retention\":\"durable\"}";

static const char RECALL_ARGUMENTS[] =
    "{\"mode\":\"lexical\",\"query\":\"release region\",\"limit\":8}";

static const char REMEMBERED_TEXT[] =
    "The C consumer records the release region eu-central-1.";

static HmStatus call_blocking(const HmEngine *engine, const char *verb,
                              const char *arguments, int32_t *kernel_code,
                              char **result) {
  HmWaiter *waiter = hm_waiter_new();
  CHECK(waiter != NULL, "hm_waiter_new returned NULL");
  HmStatus issued =
      hm_engine_call(engine, verb, arguments, hm_waiter_callback, waiter);
  CHECK(issued == HM_STATUS_OK, "hm_engine_call refused the request");
  HmStatus status = hm_waiter_wait(waiter, kernel_code, result);
  hm_waiter_free(waiter);
  return status;
}

int main(int argc, char **argv) {
  CHECK(argc == 2, "usage: hm_abi_smoke <state directory>");

  uint32_t version = hm_abi_version();
  CHECK((version >> 16) == (uint32_t)HM_ABI_VERSION_MAJOR,
        "hm_abi_version reported an unexpected major version");

  char configuration[1024];
  int written = snprintf(configuration, sizeof configuration,
                         CONFIGURATION_TEMPLATE, argv[1]);
  CHECK(written > 0 && (size_t)written < sizeof configuration,
        "the configuration JSON did not fit its buffer");

  HmEngine *engine = NULL;
  HmStatus status = hm_engine_open(configuration, &engine);
  CHECK(status == HM_STATUS_OK, "hm_engine_open refused the configuration");
  CHECK(engine != NULL, "hm_engine_open left the handle NULL");

  int32_t kernel_code = 0;
  char *result = NULL;

  status = call_blocking(engine, "remember", REMEMBER_ARGUMENTS, &kernel_code,
                         &result);
  CHECK(status == HM_STATUS_OK, "remember did not return HM_STATUS_OK");
  CHECK(kernel_code == -1, "remember reported a kernel error code");
  CHECK(result != NULL, "remember delivered no envelope");
  CHECK(strstr(result, "\"ok\":true") != NULL,
        "the remember envelope is not ok");
  hm_string_free(result);
  result = NULL;

  status =
      call_blocking(engine, "recall", RECALL_ARGUMENTS, &kernel_code, &result);
  CHECK(status == HM_STATUS_OK, "recall did not return HM_STATUS_OK");
  CHECK(kernel_code == -1, "recall reported a kernel error code");
  CHECK(result != NULL, "recall delivered no envelope");
  CHECK(strstr(result, "\"ok\":true") != NULL, "the recall envelope is not ok");
  CHECK(strstr(result, REMEMBERED_TEXT) != NULL,
        "recall did not return the remembered text");
  hm_string_free(result);
  result = NULL;

  status = call_blocking(engine, "hm_not_a_verb", "{}", &kernel_code, &result);
  CHECK(status == HM_STATUS_KERNEL,
        "an unknown verb did not return HM_STATUS_KERNEL");
  CHECK(kernel_code == 47, "an unknown verb reported the wrong kernel code");
  CHECK(result == NULL, "a kernel failure delivered an envelope");
  hm_string_free(result);

  status = hm_engine_close(engine);
  CHECK(status == HM_STATUS_OK, "hm_engine_close refused the handle");
  hm_engine_free(engine);

  puts("HM_ABI_SMOKE_OK");
  return 0;
}
