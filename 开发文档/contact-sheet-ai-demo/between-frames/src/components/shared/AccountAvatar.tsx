import {useState} from "react";
import {safeAvatarUrl} from "../../../packages/contracts/src/avatar";
export function AccountAvatar({url,name,className,allowEmbedded=false}:{url?:string|null;name:string;className:string;allowEmbedded?:boolean}) {
 const embedded=allowEmbedded&&typeof url==="string"&&url.length<=1_400_000&&/^data:image\/(png|jpeg|webp);base64,[A-Za-z0-9+/]+=*$/.test(url)?url:null;
 const source=embedded??safeAvatarUrl(url);const [failed,setFailed]=useState<string|null>(null);
 if(source&&source!==failed)return <img className={className} src={source} alt="" referrerPolicy="no-referrer" onError={()=>setFailed(source)}/>;
 return <span className={`${className} ${className}-fallback`} aria-hidden="true">{Array.from(name.trim()||"?")[0].toUpperCase()}</span>;
}
