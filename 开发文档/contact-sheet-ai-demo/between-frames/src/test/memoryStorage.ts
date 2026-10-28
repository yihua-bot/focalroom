/** 完整实现浏览器 Storage 契约，避免依赖 Node 的实验性全局存储。 */
export class MemoryStorage implements Storage {
  private data=new Map<string,string>();
  get length(){return this.data.size;}
  clear(){this.data.clear();}
  getItem(key:string){return this.data.get(key)??null;}
  setItem(key:string,value:string){this.data.set(key,value);}
  removeItem(key:string){this.data.delete(key);}
  key(index:number){return [...this.data.keys()][index]??null;}
}
