import {useState} from "react";
import {safeAvatarUrl} from "../../../packages/contracts/src/avatar";
export function AccountAvatar({url,name,className}:{url?:string|null;name:string;className:string}) {
 const source=safeAvatarUrl(url);const [failed,setFailed]=useState<string|null>(null);
 if(source&&source!==failed)return <img className={className} src={source} alt="" referrerPolicy="no-referrer" onError={()=>setFailed(source)}/>;
 return <span className={`${className} ${className}-fallback`} aria-hidden="true">{Array.from(name.trim()||"?")[0].toUpperCase()}</span>;
}
