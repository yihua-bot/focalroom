#ifndef FOCALROOM_AGENT_H
#define FOCALROOM_AGENT_H
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* 在后台队列调用。input 是 UTF-8 JSON；返回值必须且仅能释放一次。 */
char *focalroom_agent_call(const char *input);
/* 回调同步发生在本函数返回前，增量 UTF-8 指针仅在回调期间有效。 */
typedef void (*focalroom_agent_delta)(const char *text, uintptr_t context);
char *focalroom_agent_chat_stream(const char *input, focalroom_agent_delta callback, uintptr_t context);
void focalroom_agent_free(char *output);

#ifdef __cplusplus
}
#endif
#endif
