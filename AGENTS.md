# BabyEng 项目规范

## 项目定位

BabyEng 是面向单家庭、1～3 岁幼儿及母亲的本地优先英语启蒙 PWA。当前目标是完成 PRD v0.4 定义的 MVP release gate，不将 V1/V2 功能作为 MVP 放行条件。

## 目录约定

- `frontend/`：Vue 3 + Vite PWA，不引入第二套状态管理、路由或 HTTP 客户端。
- `backend/`：Rust + axum + sqlx/SQLite，所有用户输入必须通过参数绑定进入 SQL。
- `services/`：TTS、ASR、LLM Python 推理服务，服务不可用时必须可降级。
- `data/seed/`：已审核教学内容；只有 `review_status=published` 的内容可以下发。
- `deploy/`：Docker、Nginx、部署说明和验收脚本。
- `docs/`：PRD 与稳定设计文档；动态进度只写入 `ROADMAP.md`。

## MVP 部署边界

- MVP 是单家庭应用，不提供公网多用户账号体系。
- 默认部署只能用于可信家庭内网；远程访问必须通过 HTTPS 和额外访问控制层。
- 后端端口默认不直接暴露给宿主机，只通过同一 Compose 网络内的 Nginx 访问。
- 不允许通配 CORS；同源部署不需要 CORS，开发环境仅允许明确配置的来源。
- 录音、孩子信息、学习记录和未命中提问均视为未成年人敏感数据。

## 工程约束

- 精准修改，不顺手重构或扩展需求。
- 不新增依赖，除非标准库和现有依赖无法完成任务，并先说明原因。
- SQL 查询不得用 `format!` 拼接用户输入；动态筛选使用 `QueryBuilder` 或绑定参数。
- 数据删除必须由明确的用户确认触发；测试只能操作临时数据库和临时目录。
- API 错误保持统一 JSON 结构；前端不得静默宣称未实际完成的数据操作。

## Release gate

代码合并前至少通过：

```bash
cd backend && cargo test --all-targets
cd backend && cargo clippy --all-targets -- -D warnings
cd backend && cargo build --release
cd frontend && npm run build
python3 -m compileall -q services
docker compose -f deploy/docker-compose.yml config --quiet
```

`npm run verify` 应在本机系统 Chrome 可用时完成页面、API 和关键交互冒烟。PRD A6 的 58 条发音人工试听与音标核对、A8 的安卓 Chrome 和 iOS Safari 真机闭环必须保留人工证据；没有证据不得标记完成。

## 进度维护

每次代码、部署、测试或文档状态发生变化后同步更新根目录 `ROADMAP.md`。只有已经实现并验证的事项进入“已完成”；人工验收未执行时明确写“待验证”。
