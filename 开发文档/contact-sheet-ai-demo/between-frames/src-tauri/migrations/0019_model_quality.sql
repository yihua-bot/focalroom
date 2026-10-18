-- M3 质量驱动选模：摄影师盲评记录（AGENT-QUALITY-EVAL 协议）供路由消费。
-- 只增不改；运行时只读。评测工具写入，写入方保证评审者理由与样本量。
CREATE TABLE model_quality_scores (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  provider_id TEXT NOT NULL,
  model_id TEXT NOT NULL,
  task_type TEXT NOT NULL CHECK(task_type IN ('index','vision_observe','agent_orchestrate','curation','memory_extract','review_summary')),
  score_version TEXT NOT NULL,
  intent_score INTEGER NOT NULL CHECK(intent_score BETWEEN 1 AND 5),
  narrative_score INTEGER NOT NULL CHECK(narrative_score BETWEEN 1 AND 5),
  rationale_score INTEGER NOT NULL CHECK(rationale_score BETWEEN 1 AND 5),
  fact_score INTEGER NOT NULL CHECK(fact_score BETWEEN 1 AND 5),
  review_score INTEGER NOT NULL CHECK(review_score BETWEEN 1 AND 5),
  rationale TEXT NOT NULL CHECK(length(rationale) > 0),
  sample_count INTEGER NOT NULL CHECK(sample_count >= 1),
  hard_fail INTEGER NOT NULL DEFAULT 0 CHECK(hard_fail IN (0,1)),
  evaluated_at TEXT NOT NULL
);
CREATE INDEX quality_lookup ON model_quality_scores(provider_id, model_id, task_type, evaluated_at);
