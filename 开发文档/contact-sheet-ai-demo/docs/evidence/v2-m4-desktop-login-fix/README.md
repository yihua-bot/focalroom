# 桌面登录与账户资料证据

对应 [验收报告](../../V2-M4-DESKTOP-LOGIN-FIX-TEST-REPORT.md)，2026-10-19。`validation.json`为计数与边界；`sources.sha256.json`冻结本轮应用目录文件，`native-final-bundle.json`记录实际评审二进制摘要与协议声明。

## 最终验证

- `final-ui-253.txt`、`account-cloud-64.txt`、`final-rust-220.txt`。
- `final-typecheck.txt`、`final-lint.txt`、`final-clippy.txt`、`final-rustfmt.txt`、`final-workspace-build.txt`。
- `final-cloud-login-tests.txt`：最终文件4普通通过/2显式ignored；后加的真实PG测试未混进先前默认全量计数。
- `final-keychain.txt`、`final-real-pg-keychain.txt`：显式执行的系统存储及跨真实PG API链路。
- `final-native-build.txt`、`final-signature-info.txt`、`final-signature-check.txt`：debug评审构建，严格资源签名失败，不是正式分发签名。
- `fixture-originals.json`：2份仓库JPEG副本与前后核对的SHA；无LUMIX或用户照片。

## 原生截图

- `native-signed-in.png`、`native-restarted-account.png`、`native-cold-callback.png`：头像合并前，真实原生协议的暖/冷启动及重开，不冒充最终头像UI截图。
- `native-final-account.png`：最终侧栏首字头像、昵称、邮箱与退出。
- `native-final-sharing.png`：最终包复用原生会话共享至隔离PG成功，页面没有邮箱密码输入。
- `native-final-readonly.png`：2图迁移完成后桌面只读，账户状态仍显示。

## 历史失败保留

`account-cloud-first-failure.txt`记录本测试全库计数与其他套件竞争；后改为本测试身份范围。`account-rust-before-source-freeze.txt`记录并行构建期间rustdoc依赖失配。`account-clippy-before-source-freeze.txt`记录交叉写入后的缓存增量警告，最终在暂停其他写入后修复。旧阶段日志单列，不覆盖最终结果。

原生独立标识及隔离临时PG不会打开用户正式项目库；本聊天没有部署、生产迁移、真实邮件/Google/模型调用或Git提交推送。当前源码仍有未提交修改，不能以历史“工作树干净”描述本轮。
