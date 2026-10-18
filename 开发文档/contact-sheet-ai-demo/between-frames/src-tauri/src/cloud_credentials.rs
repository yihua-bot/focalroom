//! 云登录仅通过系统 Security API 存储；不经 shell/argv，不复用模型密钥服务名。
use crate::domain::{AppError,Result};
pub trait CredentialStore: Send+Sync {
    fn read(&self,service:&str)->Result<Option<String>>;
    fn write(&self,service:&str,value:&str)->Result<()>;
    fn delete(&self,service:&str)->Result<()>;
}
pub struct SystemCredentials;
#[cfg(target_os="macos")]
impl CredentialStore for SystemCredentials {
    fn read(&self,service:&str)->Result<Option<String>> {
        match security_framework::passwords::get_generic_password(service,"cloud-session") {
            Ok(v)=>String::from_utf8(v).map(Some).map_err(|_|AppError::new("KEYCHAIN_ERROR","登录存储格式错误")),
            Err(e) if e.code()==-25300=>Ok(None),
            Err(_)=>Err(AppError::new("KEYCHAIN_ERROR","无法读取登录钥匙串")),
        }
    }
    fn write(&self,service:&str,value:&str)->Result<()> {
        security_framework::passwords::set_generic_password(service,"cloud-session",value.as_bytes()).map_err(|_|AppError::new("KEYCHAIN_ERROR","无法保存登录钥匙串"))
    }
    fn delete(&self,service:&str)->Result<()> {
        match security_framework::passwords::delete_generic_password(service,"cloud-session") {
            Ok(())=>Ok(()), Err(e) if e.code()==-25300=>Ok(()), Err(_)=>Err(AppError::new("KEYCHAIN_ERROR","无法清除登录钥匙串")),
        }
    }
}
#[cfg(not(target_os="macos"))]
impl CredentialStore for SystemCredentials {
    fn read(&self,_:&str)->Result<Option<String>> { Err(AppError::new("UNSUPPORTED_PLATFORM","当前平台未接入凭据存储")) }
    fn write(&self,_:&str,_:&str)->Result<()> { Err(AppError::new("UNSUPPORTED_PLATFORM","当前平台未接入凭据存储")) }
    fn delete(&self,_:&str)->Result<()> { Err(AppError::new("UNSUPPORTED_PLATFORM","当前平台未接入凭据存储")) }
}
