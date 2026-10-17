import { z } from 'zod';
const scopedComments = z.object({ task_scope_id: z.string(), project_id: z.string(), comments: z.array(z.object({ id: z.string(), author_id: z.string(), reviewer: z.string(), body: z.string(), anchor: z.unknown(), parent_id: z.string().nullable() })).max(100), provenance: z.literal('HUMAN_PROJECT_COMMENTS'), permits_model_upload: z.literal(false), creates_author_memory: z.literal(false) });
// 只向已配置的本项目 API 获取有来源的评论；不调用模型，不把评论转为作者偏好。
export async function readScopedComments(baseUrl: string, token: string, projectId: string) {
  const url = new URL(baseUrl);
  if (!(url.protocol === 'https:' || (url.protocol === 'http:' && ['127.0.0.1', 'localhost', '[::1]'].includes(url.hostname))) || url.username || url.password || url.search || url.hash || url.pathname !== '/' || !/^[a-f0-9]{64}$/.test(token)) throw new Error('任务服务配置不合法');
  const response = await fetch(`${url.origin}/api/v2/agent/comments`, { headers: { authorization: `Task ${token}` }, redirect: 'error', signal: AbortSignal.timeout(10000) });
  if (!response.ok) throw new Error('任务评论读取被拒绝；请检查当前项目授权或重新颁发任务身份');
  const result = scopedComments.parse(await response.json());
  if (result.project_id !== projectId) throw new Error('任务身份不属于请求项目'); return result;
}
