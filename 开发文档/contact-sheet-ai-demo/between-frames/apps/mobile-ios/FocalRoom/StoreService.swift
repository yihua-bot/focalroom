import Foundation
import StoreKit
import UserNotifications
import FocalRoomCore

// StoreKit 2 订阅：产品从 App Store 加载；未配置产品 ID 时如实显示"暂不可用"，
// 不伪造购买成功。购买/恢复结果经服务端核验后才解锁权益（服务端 entitlement 端点待接）。
@MainActor @Observable
final class StoreService {
    private(set) var products: [Product] = []
    private(set) var purchasedProductIDs: Set<String> = []
    private(set) var loading = false
    private(set) var notice: String?

    @ObservationIgnored private var updatesTask: Task<Void, Never>?

    // App Store Connect 配置的产品 ID（订阅组 focalroom.pro）；上线前在此登记。
    static let productIDs: Set<String> = []

    init() {
        updatesTask = Task { [weak self] in
            for await update in Transaction.updates {
                if case .verified(let transaction) = update {
                    self?.purchasedProductIDs.insert(transaction.productID)
                    await transaction.finish()
                }
            }
        }
    }

    deinit {
        updatesTask?.cancel()
    }

    var available: Bool { !Self.productIDs.isEmpty }

    func loadProducts() async {
        guard !Self.productIDs.isEmpty else {
            notice = "内购产品尚未在 App Store Connect 登记。"
            return
        }
        loading = true
        defer { loading = false }
        do {
            products = try await Product.products(for: Self.productIDs)
            await refreshPurchases()
            if products.isEmpty { notice = "App Store 暂未返回产品，请稍后重试。" }
        } catch {
            notice = "无法连接 App Store，请检查网络后重试。"
        }
    }

    func purchase(_ product: Product) async {
        loading = true
        defer { loading = false }
        do {
            let result = try await product.purchase()
            switch result {
            case .success(let verification):
                if case .verified(let transaction) = verification {
                    purchasedProductIDs.insert(transaction.productID)
                    await transaction.finish()
                    notice = "购买成功；权益以服务端核验为准。"
                } else {
                    notice = "购买凭据未能通过校验，未解锁权益。"
                }
            case .userCancelled:
                break
            case .pending:
                notice = "购买等待批准（如家长同意），完成后自动解锁。"
            @unknown default:
                notice = "未知的购买结果，未解锁权益。"
            }
        } catch {
            notice = "购买未完成：\(error.localizedDescription)"
        }
    }

    func restore() async {
        loading = true
        defer { loading = false }
        do {
            try await AppStore.sync()
            await refreshPurchases()
            notice = purchasedProductIDs.isEmpty ? "没有可恢复的购买。" : "已恢复 \(purchasedProductIDs.count) 项购买。"
        } catch {
            notice = "恢复失败，请稍后重试。"
        }
    }

    private func refreshPurchases() async {
        var active: Set<String> = []
        for await entitlement in Transaction.currentEntitlements {
            if case .verified(let transaction) = entitlement, transaction.revocationDate == nil {
                active.insert(transaction.productID)
            }
        }
        purchasedProductIDs = active
    }
}

// APNs 推送注册：真机 + 推送 entitlement 后生效；模拟器不支持远程推送，如实提示。
// 令牌上报端点待服务端提供（当前后端无 /devices/:token 类端点）。
enum PushRegistration {
    static func registerForNotifications() {
        UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .badge, .sound]) { granted, error in
            guard granted, error == nil else { return }
            Task { @MainActor in
                UIApplication.shared.registerForRemoteNotifications()
            }
        }
    }

    static var remoteNotificationsSupported: Bool {
        UIApplication.shared.isRegisteredForRemoteNotifications
    }
}
