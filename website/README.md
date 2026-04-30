# OpenWarp Website

OpenWarp 官网,展示 Warp 二次开发版本的核心特性:自定义 AI 提供商(BYOP)、自由模型配置、多语言界面。

> 当前项目处于早期开发,尚未发布正式版本。

## 技术栈

- [Astro 5](https://astro.build/) — 内容驱动 + 原生 i18n 路由
- [Tailwind CSS 3](https://tailwindcss.com/)
- [Framer Motion](https://www.framer.com/motion/) — 终端动画(React 岛屿)
- TypeScript

## 国际化

| 语言 | 路径 |
| --- | --- |
| 简体中文(默认) | `/` |
| English | `/en/` |

字典位于 `src/i18n/{zh-CN,en}.ts`,通过 `t(locale)` 取用。新增语种步骤:

1. 在 `src/i18n/` 下新增 `xx.ts`,实现 `Dict` 接口
2. 在 `src/i18n/index.ts` 的 `locales` 与 `dicts` 中登记
3. 在 `astro.config.mjs` 的 `i18n.locales` 中登记
4. 复制 `src/pages/index.astro` 到 `src/pages/xx/index.astro`,改 `locale`

## 本地开发

```bash
cd website
npm install
npm run dev      # http://localhost:4321
npm run build    # 静态产物在 dist/
npm run preview
```

## 目录

```
website/
├── astro.config.mjs
├── tailwind.config.mjs
├── src/
│   ├── pages/
│   │   ├── index.astro        # zh-CN
│   │   └── en/index.astro     # English
│   ├── layouts/Layout.astro
│   ├── components/
│   │   ├── Header.astro
│   │   ├── Footer.astro
│   │   ├── Hero.astro
│   │   ├── Terminal.tsx       # Framer Motion 终端动画
│   │   ├── ProviderShowcase.astro
│   │   └── Features.astro
│   ├── i18n/
│   │   ├── index.ts
│   │   ├── zh-CN.ts
│   │   └── en.ts
│   └── styles/global.css
└── public/favicon.svg
```

## 设计规范

- 主色:紫(`#9670ff`)→ 粉(`#ff5fa2`)→ 青(`#46e0ff`) 渐变
- 背景:`ink-950` (`#06070b`),配合 radial 渐变光晕
- 字体:Inter / PingFang SC,等宽用 JetBrains Mono
- 风格基调参考 warp.dev,深色科技感、玻璃质感卡片

## 许可

与上游 Warp 一致:AGPL-3.0 / MIT 双许可。
