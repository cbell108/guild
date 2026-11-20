/**
 * Minimal QR code encoder (byte mode, error correction level M, versions 1–10),
 * so the host's check-in page can draw a scannable code with no dependencies.
 *
 * Follows ISO/IEC 18004. The structure mirrors Project Nayuki's reference
 * implementation (MIT): build the data codewords, add Reed–Solomon error
 * correction per block and interleave, draw the function patterns, place the
 * codewords in the zigzag order, then keep the mask with the lowest penalty.
 * Version 10 / M holds 213 bytes — far more than a check-in URL needs.
 */

/** Error-correction codewords per block, level M, indexed by version. */
const ECC_PER_BLOCK = [-1, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26];
/** Number of error-correction blocks, level M, indexed by version. */
const NUM_BLOCKS = [-1, 1, 1, 1, 2, 2, 4, 4, 4, 5, 5];
/** Largest version this encoder supports. */
export const MAX_VERSION = 10;

/** Modules available for data + ECC in a version (everything but function patterns). */
function rawDataModules(ver) {
  let r = (16 * ver + 128) * ver + 64;
  if (ver >= 2) {
    const n = Math.floor(ver / 7) + 2;
    r -= (25 * n - 10) * n - 55;
    if (ver >= 7) r -= 36;
  }
  return r;
}

/** Data codewords (bytes) a version holds at level M. */
function dataCodewords(ver) {
  return Math.floor(rawDataModules(ver) / 8) - ECC_PER_BLOCK[ver] * NUM_BLOCKS[ver];
}

/** Multiplies two GF(2^8) elements modulo x^8 + x^4 + x^3 + x^2 + 1. */
function gfMul(x, y) {
  let z = 0;
  for (let i = 7; i >= 0; i--) {
    z = (z << 1) ^ ((z >>> 7) * 0x11d);
    z ^= ((y >>> i) & 1) * x;
  }
  return z;
}

/** Reed–Solomon generator polynomial of the given degree (leading 1 omitted). */
function rsDivisor(degree) {
  const r = new Array(degree).fill(0);
  r[degree - 1] = 1;
  let root = 1;
  for (let i = 0; i < degree; i++) {
    for (let j = 0; j < r.length; j++) {
      r[j] = gfMul(r[j], root);
      if (j + 1 < r.length) r[j] ^= r[j + 1];
    }
    root = gfMul(root, 0x02);
  }
  return r;
}

/** Reed–Solomon remainder of `data` divided by `divisor`. */
function rsRemainder(data, divisor) {
  const r = divisor.map(() => 0);
  for (const b of data) {
    const factor = b ^ r.shift();
    r.push(0);
    divisor.forEach((c, i) => {
      r[i] ^= gfMul(c, factor);
    });
  }
  return r;
}

/** Smallest version (1–10) whose level-M byte-mode capacity fits `len` bytes, or 0. */
export function versionFor(len) {
  for (let v = 1; v <= MAX_VERSION; v++) {
    const countBits = v <= 9 ? 8 : 16;
    if (4 + countBits + 8 * len <= dataCodewords(v) * 8) return v;
  }
  return 0;
}

/** Data codewords for `bytes` in byte mode, padded to the version's capacity. */
function dataBytes(bytes, ver) {
  const bits = [];
  const put = (val, n) => {
    for (let i = n - 1; i >= 0; i--) bits.push((val >>> i) & 1);
  };
  put(0b0100, 4);
  put(bytes.length, ver <= 9 ? 8 : 16);
  bytes.forEach((b) => put(b, 8));
  const capBits = dataCodewords(ver) * 8;
  put(0, Math.min(4, capBits - bits.length));
  put(0, (8 - (bits.length % 8)) % 8);
  for (let pad = 0xec; bits.length < capBits; pad ^= 0xec ^ 0x11) put(pad, 8);
  const out = [];
  for (let i = 0; i < bits.length; i += 8) out.push(bits.slice(i, i + 8).reduce((a, b) => (a << 1) | b, 0));
  return out;
}

