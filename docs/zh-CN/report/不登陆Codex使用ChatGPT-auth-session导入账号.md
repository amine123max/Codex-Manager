# 不登陆 Codex 使用 ChatGPT 的 /api/auth/session 在软件中的使用

本文说明一种不走 Codex 登录授权弹窗的导入方式：先在浏览器里读取 ChatGPT 当前登录会话的 `https://chatgpt.com/api/auth/session` JSON，再把整段 JSON 粘贴到 CodexManager 的“批量导入”里。

> 注意：`/api/auth/session` 返回内容里包含可用于访问账号的敏感 token。只在自己的本机环境复制粘贴，不要发到 Issue、群聊、截图或日志里。本文配图均为脱敏示意。

## 适用场景

- 浏览器里已经登录 ChatGPT。
- 不想在 CodexManager 里重新走一次 Codex 授权登录。
- 只需要把当前 ChatGPT 会话导入到软件账号池里使用。

如果你希望软件长期自动刷新账号，优先使用软件内置“登录授权”。`/api/auth/session` 页面有时只返回当前 `accessToken`，不一定包含可长期刷新的 `refreshToken`；这种账号在 token 过期后可能需要重新导入或重新登录。

## 操作步骤

1. 在已经登录 ChatGPT 的浏览器中打开：

   ```text
   https://chatgpt.com/api/auth/session
   ```

2. 页面会显示一整段 JSON。按 `Ctrl+A` 全选，再按 `Ctrl+C` 复制全部内容，不要只复制其中一小段 token。

   ![ChatGPT auth session 页面复制示意](../../../assets/images/session.png)

3. 打开 CodexManager，进入“账号管理”，点击“新增账号”。

4. 切换到“批量导入”，把刚复制的整段 JSON 粘贴到“账号数据”输入框。

   ![CodexManager 批量导入账号示意](../../../assets/images/import.png)

5. 点击“开始导入”。导入完成后，在账号列表刷新用量，确认账号状态可用。

## 支持的字段形态

批量导入会自动识别常见字段名。`/api/auth/session` 的整段 JSON 通常包含 `accessToken`，软件会按兼容格式解析；如果 JSON 中还带有 `refreshToken`、`idToken` 或账号 ID 信息，也会一并使用。

可识别的常见字段包括：

```json
{
  "accessToken": "eyJ...",
  "idToken": "eyJ...",
  "refreshToken": "rt_...",
  "accountId": "acc_..."
}
```

也支持下划线格式：

```json
{
  "access_token": "eyJ...",
  "id_token": "eyJ...",
  "refresh_token": "rt_...",
  "account_id": "acc_..."
}
```

实际复制时不需要手动改字段名，直接粘贴 `https://chatgpt.com/api/auth/session` 页面返回的完整 JSON 即可。

还支持以下常见导入格式：

- 每行一个原始 `accessToken`。
- `token`、`tokens.*`、`credentials.*`、`auth.*` 中的 token 字段。
- ChatGPT session 的 `user.id`、`user.email`、`account.id` 嵌套结构。
- sub2api 导出的 `sub2api-data` / `sub2api-bundle` 文件，其中账号位于 `accounts[].credentials`。
- BugTeam 这类只有 `access_token`、没有 `refresh_token` 的账号。

sub2api 数据包示例：

```json
{
  "type": "sub2api-data",
  "version": 1,
  "exported_at": "2026-07-14T00:00:00Z",
  "proxies": [],
  "accounts": [
    {
      "name": "BugTeam Account",
      "platform": "openai",
      "type": "oauth",
      "credentials": {
        "access_token": "eyJ...",
        "chatgpt_account_id": "acct_...",
        "chatgpt_user_id": "user_..."
      }
    }
  ]
}
```

没有 `refresh_token` 时仍可正常导入和使用当前 access token。软件会优先使用账号/用户标识建立账号身份；缺少这些标识时，会使用 access token 指纹避免重复导入同一 token。

## 常见问题

### 打开页面不是 JSON

先确认浏览器已经登录 ChatGPT。未登录、登录过期、网络被拦截时，页面可能返回登录页、错误页或空内容，需要重新登录 ChatGPT 后再刷新。

### 导入失败提示 JSON 格式不正确

通常是没有复制完整 JSON，或复制时带入了浏览器额外文本。重新打开页面，使用 `Ctrl+A`、`Ctrl+C` 复制整页内容后再粘贴。

### 导入成功但后续不可用

如果导入内容只有 `accessToken` 而没有 `refreshToken`，token 过期后可能无法自动续期。此时重新复制 `/api/auth/session` 再导入，或改用软件内置“登录授权”。

### 能不能把这段 JSON 发给别人帮忙排查

不能。它和密码、Cookie、Refresh Token 一样敏感。排查问题时只能贴脱敏后的字段结构或错误信息，不要贴真实 token。
