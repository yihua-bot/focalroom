#ifndef FOCALROOM_AGENT_H
#define FOCALROOM_AGENT_H

#ifdef __cplusplus
extern "C" {
#endif

/* 在后台队列调用。input 是 UTF-8 JSON；返回值必须且仅能释放一次。 */
char *focalroom_agent_call(const char *input);
void focalroom_agent_free(char *output);

#ifdef __cplusplus
}
#endif
#endif
