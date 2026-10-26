import { publicationRequestSchema,parsePublicationReceipt,type PublicationAction,type PublicationCommandRequest,type PublicationReceipt } from '@betweenframes/contracts';
import { PersistentCommandQueue } from './PersistentCommandQueue';
import { zh } from '../app/copy';
export class PublicationCommandClient {
  private queue:PersistentCommandQueue<PublicationCommandRequest,PublicationReceipt>;
  constructor(send:(request:PublicationCommandRequest)=>Promise<unknown>,storage:Storage,private newId:()=>string=()=>crypto.randomUUID()){
    this.queue=new PersistentCommandQueue({prefix:'focalroom.publication.pending.v3.',code:'PUBLICATION_PENDING',storage,scope:r=>({projectId:r.projectId,entityId:r.publicationId}),parse:value=>publicationRequestSchema.parse(value),receipt:parsePublicationReceipt,send,messages:()=>({pending:zh.publicationPending,invalid:zh.publicationInvalid,storage:zh.publicationStorageFailed,busy:zh.publicationConfirming,scope:zh.publicationScopeMismatch,unknown:zh.publicationPending,missing:zh.publicationPendingMissing})});
  }
  pending(projectId:string){return this.queue.pending(projectId);}
  execute(projectId:string,publicationId:string,baseRevision:number,actions:PublicationAction[]){return this.queue.execute(projectId,publicationId,()=>({schemaVersion:3,commandId:this.newId(),idempotencyKey:this.newId(),projectId,publicationId,baseRevision,operations:actions.map(action=>({opId:this.newId(),action}))}));}
  retry(projectId:string,publicationId:string){return this.queue.retry(projectId,publicationId);}
}
