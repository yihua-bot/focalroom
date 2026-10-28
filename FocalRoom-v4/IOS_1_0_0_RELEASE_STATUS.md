# FocalRoom iPhone 1.0.0 归档与分发接续

日期：2026-10-28，Asia/Shanghai。当前目标是生成并验证本地 App Store 分发 IPA，不自动上传或提交审核。用户决定真实模型调用稍后验收，隐私政策网址暂缓处理。

## 工程与接入

- Release Bundle ID：`studio.focalroom.ios`；Team：`KXSFCZF4S3`；版本：`1.0.0`，构建号：`1`，设备族仅 iPhone，最低 iOS 17。
- Debug 保留 `.development` 身份，使模拟器演练和正式应用的 Keychain/数据容器分开；无新增延期平台框架。
- 已核对现有品牌 AppIcon、分发导出配置及权限文案。导出配置是 `app-store-connect` + `destination=export`，仅生成本地文件。
- 语音文案明确音频留在设备，确认保存的笔记文字会同步到工作区；隐私清单补充照片和设备标识的用途，不声明跟踪。
- AI 使用 `https://team.focroom.cc/api/v2` 同一托管网关与账号额度，供应商密钥不进入 App。服务状态读取失败可重试，未验证邮箱、无额度或服务未开放时不提交消息；账号切换后晚到结果不能写入新会话。
- 确定性模型桩仅存在于隔离验证脚本。无登录探测已确认 AI 账户与异步结果路由返回 401、设备路由返回 404；不能推断真实模型已启用、凭据有效或账户有额度。

## 已完成的验证与剩余验收

| 项目 | 当前证据 |
|---|---|
| Xcode 26.2 归档 | 成功，Bundle/Team/1.0.0(1)/iPhone family 校验通过，严格签名校验通过 |
| 归档的签名类型 | Archive 使用 Apple Development，导出阶段已重新使用分发身份签名 |
| 分发导出 | `EXPORT SUCCEEDED`，已生成 FocalRoom.ipa；正式 Bundle/Team/版本与 App Store 描述文件匹配 |
| IPA 签名核验 | Apple Distribution: liqiang song (KXSFCZF4S3)，严格签名核验通过，`get-task-allow=false`，无设备限定/企业分发标记 |
| Xcode 更新与登录 | 已更新到 27.1，用户完成账号认证，已确认目标团队管理员权限；初始 No Accounts 已解除 |
| Xcode 27 核心测试 | 总计 53 项：46 通过、7 条依赖隔离后端环境的测试跳过，0 失败 |
| Xcode 27 真实隔离后端联调 | 7 条通过，0 失败、0 跳过；不调用上游真实模型 |
| 完整 PostgreSQL 回归 | 17 个套件、128 项通过，0 失败、0 跳过；运行前后源码摘要一致 |
| Xcode 27 界面旅程 | 专用 iPhone 17 / iOS 26.3.1：2 项通过，0 失败、0 跳过，总计 268.199 秒；全部数据库和本机文件断言通过 |
| Xcode 27 新归档 | 成功，包含最新 AppModel 增量；归档前后应用源码摘要一致 |

本轮修复了验证工具的并行隔离：SwiftPM scratch 与 Xcode DerivedData 每轮独立，界面旅程使用专用模拟器。AI 丢回执故障注入改为覆盖全部 2xx 成功响应，并断言响应确实被丢弃，避免 202 路径未注入却声称恢复验证通过。未放宽正式回执或计费断言。

界面补验还发现新的照片观察回复可能停在列表可视区之外。已加入发送后收起键盘、滚动到最新消息，确认后可看到实际返回的观察结果；取消授权仍不会发图。测试通过实际系统按钮坐标关闭密码保存提示，并确认删除范围开关真的处于开启状态，保留取消最终确认和正式删除两条断言。离线索引检查逐个核对两条原生旅程实际登录的账号、服务与允许字段，拒绝串号和凭据落盘。

Docker 构建上下文另行排除归档、IPA、描述文件、签名私钥文件、私有配置与测试产物，避免发布准备文件被传入服务端构建。

## 证据与下一步

原始日志、归档签名检查、API 只读探测及源码摘要在工程 `apps/mobile-ios/test-results/release-preparation-2026-10-28/`。此前共享构建锁、测试进程被终止、故障注入未触发等失败记录保留，不能归为通过。

最终原生与后端证据：`apps/mobile-ios/test-results/focalroom-pg-tests-T03KjJ/ios-verification.json`、`iPhone-UI.xcresult` 和 `attachments/`。两条界面旅程分别覆盖完整创作/迁云/离线导出，以及账号删除的取消与最终确认；末尾再次核对账号关闭记录、会话全部失效、三份原片/预览摘要、迁云身份、两个账号离线索引及其无凭据字段。模型桩收到的授权图像观察总计恰为两次（Swift 恢复演练一次、UI 明确确认一次）；取消不发送，丢响应恢复不重传。真实模型调用数为 0。

最终 IPA：`apps/mobile-ios/DerivedData/Distribution/AppStore-1.0.0-1-Verified/FocalRoom.ipa`，2,340,112 字节，SHA256：`236f849c285cbb8d8ee66dc40711e3bda39016a3bed6b734daf79432da6c80b2`。此包已包含最新消息滚动修复；实际 IPA 解包核验了 Bundle、版本、设备族、分发证书、App Store 描述文件及 `get-task-allow=false`，描述文件有效至 2027-03-20 04:19:23 UTC；测试地址覆盖变量、模型密钥变量和演练账号密码不在应用二进制中。详细结果见 `ipa-final-verification.json`。

最终归档：`apps/mobile-ios/DerivedData/Distribution/FocalRoom-1.0.0-1-Verified.xcarchive`。旧的 Xcode 26.2、初次 Xcode 27 归档与初次导出包、失败日志均保留，不冒充最终分发产物。IPA 尚未上传到 App Store Connect，也未完成真机、真实模型或商店审核。

生产升级方案另见 `IOS_1_0_0_PRODUCTION_UPGRADE_PLAN.md`，需先核对线上实际结构、备份恢复、迁移集合和回退方式。真实模型、真机、TestFlight 和 App Store 发行分别验收。

本聊天没有提交/推送、生产迁移/部署、真实模型调用、额度分配、网站部署、App Store 上传或审核提交，也没有读取用户正式照片。
