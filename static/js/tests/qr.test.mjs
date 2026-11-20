/**
 * Unit tests for the QR encoder in `static/js/qr.js`.
 *
 * Run with: `node --test static/js/tests/`
 *
 * The reference matrix below was produced by libqrencode
 * (`qrencode -8 -l M -m 0 -t ASCII "https://example.com"`); the encoder was also
 * checked module-for-module against it for every length 1–213 bytes.
 */
import test from "node:test";
import assert from "node:assert/strict";

import { qrMatrix, qrSvg, versionFor, MAX_VERSION } from "../qr.js";

const EXAMPLE_COM = [
  "1111111010101010001111111",
  "1000001001101110101000001",
  "1011101011100100101011101",
  "1011101001001110001011101",
  "1011101001010010001011101",
  "1000001011011010001000001",
  "1111111010101010101111111",
  "0000000001001011100000000",
  "1010001100111110100100101",
  "1101110011111101011101011",
  "1001011101110001010011101",
  "0100110011111100100101000",
  "0001111110101101101100001",
  "0110100011101001101100011",
  "1110111010001111111001101",
  "0000000101011010110111000",
  "1100101101000110111110010",
  "0000000010000110100010001",
  "1111111010110000101010001",
  "1000001000000101100010011",
  "1011101001011101111110011",
  "1011101001001000010010110",
  "1011101011001111110111011",
  "1000001001011011111110000",
  "1111111011100110101001001",
];

const rows = (grid) => grid.map((r) => r.map((d) => (d ? "1" : "0")).join(""));

test("matches libqrencode for https://example.com (its mask 1; any mask decodes)", () => {
  assert.deepEqual(rows(qrMatrix("https://example.com", 1)), EXAMPLE_COM);
});

test("picks the smallest version that fits (level M, byte mode)", () => {
  assert.equal(versionFor(1), 1);
  assert.equal(versionFor(14), 1);
  assert.equal(versionFor(15), 2);
  assert.equal(versionFor(213), MAX_VERSION);
  assert.equal(versionFor(214), 0);
  assert.equal(qrMatrix("x".repeat(14)).length, 21);
  assert.equal(qrMatrix("x".repeat(213)).length, 57);
});

test("returns null / empty SVG when the text is too long", () => {
  assert.equal(qrMatrix("x".repeat(214)), null);
  assert.equal(qrSvg("x".repeat(214)), "");
});

test("every mask keeps the three finder patterns intact", () => {
  for (let m = 0; m < 8; m++) {
    const g = qrMatrix("check-in", m);
    const n = g.length;
    for (const [x0, y0] of [[0, 0], [n - 7, 0], [0, n - 7]]) {
      assert.equal(g[y0][x0], true);
      assert.equal(g[y0 + 1][x0 + 1], false);
      assert.equal(g[y0 + 3][x0 + 3], true);
    }
  }
});

test("a check-in URL fits, and the SVG has a quiet zone and an escaped label", () => {
  const url = `http://192.168.1.50:3000/checkin?event=${"0".repeat(36)}&token=${"A".repeat(43)}`;
  assert.ok(qrMatrix(url));
  const svg = qrSvg(url, `Check in "Smash" <now>`);
  const n = qrMatrix(url).length + 8;
  assert.match(svg, new RegExp(`viewBox="0 0 ${n} ${n}"`));
  assert.match(svg, /aria-label="Check in &#34;Smash&#34; &#60;now&#62;"/);
  assert.match(svg, /fill="#fff"/);
});
