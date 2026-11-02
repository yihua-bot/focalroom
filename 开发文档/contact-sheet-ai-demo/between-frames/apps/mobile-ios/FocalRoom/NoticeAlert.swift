import SwiftUI

extension View {
    /// 「提示」弹窗。文本必须以参数传入：@Observable 只会为 body 中真正读过的属性重算视图，
    /// 而计算型 Binding 的 getter 在追踪上下文之外执行。直接写 Binding(get: { model.notice != nil })
    /// 会让宿主视图永远不失效，提示弹不出来，用户看到的就是「点了没反应」。
    func noticeAlert(_ message: String?, onDismiss: @escaping () -> Void) -> some View {
        alert("提示", isPresented: Binding(get: { message != nil }, set: { if !$0 { onDismiss() } })) {
            Button("知道了", role: .cancel) { onDismiss() }
        } message: {
            Text(message ?? "")
        }
    }
}
