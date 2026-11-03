import Foundation

/// 把「已经是中文字面量」的运行时字符串过一遍本地化查表：命中返回当前语言译文，未命中原样返回。
///
/// FocalRoomCore 是独立包，包内没有翻译目录，它的错误说明和枚举标题（保留/待选/加入序列…）
/// 只能在 App 侧消费点查表；包内写 String(localized:) 会去找 Bundle.module，查不到。
@inline(__always) func localizedText(_ key: String) -> String {
    NSLocalizedString(key, bundle: .main, value: key, comment: "")
}
