/** 一条知识为什么被算进这次回答：只解释挑选过程，不证明它与照片有关或它正确。 */
export type KnowledgeMatch =
  | { kind: 'local_terms'; termCount: number; matchedTerms: string[]; fields: ('title' | 'tags' | 'text')[]; matched?: number; returned?: number }
  | { kind: 'builtin_method' }
  | { kind: 'cloud_search'; relevance: number | null };