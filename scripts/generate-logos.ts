#!/usr/bin/env bun
/**
 * OpenWarp logo 生成器
 *
 * 流程:
 *   1. logo.png → potrace 矢量化 → assets/logo.svg(自动 trim 到 content bbox)
 *   2. sharp 读 SVG,按各尺寸光栅化为透明背景 PNG,统一外加 ~6% 内边距
 *   3. 16/32/48 三个小尺寸合成 icon.ico
 *   4. 写入 dev / preview / stable 三个 channel 的 no-padding/ 目录
 *
 * 用法:  cd scripts && bun install && bun run logos
 */

import { promises as fs } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";
import potrace from "potrace";
import pngToIco from "png-to-ico";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "..");
const SOURCE_PNG = path.join(REPO_ROOT, "logo.png");
const ASSETS_DIR = path.join(REPO_ROOT, "assets");
const SVG_OUT = path.join(ASSETS_DIR, "logo.svg");

const CHANNELS = ["dev", "preview", "stable", "local", "oss"] as const;
const PNG_SIZES = [16, 32, 48, 64, 128, 256, 512] as const;
const ICO_SIZES = [16, 32, 48] as const;
const PADDING_RATIO = 0.06; // 渲染时给 logo 留 6% 透明内边距,避免贴边

function traceToSvg(input: string): Promise<string> {
  return new Promise((resolve, reject) => {
    potrace.trace(
      input,
      {
        threshold: 128,
        turdSize: 4,
        optTolerance: 0.4,
        color: "#1a1a1a",
        background: "transparent",
      },
      (err, svg) => (err ? reject(err) : resolve(svg)),
    );
  });
}

/** potrace 输出的 SVG 自带紧贴 content 的 viewBox,直接用来渲染即可保证已裁剪边框 */
async function renderPng(svg: Buffer, size: number, padding: number): Promise<Buffer> {
  const inner = Math.max(1, size - padding * 2);
  const innerPng = await sharp(svg, { density: 384 })
    .resize(inner, inner, { fit: "contain", background: { r: 0, g: 0, b: 0, alpha: 0 } })
    .png()
    .toBuffer();
  return sharp({
    create: {
      width: size,
      height: size,
      channels: 4,
      background: { r: 0, g: 0, b: 0, alpha: 0 },
    },
  })
    .composite([{ input: innerPng, gravity: "center" }])
    .png({ compressionLevel: 9 })
    .toBuffer();
}

async function main() {
  await fs.mkdir(ASSETS_DIR, { recursive: true });

  console.log(`[1/4] potrace 矢量化 ${path.relative(REPO_ROOT, SOURCE_PNG)}`);
  const svgText = await traceToSvg(SOURCE_PNG);
  await fs.writeFile(SVG_OUT, svgText, "utf8");
  console.log(`      → ${path.relative(REPO_ROOT, SVG_OUT)}`);

  console.log(`[2/4] 渲染 PNG (${PNG_SIZES.join("/")})`);
  const svgBuf = Buffer.from(svgText, "utf8");
  const pngBySize = new Map<number, Buffer>();
  for (const size of PNG_SIZES) {
    const padding = Math.round(size * PADDING_RATIO);
    pngBySize.set(size, await renderPng(svgBuf, size, padding));
  }

  console.log(`[3/4] 合成 icon.ico (${ICO_SIZES.join("/")})`);
  const icoSrc: Buffer[] = [];
  for (const size of ICO_SIZES) {
    icoSrc.push(pngBySize.get(size)!);
  }
  const icoBuf = await pngToIco(icoSrc);

  console.log(`[4/4] 写入 ${CHANNELS.length} 个 channel`);
  for (const ch of CHANNELS) {
    const outDir = path.join(REPO_ROOT, "app", "channels", ch, "icon", "no-padding");
    await fs.mkdir(outDir, { recursive: true });
    for (const size of PNG_SIZES) {
      const file = path.join(outDir, `${size}x${size}.png`);
      await fs.writeFile(file, pngBySize.get(size)!);
    }
    await fs.writeFile(path.join(outDir, "icon.ico"), icoBuf);
    console.log(`      ✓ ${ch}`);
  }

  console.log("✅ 完成");
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
