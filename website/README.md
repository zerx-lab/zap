# OpenWarp Website

Astro 站点,源自 `design/` 下的视觉稿。

```bash
npm install
npm run dev      # http://localhost:4321
npm run build    # 输出 dist/
```

结构:

- `src/pages/index.astro` — Landing
- `src/pages/docs/[...slug].astro` — 文档动态路由
- `src/content/docs/*.mdx` — 文档内容(Content Collections)
- `src/components/` — Nav / Footer / Banner 等
- `src/styles/` — 设计 token 与全局样式
