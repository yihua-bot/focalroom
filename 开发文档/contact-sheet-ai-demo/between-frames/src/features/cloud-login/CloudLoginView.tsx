// 浏览器借道登录，凭据由原生后端保存；界面仅接收账户状态。
import {useCallback,useEffect,useRef,useState} from "react";
import type {CloudAuthRepository,CloudStatus} from "../../domain/cloud-auth";
import {AccountAvatar} from "../../components/shared/AccountAvatar";
import {useI18n} from "../../i18n";
const copy={
 zh:{signin:"登录云端",waiting:"等待浏览器授权…",cancel:"取消",signout:"退出登录",title:"云端账户",retry:"重试",offline:"离线",connected:"已连接",failed:"无法读取登录状态，请重试",listenFailed:"登录状态监听失败，请重试",offlineLogout:"已在本机退出；云端会话未能撤销。"},
 en:{signin:"Sign in to Cloud",waiting:"Waiting for browser authorization…",cancel:"Cancel",signout:"Sign out",title:"Cloud Account",retry:"Retry",offline:"Offline",connected:"Connected",failed:"Unable to read sign-in status. Please retry.",listenFailed:"Unable to watch sign-in changes. Please retry.",offlineLogout:"Signed out locally; the cloud session could not be revoked."},
 ja:{signin:"クラウドにログイン",waiting:"ブラウザ認証を待っています…",cancel:"キャンセル",signout:"ログアウト",title:"クラウドアカウント",retry:"再試行",offline:"オフライン",connected:"接続済み",failed:"ログイン状態を取得できません。再試行してください。",listenFailed:"ログイン状態を監視できません。再試行してください。",offlineLogout:"本機ではログアウト済みです。クラウドセッションは解除できませんでした。"},
 ko:{signin:"클라우드 로그인",waiting:"브라우저 인증 대기 중…",cancel:"취소",signout:"로그아웃",title:"클라우드 계정",retry:"다시 시도",offline:"오프라인",connected:"연결됨",failed:"로그인 상태를 읽을 수 없습니다. 다시 시도하세요.",listenFailed:"로그인 상태 변경을 확인할 수 없습니다. 다시 시도하세요.",offlineLogout:"이 기기에서 로그아웃했습니다. 클라우드 세션은 취소하지 못했습니다."},
} as const;
export function useCloudStatus(repository?:CloudAuthRepository){
 const {locale}=useI18n(),text=copy[locale];
 const [status,setStatus]=useState<CloudStatus|null>(null),[error,setError]=useState("");
 const sequence=useRef({value:0});
 const refresh=useCallback(async()=>{
  if(!repository)return;const current=++sequence.current.value;
  try{const value=await repository.status();if(current===sequence.current.value){setStatus(value);setError("");}}
  catch(cause){if(current===sequence.current.value){setError(cause instanceof Error?cause.message:text.failed);setStatus(null);}}
 },[repository,text.failed]);
 useEffect(()=>{
  let active=true;let cleanup:(()=>void)|undefined;const order=sequence.current;
  void Promise.resolve().then(()=>{if(active)return refresh();});
  if(repository)void repository.subscribe(()=>{if(active)void refresh();}).then(fn=>{if(active)cleanup=fn;else fn();}).catch(()=>{if(active)setError(text.listenFailed);});
  return()=>{active=false;++order.value;cleanup?.();};
 },[repository,refresh,text.listenFailed]);
 return {status,error,refresh};
}
function CloudAuthBody({repository,onClose}:{repository:CloudAuthRepository;onClose?:()=>void}){
 const {locale}=useI18n(),text=copy[locale];const {status,error:readError,refresh}=useCloudStatus(repository);
 const [busy,setBusy]=useState(false),[error,setError]=useState("");const pending=useRef(false);
 const perform=async(action:()=>Promise<unknown>)=>{
  if(pending.current)return;pending.current=true;setBusy(true);setError("");
  try{await action();await refresh();}catch(cause){setError(cause instanceof Error?cause.message:text.failed);}
  finally{pending.current=false;setBusy(false);}
 };
 const name=status?.user?.name.trim()||status?.user?.email||"?";
 const message=error||readError||status?.error;
 return <div className="cloud-account-panel">
  {status?.user&&<div className="cloud-account" data-testid="cloud-account">
   <span className="cloud-account-status" title={status.error??undefined} aria-label={status.authenticated?text.connected:text.offline} data-offline={!status.authenticated?"true":undefined}>●</span>
   <AccountAvatar allowEmbedded className="cloud-avatar" url={status.user.avatar} name={name}/>
   <div className="cloud-account-user" title={status.user.email}><div className="cloud-account-name">{name}</div><div className="cloud-account-email">{status.user.email}</div></div>
   <button type="button" disabled={busy} onClick={()=>void perform(async()=>{if(!await repository.logout())setError(text.offlineLogout);})}>{text.signout}</button>
  </div>}
  {status?.waiting?<div className="cloud-account cloud-account-waiting"><span role="status">{text.waiting}</span><button type="button" disabled={busy} onClick={()=>void perform(()=>repository.cancel())}>{text.cancel}</button></div>:
   status&&!status.authenticated&&<div className="cloud-account"><button type="button" disabled={busy} onClick={()=>void perform(()=>repository.open())}>{text.signin}</button></div>}
  {message&&<p role="alert" className="error-message">{message}</p>}
  {readError&&<button type="button" disabled={busy} onClick={()=>void refresh()}>{text.retry}</button>}
  {onClose&&<button type="button" disabled={busy} onClick={()=>void perform(async()=>{await repository.cancel();onClose();})}>{text.cancel}</button>}
 </div>;
}
export function CloudAccount({repository}:{repository:CloudAuthRepository}){return <CloudAuthBody repository={repository}/>;}
export function CloudLoginView({repository,onClose}:{repository:CloudAuthRepository;onClose:()=>void}){
 const {locale}=useI18n();return <section className="info-card cloud-login" aria-label={copy[locale].title}><h3>{copy[locale].title}</h3><CloudAuthBody repository={repository} onClose={onClose}/></section>;
}
