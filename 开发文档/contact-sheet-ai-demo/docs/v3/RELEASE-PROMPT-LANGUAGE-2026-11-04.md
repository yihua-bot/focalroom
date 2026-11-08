# FocalRoom 提示词发布记录（2026-11-04）

桌面 1.5.2 已签名、公证并发布，官网下载及自动更新均指向 1.5.2：
https://focroom.cc/download/FocalRoom-1.5.2.dmg
https://focroom.cc/download/latest.json

iPhone 1.0.3（24）已上传 App Store Connect / TestFlight，上传回执 Upload succeeded。Apple 后台处理中，未提交商店审核；用户明确要求自行提审。23 为此前构建，不包含历史授权列表修复，应使用 24。

包含多语言观察来源修复、按需作品集策展提示词，以及升级后旧画布任务校验误阻塞整个任务列表的修复。历史只读展示使用冻结范围，实际执行仍严格检查当前提示词和工具契约；未修改用户数据库、历史授权、原图或正式作品。

1.5.1 初始更新包误带 macOS AppleDouble 元数据导致解包失败，已重打包并换 r2 URL；1.5.2 使用无 AppleDouble 的归档方式，包含同一修复。

来源冻结：10dc3cde97be380ec3180748d7d48ab9f67f34f4
分支：codex/prompt-language-release-20261104
工作树：/Users/yihua/Documents/film-releases/prompt-language-20261104/source
发布：https://github.com/yihua-bot/focalroom/releases/tag/v1.5.2
发行记录、安装包、日志、来源 SHA：/Users/yihua/Documents/film-releases/prompt-language-20261104

验证：只针对用户新报告的历史授权错误运行 1 条定向回归，已通过；双端发行构建及桌面公证成功。此前用户取消的最终全量回归和真实模型质量验证未追加。

注意：发行通过隔离工作树完成，原主工作树其他人的 WIP 保留。作品集接入和本次授权修复以发行分支为准；继续开发或发版前须合入该分支，避免遗漏修复。
