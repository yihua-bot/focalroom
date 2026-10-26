// 本地正式命令共用持久回执队列；未确认时只重发原命令，不生成新的副作用。
export interface CommandScope {projectId:string;entityId:string}
export interface QueueMessages {pending:string;invalid:string;storage:string;busy:string;scope:string;unknown:string;missing:string}
const definitive=new Set(['REVISION_CONFLICT','VALIDATION_ERROR','INVALID_INPUT','NOT_FOUND','PERMISSION_DENIED','AUTHORITY_MISMATCH','CONFLICT','IDEMPOTENCY_CONFLICT','ASSET_NOT_FOUND']);
export class PersistentCommandQueue<Request,Receipt> {
  private busy=new Set<string>();
  constructor(private options:{prefix:string;code:string;storage:Storage;scope:(request:Request)=>CommandScope;parse:(value:unknown)=>Request;receipt:(request:Request,value:unknown)=>Receipt;send:(request:Request)=>Promise<unknown>;messages:()=>QueueMessages}){}
  pending(projectId:string):Request[] {
    const found:Request[]=[];
    for(let i=0;i<this.options.storage.length;i++){
      const key=this.options.storage.key(i);if(!key?.startsWith(this.options.prefix))continue;
      try{const value=this.options.parse(JSON.parse(this.options.storage.getItem(key)??'null'));if(this.options.scope(value).projectId===projectId)found.push(value);}catch{/* 损坏的记录不构成发送授权，也不在此处擅自删除。 */}
    }return found;
  }
  async execute(projectId:string,entityId:string,create:()=>unknown):Promise<Receipt> {
    const {storage,prefix,messages,code}=this.options;const copy=messages();
    if(storage.getItem(prefix+entityId))throw Object.assign(new Error(copy.pending),{code});
    let request:Request;try{request=this.options.parse(create());}catch(cause){throw Object.assign(new Error(copy.invalid),{cause});}
    const scope=this.options.scope(request);if(scope.projectId!==projectId||scope.entityId!==entityId)throw new Error(copy.scope);
    try{storage.setItem(prefix+entityId,JSON.stringify(request));}catch{throw new Error(copy.storage);}
    return this.submit(request);
  }
  async retry(projectId:string,entityId:string):Promise<Receipt> {
    let request:Request;try{request=this.options.parse(JSON.parse(this.options.storage.getItem(this.options.prefix+entityId)??'null'));}catch(cause){throw Object.assign(new Error(this.options.messages().missing),{cause});}
    const scope=this.options.scope(request);if(scope.projectId!==projectId||scope.entityId!==entityId)throw new Error(this.options.messages().scope);
    return this.submit(request);
  }
  private async submit(request:Request):Promise<Receipt> {
    const {entityId}=this.options.scope(request);const copy=this.options.messages();
    if(this.busy.has(entityId))throw new Error(copy.busy);this.busy.add(entityId);
    try{const receipt=this.options.receipt(request,await this.options.send(request));this.options.storage.removeItem(this.options.prefix+entityId);return receipt;}
    catch(error){if(definitive.has((error as {code?:string})?.code??'')){this.options.storage.removeItem(this.options.prefix+entityId);throw error;}throw Object.assign(new Error(copy.unknown),{cause:error,code:this.options.code});}
    finally{this.busy.delete(entityId);}
  }
}
