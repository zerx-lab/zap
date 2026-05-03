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
<<<<<<< HEAD
import pngToIco from "png-to-ico";
=======
>>>>>>> origin/main

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "..");
const SOURCE_PNG = path.join(REPO_ROOT, "logo.png");
const ASSETS_DIR = path.join(REPO_ROOT, "assets");
const SVG_OUT = path.join(ASSETS_DIR, "logo.svg");

const CHANNELS = ["dev", "preview", "stable", "local", "oss"] as const;
const PNG_SIZES = [16, 32, 48, 64, 128, 256, 512] as const;
<<<<<<< HEAD
const ICO_SIZES = [16, 32, 48] as const;
=======
// 与上游 warpdotdev/warp 对齐: 16/32/48/64 用 BMP,256 用 PNG 嵌入 (Vista+ 标准做法)
// 这样总大小 ~110KB 而不是全 BMP 的 370KB,避免 Windows 任务栏在窗口创建时还在解码
// 大尺寸 BMP 而显示透明占位图标的过渡。
const ICO_BMP_SIZES = [16, 32, 48, 64] as const;
const ICO_PNG_SIZES = [256] as const;
>>>>>>> origin/main
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

<<<<<<< HEAD
=======
/** 把 PNG buffer 解码成 RGBA raw,用于 ICO 中的 BMP DIB 编码 */
async function decodeRgba(png: Buffer): Promise<{ width: number; height: number; data: Buffer }> {
  const img = sharp(png).ensureAlpha();
  const { data, info } = await img.raw().toBuffer({ resolveWithObject: true });
  return { width: info.width, height: info.height, data };
}

/** ICO 中 BMP image 的编码: BITMAPINFOHEADER (height 双倍) + XOR map (BGRA, 自下而上) + AND map */
function encodeBmpDib(rgba: { width: number; height: number; data: Buffer }): Buffer {
  const { width, height, data } = rgba;
  if (width !== height) throw new Error(`ICO 要求方形,实际 ${width}x${height}`);
  const bpp = 32;
  const xorSize = width * height * 4;
  const andRowStride = Math.ceil(width / 32) * 4; // 每行 32-bit 对齐
  const andSize = andRowStride * height;
  const headerSize = 40;
  const buf = Buffer.alloc(headerSize + xorSize + andSize);

  // BITMAPINFOHEADER
  buf.writeUInt32LE(40, 0); // 头大小
  buf.writeInt32LE(width, 4);
  buf.writeInt32LE(height * 2, 8); // ICO 约定 height 双倍 (XOR + AND 合计)
  buf.writeUInt16LE(1, 12); // planes
  buf.writeUInt16LE(bpp, 14);
  buf.writeUInt32LE(0, 16); // BI_RGB 不压缩
  buf.writeUInt32LE(0, 20);
  // 24-39 全 0

  // XOR map: BGRA, 自下而上
  for (let y = 0; y < height; y++) {
    const srcRow = y * width * 4;
    const dstRow = headerSize + (height - 1 - y) * width * 4;
    for (let x = 0; x < width; x++) {
      const r = data[srcRow + x * 4];
      const g = data[srcRow + x * 4 + 1];
      const b = data[srcRow + x * 4 + 2];
      const a = data[srcRow + x * 4 + 3];
      buf[dstRow + x * 4] = b;
      buf[dstRow + x * 4 + 1] = g;
      buf[dstRow + x * 4 + 2] = r;
      buf[dstRow + x * 4 + 3] = a;
    }
  }

  // AND map: 透明像素位为 1, 不透明为 0; 自下而上, 行 32-bit 对齐
  const andOffset = headerSize + xorSize;
  for (let y = 0; y < height; y++) {
    const srcRow = y * width * 4;
    const dstRow = andOffset + (height - 1 - y) * andRowStride;
    for (let x = 0; x < width; x++) {
      const a = data[srcRow + x * 4 + 3];
      if (a === 0) {
        const byteIdx = dstRow + (x >> 3);
        const bitIdx = 7 - (x & 7);
        buf[byteIdx] |= 1 << bitIdx;
      }
    }
  }

  return buf;
}

/**
 * 自实现的 ICO 编码器 (取代 png-to-ico),与上游 warpdotdev/warp 的格式对齐:
 * 小尺寸 (16/32/48/64) 用 BMP/DIB; 大尺寸 (256) 直接嵌入 PNG 字节,Windows
 * 通过 magic bytes (89 50 4E 47) 识别。这样 ICO 文件总大小从 370KB 降到 ~110KB。
 */
async function buildIco(
  pngBySize: Map<number, Buffer>,
  bmpSizes: readonly number[],
  pngSizes: readonly number[],
): Promise<Buffer> {
  type Image = { size: number; data: Buffer; isPng: boolean };
  const images: Image[] = [];
  for (const size of bmpSizes) {
    const rgba = await decodeRgba(pngBySize.get(size)!);
    images.push({ size, data: encodeBmpDib(rgba), isPng: false });
  }
  for (const size of pngSizes) {
    images.push({ size, data: pngBySize.get(size)!, isPng: true });
  }

  const headerSize = 6;
  const dirSize = 16 * images.length;
  let dataOffset = headerSize + dirSize;

  const header = Buffer.alloc(headerSize);
  header.writeUInt16LE(0, 0); // reserved
  header.writeUInt16LE(1, 2); // type=ICO
  header.writeUInt16LE(images.length, 4);

  const dirs: Buffer[] = [];
  for (const img of images) {
    const dir = Buffer.alloc(16);
    dir.writeUInt8(img.size >= 256 ? 0 : img.size, 0); // width (256 写 0)
    dir.writeUInt8(img.size >= 256 ? 0 : img.size, 1); // height
    dir.writeUInt8(0, 2); // 调色板
    dir.writeUInt8(0, 3); // reserved
    dir.writeUInt16LE(1, 4); // planes
    dir.writeUInt16LE(32, 6); // bpp
    dir.writeUInt32LE(img.data.length, 8);
    dir.writeUInt32LE(dataOffset, 12);
    dirs.push(dir);
    dataOffset += img.data.length;
  }

  return Buffer.concat([header, ...dirs, ...images.map((i) => i.data)]);
}

>>>>>>> origin/main
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

<<<<<<< HEAD
  console.log(`[3/4] 合成 icon.ico (${ICO_SIZES.join("/")})`);
  const icoSrc: Buffer[] = [];
  for (const size of ICO_SIZES) {
    icoSrc.push(pngBySize.get(size)!);
  }
  const icoBuf = await pngToIco(icoSrc);
=======
  console.log(
    `[3/4] 合成 icon.ico (${ICO_BMP_SIZES.join("/")} BMP + ${ICO_PNG_SIZES.join("/")} PNG)`,
  );
  const icoBuf = await buildIco(pngBySize, ICO_BMP_SIZES, ICO_PNG_SIZES);
>>>>>>> origin/main

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