/** Splits data into blocks, appends each block's ECC and interleaves them. */
function withEcc(data, ver) {
  const numBlocks = NUM_BLOCKS[ver];
  const blockEcc = ECC_PER_BLOCK[ver];
  const rawCodewords = Math.floor(rawDataModules(ver) / 8);
  const numShort = numBlocks - (rawCodewords % numBlocks);
  const shortLen = Math.floor(rawCodewords / numBlocks);
  const divisor = rsDivisor(blockEcc);
  const blocks = [];
  for (let i = 0, k = 0; i < numBlocks; i++) {
    const dat = data.slice(k, k + shortLen - blockEcc + (i < numShort ? 0 : 1));
    k += dat.length;
    const ecc = rsRemainder(dat, divisor);
    if (i < numShort) dat.push(0);
    blocks.push(dat.concat(ecc));
  }
  const out = [];
  for (let i = 0; i < blocks[0].length; i++) {
    blocks.forEach((b, j) => {
      if (i !== shortLen - blockEcc || j >= numShort) out.push(b[i]);
    });
  }
  return out;
}

/** Centres of the alignment patterns for a version. */
function alignmentPositions(ver) {
  if (ver === 1) return [];
  const n = Math.floor(ver / 7) + 2;
  const step = Math.ceil((ver * 4 + 4) / (n * 2 - 2)) * 2;
  const out = [6];
  for (let i = 0, pos = ver * 4 + 17 - 7; i < n - 1; i++, pos -= step) out.splice(1, 0, pos);
  return out;
}

const MASKS = [
  (x, y) => (x + y) % 2 === 0,
  (x, y) => y % 2 === 0,
  (x) => x % 3 === 0,
  (x, y) => (x + y) % 3 === 0,
  (x, y) => (Math.floor(x / 3) + Math.floor(y / 2)) % 2 === 0,
  (x, y) => ((x * y) % 2) + ((x * y) % 3) === 0,
  (x, y) => (((x * y) % 2) + ((x * y) % 3)) % 2 === 0,
  (x, y) => (((x + y) % 2) + ((x * y) % 3)) % 2 === 0,
];

/** Builds the full symbol for one mask; `grid[y][x]` is true for a dark module. */
function build(ver, codewords, mask) {
  const size = ver * 4 + 17;
  const grid = Array.from({ length: size }, () => new Array(size).fill(false));
  const fixed = Array.from({ length: size }, () => new Array(size).fill(false));
  const set = (x, y, dark) => {
    grid[y][x] = dark;
    fixed[y][x] = true;
  };

  for (let i = 0; i < size; i++) {
    set(6, i, i % 2 === 0);
    set(i, 6, i % 2 === 0);
  }
  for (const [cx, cy] of [[3, 3], [size - 4, 3], [3, size - 4]]) {
    for (let dy = -4; dy <= 4; dy++) {
      for (let dx = -4; dx <= 4; dx++) {
        const x = cx + dx;
        const y = cy + dy;
        const d = Math.max(Math.abs(dx), Math.abs(dy));
        if (x >= 0 && x < size && y >= 0 && y < size) set(x, y, d !== 2 && d !== 4);
      }
    }
  }
  const align = alignmentPositions(ver);
  const last = align.length - 1;
  align.forEach((ax, i) => {
    align.forEach((ay, j) => {
      if ((i === 0 && j === 0) || (i === 0 && j === last) || (i === last && j === 0)) return;
      for (let dy = -2; dy <= 2; dy++) {
        for (let dx = -2; dx <= 2; dx++) set(ax + dx, ay + dy, Math.max(Math.abs(dx), Math.abs(dy)) !== 1);
      }
    });
  });

  // Format bits: level M (00) + mask, BCH(15,5), XOR 0x5412.
  const data = mask;
  let rem = data;
  for (let i = 0; i < 10; i++) rem = (rem << 1) ^ ((rem >>> 9) * 0x537);
  const fmt = ((data << 10) | rem) ^ 0x5412;
  const fbit = (i) => ((fmt >>> i) & 1) === 1;
  for (let i = 0; i <= 5; i++) set(8, i, fbit(i));
  set(8, 7, fbit(6));
  set(8, 8, fbit(7));
  set(7, 8, fbit(8));
  for (let i = 9; i < 15; i++) set(14 - i, 8, fbit(i));
  for (let i = 0; i < 8; i++) set(size - 1 - i, 8, fbit(i));
  for (let i = 8; i < 15; i++) set(8, size - 15 + i, fbit(i));
  set(8, size - 8, true);

  if (ver >= 7) {
    let vrem = ver;
    for (let i = 0; i < 12; i++) vrem = (vrem << 1) ^ ((vrem >>> 11) * 0x1f25);
    const vbits = (ver << 12) | vrem;
    for (let i = 0; i < 18; i++) {
      const dark = ((vbits >>> i) & 1) === 1;
      const a = size - 11 + (i % 3);
      const b = Math.floor(i / 3);
      set(a, b, dark);
      set(b, a, dark);
    }
  }

  // Codewords in the two-column zigzag, then the mask on non-function modules.
  let bit = 0;
  for (let right = size - 1; right >= 1; right -= 2) {
    if (right === 6) right = 5;
    for (let vert = 0; vert < size; vert++) {
      for (let j = 0; j < 2; j++) {
        const x = right - j;
        const upward = ((right + 1) & 2) === 0;
        const y = upward ? size - 1 - vert : vert;
        if (!fixed[y][x] && bit < codewords.length * 8) {
          grid[y][x] = ((codewords[bit >>> 3] >>> (7 - (bit & 7))) & 1) === 1;
          bit++;
        }
      }
    }
  }
  const m = MASKS[mask];
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) if (!fixed[y][x] && m(x, y)) grid[y][x] = !grid[y][x];
  }
  return grid;
}

