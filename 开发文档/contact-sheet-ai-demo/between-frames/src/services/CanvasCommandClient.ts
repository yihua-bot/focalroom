import { parseCanvasReceipt, canvasRequestSchema, type CanvasAction, type CanvasCommandRequest, type CanvasReceipt } from '../domain/canvasCommands';
import { PersistentCommandQueue } from './PersistentCommandQueue';

type Send=(request:CanvasCommandRequest)=>Promise<unknown>;
export class CanvasCommandClient {
  private queue:PersistentCommandQueue<CanvasCommandRequest,CanvasReceipt>;
  constructor(send:Send,storage:Storage,private newId:()=>string=()=>crypto.randomUUID()){
    this.queue=new PersistentCommandQueue({prefix:'focalroom.canvas.pending.v3.',code:'CANVAS_PENDING',storage,scope:r=>({projectId:r.projectId,entityId:r.boardId}),parse:value=>canvasRequestSchema.parse(value),receipt:parseCanvasReceipt,send,messages:()=>({pending:'上次画布操作尚未确认，请先点击“确认上次操作”。',invalid:'这次操作未能保存，请重新打开画布后重试。',storage:'无法保存这次画布操作，请检查可用存储空间后重试。',busy:'正在确认画布操作，请稍候。',scope:'这次操作不属于当前项目。',unknown:'尚未确认画布操作结果，请点击“确认上次操作”。',missing:'无法读取上次画布操作，请重新打开画布后检查。'})});
  }
  pending(projectId:string){return this.queue.pending(projectId);}
  execute(projectId:string,boardId:string,baseRevision:number,actions:CanvasAction[]){return this.queue.execute(projectId,boardId,()=>({schemaVersion:3,commandId:this.newId(),idempotencyKey:this.newId(),projectId,boardId,baseRevision,operations:actions.map(action=>({opId:this.newId(),action}))}));}
  retry(projectId:string,boardId:string){return this.queue.retry(projectId,boardId);}
}
