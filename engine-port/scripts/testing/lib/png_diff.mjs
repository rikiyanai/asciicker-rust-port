import fs from "node:fs/promises";
import zlib from "node:zlib";

function paethPredictor(a, b, c) {
  const p = a + b - c;
  const pa = Math.abs(p - a);
  const pb = Math.abs(p - b);
  const pc = Math.abs(p - c);

  if (pa <= pb && pa <= pc) {
    return a;
  }
  if (pb <= pc) {
    return b;
  }
  return c;
}

function bytesPerPixel(colorType) {
  if (colorType === 6) {
    return 4;
  }
  if (colorType === 2) {
    return 3;
  }
  throw new Error(`unsupported PNG color type: ${colorType}`);
}

function unfilterScanlines(raw, width, height, bpp) {
  const stride = width * bpp;
  const out = Buffer.alloc(height * stride);
  let rawOffset = 0;

  for (let y = 0; y < height; y += 1) {
    const filterType = raw[rawOffset];
    rawOffset += 1;
    const rowStart = y * stride;
    const prevRowStart = (y - 1) * stride;

    for (let x = 0; x < stride; x += 1) {
      const src = raw[rawOffset];
      rawOffset += 1;

      const left = x >= bpp ? out[rowStart + x - bpp] : 0;
      const up = y > 0 ? out[prevRowStart + x] : 0;
      const upLeft = y > 0 && x >= bpp ? out[prevRowStart + x - bpp] : 0;

      let value = src;
      if (filterType === 1) {
        value = (src + left) & 0xff;
      } else if (filterType === 2) {
        value = (src + up) & 0xff;
      } else if (filterType === 3) {
        value = (src + Math.floor((left + up) / 2)) & 0xff;
      } else if (filterType === 4) {
        value = (src + paethPredictor(left, up, upLeft)) & 0xff;
      } else if (filterType !== 0) {
        throw new Error(`unsupported PNG filter type: ${filterType}`);
      }

      out[rowStart + x] = value;
    }
  }

  return out;
}

export async function readPng(filePath) {
  const input = await fs.readFile(filePath);
  const signature = input.subarray(0, 8);
  const expected = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  if (!signature.equals(expected)) {
    throw new Error(`invalid PNG signature: ${filePath}`);
  }

  let offset = 8;
  let width = 0;
  let height = 0;
  let bitDepth = 0;
  let colorType = 0;
  const idatChunks = [];

  while (offset < input.length) {
    const length = input.readUInt32BE(offset);
    offset += 4;
    const type = input.toString("ascii", offset, offset + 4);
    offset += 4;
    const chunk = input.subarray(offset, offset + length);
    offset += length + 4;

    if (type === "IHDR") {
      width = chunk.readUInt32BE(0);
      height = chunk.readUInt32BE(4);
      bitDepth = chunk[8];
      colorType = chunk[9];
    } else if (type === "IDAT") {
      idatChunks.push(chunk);
    } else if (type === "IEND") {
      break;
    }
  }

  if (bitDepth !== 8) {
    throw new Error(`unsupported PNG bit depth: ${bitDepth}`);
  }

  const bpp = bytesPerPixel(colorType);
  const inflated = zlib.inflateSync(Buffer.concat(idatChunks));
  const unfiltered = unfilterScanlines(inflated, width, height, bpp);
  const rgba = Buffer.alloc(width * height * 4);

  if (colorType === 6) {
    unfiltered.copy(rgba);
  } else if (colorType === 2) {
    for (let i = 0, j = 0; i < unfiltered.length; i += 3, j += 4) {
      rgba[j] = unfiltered[i];
      rgba[j + 1] = unfiltered[i + 1];
      rgba[j + 2] = unfiltered[i + 2];
      rgba[j + 3] = 255;
    }
  }

  return { width, height, data: rgba };
}

function pngChunk(type, payload) {
  const typeBuffer = Buffer.from(type, "ascii");
  const header = Buffer.alloc(8);
  header.writeUInt32BE(payload.length, 0);
  typeBuffer.copy(header, 4);

  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(zlib.crc32(Buffer.concat([typeBuffer, payload])) >>> 0, 0);
  return Buffer.concat([header, payload, crc]);
}

export async function writePng(filePath, { width, height, data }) {
  const stride = width * 4;
  const raw = Buffer.alloc(height * (stride + 1));

  for (let y = 0; y < height; y += 1) {
    const dest = y * (stride + 1);
    raw[dest] = 0;
    data.copy(raw, dest + 1, y * stride, (y + 1) * stride);
  }

  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;

  const png = Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    pngChunk("IHDR", ihdr),
    pngChunk("IDAT", zlib.deflateSync(raw, { level: 9 })),
    pngChunk("IEND", Buffer.alloc(0)),
  ]);

  await fs.writeFile(filePath, png);
}

export function comparePngImages(actual, baseline) {
  if (actual.width !== baseline.width || actual.height !== baseline.height) {
    return {
      ok: false,
      reason: "size_mismatch",
      actualSize: `${actual.width}x${actual.height}`,
      baselineSize: `${baseline.width}x${baseline.height}`,
      totalPixels: actual.width * actual.height,
      mismatchedPixels: actual.width * actual.height,
      mismatchRatio: 1,
    };
  }

  const totalPixels = actual.width * actual.height;
  let mismatchedPixels = 0;
  let sumChannelDiff = 0;
  let maxChannelDiff = 0;
  const diffData = Buffer.alloc(actual.data.length);
  const firstDiffs = [];

  for (let index = 0; index < actual.data.length; index += 4) {
    let pixelMax = 0;
    let pixelSum = 0;

    for (let channel = 0; channel < 4; channel += 1) {
      const delta = Math.abs(actual.data[index + channel] - baseline.data[index + channel]);
      pixelMax = Math.max(pixelMax, delta);
      pixelSum += delta;
    }

    const pixelOffset = index / 4;
    const x = pixelOffset % actual.width;
    const y = Math.floor(pixelOffset / actual.width);

    if (pixelMax > 0) {
      mismatchedPixels += 1;
      sumChannelDiff += pixelSum;
      maxChannelDiff = Math.max(maxChannelDiff, pixelMax);
      if (firstDiffs.length < 20) {
        firstDiffs.push({ x, y, pixelMax, pixelSum });
      }
      diffData[index] = 255;
      diffData[index + 1] = Math.min(255, pixelSum);
      diffData[index + 2] = 0;
      diffData[index + 3] = 255;
    } else {
      const gray = Math.round(
        actual.data[index] * 0.3 + actual.data[index + 1] * 0.59 + actual.data[index + 2] * 0.11,
      );
      const muted = Math.max(24, Math.floor(gray * 0.2));
      diffData[index] = muted;
      diffData[index + 1] = muted;
      diffData[index + 2] = muted;
      diffData[index + 3] = 255;
    }
  }

  return {
    ok: mismatchedPixels === 0,
    reason: mismatchedPixels === 0 ? "match" : "pixel_mismatch",
    totalPixels,
    mismatchedPixels,
    mismatchRatio: totalPixels === 0 ? 0 : mismatchedPixels / totalPixels,
    maxChannelDiff,
    averageChannelDiff:
      mismatchedPixels === 0 ? 0 : Number((sumChannelDiff / (mismatchedPixels * 4)).toFixed(3)),
    firstDiffs,
    diffImage: {
      width: actual.width,
      height: actual.height,
      data: diffData,
    },
  };
}