/** ISO penalty score (N1 runs, N2 2×2 blocks, N3 finder-like patterns, N4 balance). */
export function penalty(grid) {
  const size = grid.length;
  let score = 0;
  const lines = [];
  for (let i = 0; i < size; i++) {
    lines.push(grid[i]);
    lines.push(grid.map((row) => row[i]));
  }
  const finderA = [true, false, true, true, true, false, true, false, false, false, false];
  const finderB = [...finderA].reverse();
  for (const line of lines) {
    let run = 1;
    for (let i = 1; i <= size; i++) {
      if (i < size && line[i] === line[i - 1]) run++;
      else {
        if (run >= 5) score += run - 2;
        run = 1;
      }
    }
    for (let i = 0; i + 11 <= size; i++) {
      const hit = (p) => p.every((v, k) => line[i + k] === v);
      if (hit(finderA) || hit(finderB)) score += 40;
    }
  }
  for (let y = 0; y + 1 < size; y++) {
    for (let x = 0; x + 1 < size; x++) {
      const c = grid[y][x];
      if (c === grid[y][x + 1] && c === grid[y + 1][x] && c === grid[y + 1][x + 1]) score += 3;
    }
  }
  const dark = grid.reduce((n, row) => n + row.filter(Boolean).length, 0);
  const total = size * size;
  score += Math.max(0, Math.ceil(Math.abs(dark * 20 - total * 10) / total) - 1) * 10;
  return score;
}

/**
 * Encodes `text` (UTF-8) as a QR matrix: `grid[y][x]` is true for a dark module.
 * `mask` forces a mask pattern (0–7); by default the lowest-penalty one is used.
 * Returns null when the text is too long for version 10 / M (213 bytes).
 */
export function qrMatrix(text, mask = -1) {
  const bytes = [...new TextEncoder().encode(String(text))];
  const ver = versionFor(bytes.length);
  if (!ver) return null;
  const codewords = withEcc(dataBytes(bytes, ver), ver);
  if (mask >= 0) return build(ver, codewords, mask);
  let best = null;
  let bestScore = Infinity;
  for (let m = 0; m < 8; m++) {
    const grid = build(ver, codewords, m);
    const s = penalty(grid);
    if (s < bestScore) {
      best = grid;
      bestScore = s;
    }
  }
  return best;
}

/**
 * An SVG string for `text` with a 4-module quiet zone, always dark-on-white so
 * phones can scan it in dark mode too. Returns "" when the text doesn't fit.
 */
export function qrSvg(text, label = "QR code") {
  const grid = qrMatrix(text);
  if (!grid) return "";
  const q = 4;
  const n = grid.length + q * 2;
  let d = "";
  grid.forEach((row, y) => {
    row.forEach((dark, x) => {
      if (dark) d += `M${x + q} ${y + q}h1v1h-1z`;
    });
  });
  const safe = String(label).replace(/[&<>"]/g, (c) => `&#${c.charCodeAt(0)};`);
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${n} ${n}" role="img" aria-label="${safe}" shape-rendering="crispEdges"><rect width="${n}" height="${n}" fill="#fff"/><path d="${d}" fill="#000"/></svg>`;
}
