// 服务端消息本地化：按请求 Accept-Language 在错误出口翻译用户可见文案。
// iOS/桌面/Web 客户端默认携带系统语言头；未知语言回落中文原文。
export type Locale = "zh" | "en" | "ja" | "ko";

export function negotiateLocale(header: string | undefined): Locale {
  if (!header) return "zh";
  const lower = header.toLowerCase();
  // 按质量权重粗略取第一个可识别语言；zh 系（zh-Hans/zh-CN/zh-TW）统一 zh。
  for (const part of lower.split(",")) {
    const tag = part.split(";")[0].trim();
    if (tag.startsWith("zh")) return "zh";
    if (tag.startsWith("ja")) return "ja";
    if (tag.startsWith("ko")) return "ko";
    if (tag.startsWith("en")) return "en";
  }
  return "zh";
}

type Translations = Record<string, { en: string; ja: string; ko: string }>;

export const messages: Translations = {
  "请先登录": { en: "Please sign in first", ja: "先にログインしてください", ko: "먼저 로그인하세요" },
  "会话已失效，请重新登录": { en: "Session expired, please sign in again", ja: "セッションが失効しました。再ログインしてください", ko: "세션이 만료되었습니다. 다시 로그인하세요" },
  "会话已失效": { en: "Session expired", ja: "セッションが失効しました", ko: "세션이 만료되었습니다" },
  "邮箱或密码错误": { en: "Incorrect email or password", ja: "メールアドレスまたはパスワードが正しくありません", ko: "이메일 또는 비밀번호가 올바르지 않습니다" },
  "登录尝试过多，请稍后重试": { en: "Too many sign-in attempts, try again later", ja: "ログイン試行が多すぎます。後でお試しください", ko: "로그인 시도가 너무 많습니다. 잠시 후 재시도하세요" },
  "登录请求过多，请稍后重试": { en: "Too many sign-in requests, try again later", ja: "ログインリクエストが多すぎます。後でお試しください", ko: "로그인 요청이 너무 많습니다. 잠시 후 재시도하세요" },
  "验证码错误或已过期": { en: "Verification code is wrong or expired", ja: "認証コードが誤っているか期限切れです", ko: "인증코드가 틀렸거나 만료되었습니다" },
  "验证码请求过于频繁，请稍后再试": { en: "Code requests too frequent, try later", ja: "認証コードの請求が頻繁すぎます。後でお試しください", ko: "인증코드 요청이 너무 잦습니다. 잠시 후 재시도하세요" },
  "验证尝试过多，请稍后重试": { en: "Too many verification attempts, try later", ja: "認証試行が多すぎます。後でお試しください", ko: "인증 시도가 너무 많습니다. 잠시 후 재시도하세요" },
  "邮件服务暂不可用，请稍后重试": { en: "Email service unavailable, try later", ja: "メールサービスは利用できません。後でお試しください", ko: "메일 서비스를 이용할 수 없습니다. 잠시 후 재시도하세요" },
  "AI 额度不足，请联系 FocalRoom": { en: "AI credits exhausted, contact FocalRoom", ja: "AI残高が不足しています。FocalRoomまでご連絡ください", ko: "AI 크레딧이 부족합니다. FocalRoom에 문의하세요" },
  "AI 服务暂未开放": { en: "AI service is not available yet", ja: "AIサービスは現在利用できません", ko: "AI 서비스를 아직 이용할 수 없습니다" },
  "AI 请求过于频繁，请稍后再试": { en: "AI requests too frequent, try later", ja: "AIリクエストが頻繁すぎます。後でお試しください", ko: "AI 요청이 너무 잦습니다. 잠시 후 재시도하세요" },
  "正在处理其他 AI 请求，请稍后再试": { en: "Another AI request in progress, try later", ja: "別のAIリクエスト処理中です。後でお試しください", ko: "다른 AI 요청 처리 중입니다. 잠시 후 재시도하세요" },
  "AI 请求超过大小上限": { en: "AI request exceeds size limit", ja: "AIリクエストがサイズ上限を超えています", ko: "AI 요청이 크기 제한을 초과했습니다" },
  "模型服务拒绝了请求，额度已释放": { en: "Model service rejected the request; credits released", ja: "モデルサービスに拒否されました。残高は解放済みです", ko: "모델 서비스가 요청을 거부했습니다. 크레딧이 해제되었습니다" },
  "模型结果无法确认，请勿重复提交": { en: "Model result unconfirmed; do not resubmit", ja: "モデル結果を確認できません。再送信しないでください", ko: "모델 결과를 확인할 수 없습니다. 다시 제출하지 마세요" },
  "模型结果无法确认，请稍后查询回执": { en: "Model result unconfirmed; check the receipt later", ja: "モデル結果を確認できません。後ほど受領を照会してください", ko: "모델 결과를 확인할 수 없습니다. 나중에 확인하세요" },
  "模型结果无法确认，预留额度暂未释放，请联系 FocalRoom": { en: "Model result unconfirmed; reserved credits held. Contact FocalRoom", ja: "モデル結果未確認のため予約残高は保留中です。FocalRoomまでご連絡ください", ko: "모델 결과 미확정으로 예약 크레딧이 유지 중입니다. FocalRoom에 문의하세요" },
  "模型结果或计量无法确认，请勿重复提交": { en: "Model result or metering unconfirmed; do not resubmit", ja: "モデル結果または計量を確認できません。再送信しないでください", ko: "모델 결과 또는 계량을 확인할 수 없습니다. 다시 제출하지 마세요" },
  "模型响应超过上限，请勿重复提交": { en: "Model response exceeds limit; do not resubmit", ja: "モデル応答が上限を超えています。再送信しないでください", ko: "모델 응답이 한도를 초과했습니다. 다시 제출하지 마세요" },
  "此请求已处理或结果待确认，未重复调用模型": { en: "Request already processed or pending; model not called again", ja: "このリクエストは処理済みまたは確認待ちです。モデルは再呼び出しされません", ko: "이 요청은 처리되었거나 대기 중입니다. 모델이 다시 호출되지 않습니다" },
  "请求编号已用于其他内容，未重复调用模型": { en: "Request ID already used for different content; model not called again", ja: "リクエストIDは別の内容に使用済みです。モデルは再呼び出しされません", ko: "요청 ID가 다른 내용에 사용되었습니다. 모델이 다시 호출되지 않습니다" },
  "此请求已有结果或正在处理，请稍后查询回执": { en: "Request already has a result or is in progress; check the receipt later", ja: "このリクエストには結果があるか処理中です。後ほど受領を照会してください", ko: "이 요청은 결과가 있거나 처리 중입니다. 나중에 확인하세요" },
  "请先验证邮箱": { en: "Verify your email first", ja: "先にメールアドレスを認証してください", ko: "먼저 이메일을 인증하세요" },
  "请输入有效邮箱、8–1024 字节密码与姓名": { en: "Enter a valid email, 8–1024 byte password and name", ja: "有効なメール、8〜1024バイトのパスワードと名前を入力してください", ko: "유효한 이메일, 8~1024바이트 비밀번호와 이름을 입력하세요" },
  "账户信息不完整": { en: "Incomplete account information", ja: "アカウント情報が不完全です", ko: "계정 정보가 불완전합니다" },
  "昵称不能为空": { en: "Nickname cannot be empty", ja: "ニックネームは空にできません", ko: "닉네임은 비울 수 없습니다" },
  "新密码至少 8 位，且不超过 1024 字节": { en: "New password: 8+ characters, max 1024 bytes", ja: "新しいパスワードは8文字以上、1024バイト以下で入力してください", ko: "새 비밀번호는 8자 이상 1024바이트 이하로 입력하세요" },
  "当前密码不正确": { en: "Current password is incorrect", ja: "現在のパスワードが正しくありません", ko: "현재 비밀번호가 올바르지 않습니다" },
  "查询过于频繁，请稍后再试": { en: "Queries too frequent, try later", ja: "照会が頻繁すぎます。後でお試しください", ko: "조회가 너무 잦습니다. 잠시 후 재시도하세요" },
  "消息过于频繁": { en: "Messages too frequent", ja: "メッセージが頻繁すぎます", ko: "메시지가 너무 잦습니다" },
  "重发过于频繁，请 10 分钟后再试": { en: "Resends too frequent; retry in 10 minutes", ja: "再送信が頻繁すぎます。10分後にお試しください", ko: "재발송이 너무 잦습니다. 10분 후 재시도하세요" },
  "接口不存在": { en: "Endpoint not found", ja: "エンドポイントが存在しません", ko: "엔드포인트가 없습니다" },
  "请求来源未获授权": { en: "Request origin not authorized", ja: "リクエスト元が承認されていません", ko: "요청 출처가 승인되지 않았습니다" },
  "请求来源或会话校验失败": { en: "Request origin or session check failed", ja: "リクエスト元またはセッション検証に失敗しました", ko: "요청 출처 또는 세션 검증에 실패했습니다" },
  "请求内容不合法或超过大小上限": { en: "Invalid request or over size limit", ja: "リクエストが不正かサイズ上限を超えています", ko: "요청이 잘못되었거나 크기 제한을 초과했습니다" },
  "请求不符合接口契约": { en: "Request does not match the API contract", ja: "リクエストがAPI契約に一致しません", ko: "요청이 API 계약과 일치하지 않습니다" },
  "请求嵌套过深": { en: "Request nested too deeply", ja: "リクエストのネストが深すぎます", ko: "요청 중첩이 너무 깊습니다" },
  "服务暂时不可用": { en: "Service temporarily unavailable", ja: "サービスは一時的に利用できません", ko: "서비스를 일시적으로 이용할 수 없습니다" },
  "项目不存在": { en: "Project not found", ja: "プロジェクトが見つかりません", ko: "프로젝트가 없습니다" },
  "项目不可访问": { en: "Project not accessible", ja: "プロジェクトにアクセスできません", ko: "프로젝트에 접근할 수 없습니다" },
  "照片不存在": { en: "Photo not found", ja: "写真が見つかりません", ko: "사진이 없습니다" },
  "照片不属于此项目": { en: "Photo does not belong to this project", ja: "写真はこのプロジェクトに属していません", ko: "사진이 이 프로젝트에 속하지 않습니다" },
  "预览不存在": { en: "Preview not found", ja: "プレビューが見つかりません", ko: "미리보기가 없습니다" },
  "预览不可用": { en: "Preview unavailable", ja: "プレビューを利用できません", ko: "미리보기를 이용할 수 없습니다" },
  "任务不存在": { en: "Task not found", ja: "タスクが見つかりません", ko: "작업이 없습니다" },
  "设备不存在": { en: "Device not found", ja: "デバイスが見つかりません", ko: "기기를 찾을 수 없습니다" },
  "工作区不存在": { en: "Workspace not found", ja: "ワークスペースが見つかりません", ko: "워크스페이스가 없습니다" },
  "画布不存在": { en: "Canvas not found", ja: "キャンバスが見つかりません", ko: "캔버스가 없습니다" },
  "提案不存在": { en: "Proposal not found", ja: "提案が見つかりません", ko: "제안이 없습니다" },
  "评论不能为空": { en: "Comment cannot be empty", ja: "コメントは空にできません", ko: "댓글은 비울 수 없습니다" },
  "只读角色不能评论": { en: "Read-only role cannot comment", ja: "読み取り専用ロールはコメントできません", ko: "읽기 전용 역할은 댓글을 달 수 없습니다" },
  "只读角色不能写草稿": { en: "Read-only role cannot write drafts", ja: "読み取り専用ロールは草案を書けません", ko: "읽기 전용 역할은 초안을 쓸 수 없습니다" },
  "权限或项目版本已变化，请刷新后重新提交": { en: "Permissions or version changed; refresh and resubmit", ja: "権限またはバージョンが変化しました。更新して再送信してください", ko: "권한 또는 버전이 변경되었습니다. 새로고침 후 다시 제출하세요" },
  "画布修订已变化，请刷新后重新提交": { en: "Canvas revision changed; refresh and resubmit", ja: "キャンバスのリビジョンが変化しました。更新して再送信してください", ko: "캔버스 리비전이 변경되었습니다. 새로고침 후 다시 제출하세요" },
  "画布已冻结": { en: "Canvas is frozen", ja: "キャンバスは凍結されています", ko: "캔버스가 고정되었습니다" },
  "云端已提交，不能恢复本地写入；请恢复迁移确认": { en: "Cloud copy committed; local writes stay frozen. Resume the migration to confirm", ja: "クラウド提出済みのためローカル書き込みは復元できません。移行を再開して確認してください", ko: "클라우드 제출 완료로 로컬 쓰기를 복원할 수 없습니다. 이동을 재개해 확인하세요" },
  "迁移尚未完成": { en: "Migration not finished", ja: "移行が未完了です", ko: "이동이 완료되지 않았습니다" },
  "迁云未完成": { en: "Migration incomplete", ja: "クラウド移行が未完了です", ko: "클라우드 이동이 미완료입니다" },
  "迁移校验和不匹配": { en: "Migration checksum mismatch", ja: "移行チェックサムが一致しません", ko: "이동 체크섬이 일치하지 않습니다" },
  "邀请已使用、过期、撤销或与登录邮箱不匹配": { en: "Invite used, expired, revoked, or email mismatch", ja: "招待は使用済み・期限切れ・取り消し、またはメール不一致です", ko: "초대가 사용됐거나 만료·해지되었거나 이메일이 일치하지 않습니다" },
  "只有工作区管理员可邀请成员": { en: "Only workspace admins can invite members", ja: "ワークスペース管理者のみ招待できます", ko: "워크스페이스 관리자만 초대할 수 있습니다" },
  "只有工作区管理员可管理成员": { en: "Only workspace admins can manage members", ja: "ワークスペース管理者のみメンバー管理できます", ko: "워크스페이스 관리자만 멤버를 관리할 수 있습니다" },
  "头像格式不支持": { en: "Unsupported avatar format", ja: "対応していないアバター形式です", ko: "지원하지 않는 아바타 형식입니다" },
  "上传必须为 JPEG 派生预览": { en: "Uploads must be derived JPEG previews", ja: "アップロードは派生JPEGプレビューのみです", ko: "업로드는 파생 JPEG 미리보기만 가능합니다" },
  "仅接受长边不超过 2048px 的单张 JPEG 预览": { en: "Only single JPEG previews up to 2048px long edge", ja: "長辺2048px以下の単一枚JPEGプレビューのみ受け付けます", ko: "장변 2048px 이하의 단일 JPEG 미리보기만 허용됩니다" },
  "照片预览不得超过 1400px": { en: "Photo preview must not exceed 1400px", ja: "写真プレビューは1400pxを超えられません", ko: "사진 미리보기는 1400px를 초과할 수 없습니다" },
  "照片预览格式不合法": { en: "Invalid photo preview format", ja: "写真プレビューの形式が不正です", ko: "사진 미리보기 형식이 잘못되었습니다" },
  "预览损坏或编码不受支持，请重新导出 JPEG 预览": { en: "Preview corrupt or unsupported encoding; re-export a JPEG preview", ja: "プレビューが破損または未対応です。JPEGプレビューを再書き出しください", ko: "미리보기가 손상되었거나 지원되지 않습니다. JPEG 미리보기를 다시 내보내세요" },
  "预览完整性检查失败": { en: "Preview integrity check failed", ja: "プレビュー整合性チェックに失敗しました", ko: "미리보기 무결성 검사에 실패했습니다" },
  "预览尚未全部上传，可安全重试": { en: "Previews not fully uploaded; safe to retry", ja: "プレビュー未アップロード分あり。安全に再試行できます", ko: "미리보기가 일부 업로드되지 않았습니다. 안전하게 재시도할 수 있습니다" },
};

/// 出口翻译：命中词表则按语言替换，否则原样返回（新增文案回落中文，不阻塞）。
export function localizeMessage(message: string, locale: Locale): string {
  if (locale === "zh") return message;
  const entry = messages[message];
  return entry ? entry[locale] : message;
}
