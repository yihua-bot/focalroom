#import <Foundation/Foundation.h>
#import <NaturalLanguage/NaturalLanguage.h>
#include <stdint.h>
#include <time.h>
#include <string.h>

typedef struct {
    uint32_t dimension;
    uint32_t revision;
    uint32_t language;
    uint32_t processed;
    uint32_t status;
} BfEmbeddingInfo;

static double monotonicSeconds(void) {
    struct timespec value;
    clock_gettime(CLOCK_MONOTONIC, &value);
    return value.tv_sec + value.tv_nsec / 1e9;
}

// 输入由 Rust 校验；输出每行固定 1024 个 float，成功掩码只标记实际生成的向量。
void bf_sentence_embeddings(const char *const *texts, size_t count,
                            float *vectors, uint8_t *available,
                            uint32_t budgetMs, BfEmbeddingInfo *info,
                            const float *cachedVectors, const uint8_t *cachedAvailable,
                            const BfEmbeddingInfo *cachedInfo, uint8_t *reused) {
    *info = (BfEmbeddingInfo){0};
    if (!texts || !vectors || !available || count == 0 || count > 501 || budgetMs == 0) return;
    @autoreleasepool {
        @try {
            if (@available(macOS 11.0, *)) {
                const double deadline = monotonicSeconds() + MIN(budgetMs, 3000) / 1000.0;
                NSString *query = [NSString stringWithUTF8String:texts[0]];
                NLLanguage detected = [NLLanguageRecognizer dominantLanguageForString:query];
                NLLanguage language;
                if ([detected isEqualToString:NLLanguageEnglish]) {
                    language = NLLanguageEnglish;
                    info->language = 1;
                } else if ([detected isEqualToString:NLLanguageSimplifiedChinese] ||
                           [detected isEqualToString:NLLanguageTraditionalChinese]) {
                    language = NLLanguageSimplifiedChinese;
                    info->language = 2;
                } else {
                    info->status = 3;
                    return;
                }
                // 不请求下载资产；系统未提供模型时交回显式的不可用状态。
                NLEmbedding *embedding = [NLEmbedding sentenceEmbeddingForLanguage:language];
                if (!embedding || embedding.dimension == 0 || embedding.dimension > 1024) return;
                info->dimension = (uint32_t)embedding.dimension;
                info->revision = (uint32_t)embedding.revision;
                info->status = 1;
                for (size_t index = 0; index < count; index++) {
                    if (monotonicSeconds() >= deadline) { info->status = 2; break; }
                    // 查询每次重新计算；文档只复用同语言、revision、维数且已校验来源的缓存。
                    if (index > 0 && cachedAvailable[index] == 1 &&
                        cachedInfo->language == info->language && cachedInfo->revision == info->revision &&
                        cachedInfo->dimension == info->dimension) {
                        memcpy(vectors + index * 1024, cachedVectors + index * 1024, info->dimension * sizeof(float));
                        available[index] = 1;
                        reused[index] = 1;
                        info->processed++;
                        continue;
                    }
                    NSString *text = [NSString stringWithUTF8String:texts[index]];
                    NLLanguage documentLanguage = [NLLanguageRecognizer dominantLanguageForString:text];
                    BOOL sameLanguage = [documentLanguage isEqualToString:language] ||
                        (info->language == 2 && [documentLanguage isEqualToString:NLLanguageTraditionalChinese]);
                    if (text && text.length > 0 && text.length <= 2000 && sameLanguage) {
                        available[index] = [embedding getVector:vectors + index * 1024 forString:text] ? 1 : 0;
                    }
                    info->processed++;
                    if (index == 0 && !available[0]) { info->status = 0; break; }
                }
            }
        } @catch (NSException *exception) {
            // 不记录输入或系统异常正文；Rust 只接受成功掩码和有限数值。
            info->status = 0;
        }
    }
}
