/** 一次知识检索的台账：查过哪个词、取到几条、云端答没答上来。空手而归的那几次也要看得见。 */
export type KnowledgeSearchTool =
  | 'search_photography_knowledge'
  | 'search_photographers'
  | 'get_photobook_structure'
  | 'compare_photography_series';

export interface KnowledgeSearch {
  callId: string;
  tool: KnowledgeSearchTool;
  query: string;
  returned: number;
  localMatched?: number | null;
  cloudAvailable?: boolean | null;
  cloudError?: { code: string; message: string } | null;
  error?: { code: string; message: string } | null;
  outcome: 'found' | 'no_match' | 'unavailable' | 'failed';
}