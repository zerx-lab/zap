#!/usr/bin/env bun
import { promises as fs } from "node:fs";
import path from "node:path";
import sharp from "sharp";

const ROOT = path.resolve(import.meta.dirname, "..");
const SVG = path.join(ROOT, "website", "public", "logo-mono.svg");
const OUT = path.join(ROOT, "website", "public", "logo-mono-preview.png");

const W = 520;
const H = 260;
const TILE = 16;

// 棋盘格(证明透明)
const checker = Buffer.alloc(W * H * 4);
for (let y = 0; y < H; y++) {
  for (let x = 0; x < W; x++) {
    const i = (y * W + x) * 4;
    if (x < W / 2) {
      const c = ((x / TILE) | 0) + ((y / TILE) | 0);
      const v = c % 2 === 0 ? 220 : 180;
      checker[i] = v; checker[i + 1] = v; checker[i + 2] = v; checker[i + 3] = 255;
    } else {
      // 右半:站点真实暗底 ink-950 ≈ #0a0a0f
      checker[i] = 10; checker[i + 1] = 10; checker[i + 2] = 15; checker[i + 3] = 255;
    }
  }
}

const svg = await fs.readFile(SVG);

// 模拟 Header 容器:h-9 w-9 圆角 11px,bg-white/[0.06],border white/10
const containerSize = 144;
const inner = Math.round(containerSize * 0.78);
const logoPng = await sharp(svg, { density: 512 })
  .resize(inner, inner, { fit: "contain", background: { r: 0, g: 0, b: 0, alpha: 0 } })
  .png()
  .toBuffer();

const radius = 18;
const containerBg = `<svg xmlns="http://www.w3.org/2000/svg" width="${containerSize}" height="${containerSize}">
  <rect x="0.5" y="0.5" width="${containerSize - 1}" height="${containerSize - 1}" rx="${radius}" ry="${radius}" fill="rgba(255,255,255,0.06)" stroke="rgba(255,255,255,0.10)" stroke-width="1"/>
</svg>`;
const containerLayer = await sharp(Buffer.from(containerBg))
  .composite([{ input: logoPng, gravity: "center" }])
  .png()
  .toBuffer();

// 左侧:SVG 直接放在棋盘格上(无容器)
const leftLogo = await sharp(svg, { density: 512 })
  .resize(120, 120, { fit: "contain", background: { r: 0, g: 0, b: 0, alpha: 0 } })
  .png()
  .toBuffer();

await sharp(checker, { raw: { width: W, height: H, channels: 4 } })
  .composite([
    { input: leftLogo, top: (H - 120) / 2, left: Math.round(W / 4 - 60) },
    { input: containerLayer, top: (H - containerSize) / 2, left: Math.round((3 * W) / 4 - containerSize / 2) },
  ])
  .png()
  .toFile(OUT);

console.log("✅", path.relative(ROOT, OUT));
