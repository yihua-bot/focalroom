-- 账户头像：Google 登录持久化 avatar_url；验证码账户为 NULL（前端用首字占位）。
ALTER TABLE fr_users ADD COLUMN avatar_url text;
