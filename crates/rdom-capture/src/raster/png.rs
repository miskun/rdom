//! A PNG encoder, enough for the image views: 8-bit indexed color when an
//! image has at most 256 colors (a terminal frame almost always does),
//! 8-bit RGB otherwise; each row filtered by whichever of None / Sub / Up
//! gives the smallest sum (the usual heuristic); one zlib stream
//! compressed by LZ77 with fixed Huffman codes. Deterministic: the same
//! image is always the same bytes.

use std::collections::HashMap;

use super::Image;
use crate::profile::Rgb;

/// `image` as a PNG file.
pub(crate) fn encode(image: &Image) -> Vec<u8> {
    let (w, h) = (image.width, image.height);
    let (palette, raw) = match indexed(&image.pixels) {
        Some((colors, indices)) => (Some(colors), indices),
        None => (None, image.pixels.iter().flatten().copied().collect()),
    };
    let bpp = if palette.is_some() { 1 } else { 3 };
    let stride = w as usize * bpp;
    let filtered = filter(&raw, stride, h as usize, bpp);

    let mut out = Vec::new();
    out.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    // Bit depth 8; color type 3 (indexed) or 2 (RGB); deflate; adaptive
    // filtering; no interlace.
    ihdr.extend_from_slice(&[8, if palette.is_some() { 3 } else { 2 }, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    if let Some(colors) = &palette {
        let plte: Vec<u8> = colors.iter().flatten().copied().collect();
        chunk(&mut out, b"PLTE", &plte);
    }
    chunk(&mut out, b"IDAT", &zlib(&filtered));
    chunk(&mut out, b"IEND", &[]);
    out
}

/// The image's colors in first-seen order and each pixel's index into
/// them, when there are at most 256. One pass; a pixel the color of the one
/// before it (most of a terminal frame) takes its index without a lookup.
fn indexed(pixels: &[Rgb]) -> Option<(Vec<Rgb>, Vec<u8>)> {
    let mut colors: Vec<Rgb> = Vec::new();
    let mut map: HashMap<Rgb, u8> = HashMap::new();
    let mut indices = Vec::with_capacity(pixels.len());
    let mut last: Option<(Rgb, u8)> = None;
    for &p in pixels {
        let i = match last {
            Some((c, i)) if c == p => i,
            _ => {
                let i = match map.get(&p) {
                    Some(&i) => i,
                    None if colors.len() == 256 => return None,
                    None => {
                        let i = colors.len() as u8;
                        map.insert(p, i);
                        colors.push(p);
                        i
                    }
                };
                last = Some((p, i));
                i
            }
        };
        indices.push(i);
    }
    Some((colors, indices))
}

/// Each row prefixed by its filter type and filtered.
fn filter(raw: &[u8], stride: usize, rows: usize, bpp: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity((stride + 1) * rows);
    let zero = vec![0u8; stride];
    let mut cand = [vec![0u8; stride], vec![0u8; stride], vec![0u8; stride]];
    for y in 0..rows {
        let row = &raw[y * stride..(y + 1) * stride];
        let up = if y == 0 {
            &zero[..]
        } else {
            &raw[(y - 1) * stride..y * stride]
        };
        for i in 0..stride {
            let left = if i >= bpp { row[i - bpp] } else { 0 };
            cand[0][i] = row[i];
            cand[1][i] = row[i].wrapping_sub(left);
            cand[2][i] = row[i].wrapping_sub(up[i]);
        }
        // The smallest sum of the bytes as signed magnitudes; ties to the
        // lower filter type.
        let cost =
            |c: &[u8]| -> u64 { c.iter().map(|&b| u64::from((b as i8).unsigned_abs())).sum() };
        let best = (0..3).min_by_key(|&k| (cost(&cand[k]), k)).unwrap_or(0);
        out.push(best as u8);
        out.extend_from_slice(&cand[best]);
    }
    out
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc = Crc::new();
    crc.update(kind);
    crc.update(data);
    out.extend_from_slice(&crc.finish().to_be_bytes());
}

const CRC_TABLE: [u32; 256] = {
    let mut t = [0u32; 256];
    let mut n = 0;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xedb8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        t[n] = c;
        n += 1;
    }
    t
};

struct Crc(u32);

impl Crc {
    fn new() -> Self {
        Crc(0xffff_ffff)
    }
    fn update(&mut self, data: &[u8]) {
        for &b in data {
            self.0 = CRC_TABLE[((self.0 ^ u32::from(b)) & 0xff) as usize] ^ (self.0 >> 8);
        }
    }
    fn finish(&self) -> u32 {
        self.0 ^ 0xffff_ffff
    }
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for block in data.chunks(5552) {
        for &x in block {
            a += u32::from(x);
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}

/// `data` as a zlib stream: one final deflate block with fixed Huffman
/// codes.
fn zlib(data: &[u8]) -> Vec<u8> {
    let mut bits = Bits::default();
    // CMF 0x78 (deflate, 32K window), FLG 0x01 (check bits, no dict).
    bits.out.extend_from_slice(&[0x78, 0x01]);
    bits.put(1, 1); // BFINAL
    bits.put(1, 2); // BTYPE = fixed Huffman
    lz77(data, &mut bits);
    literal(&mut bits, 256);
    let mut out = bits.finish();
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

/// An LSB-first bit stream.
#[derive(Default)]
struct Bits {
    out: Vec<u8>,
    acc: u64,
    n: u32,
}

impl Bits {
    fn put(&mut self, value: u32, count: u32) {
        self.acc |= u64::from(value) << self.n;
        self.n += count;
        while self.n >= 8 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.n -= 8;
        }
    }
    /// A Huffman code, which deflate writes most significant bit first.
    fn code(&mut self, code: u32, len: u32) {
        self.put(code.reverse_bits() >> (32 - len), len);
    }
    fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            self.out.push(self.acc as u8);
        }
        self.out
    }
}

/// A literal/length symbol in the fixed code.
fn literal(bits: &mut Bits, sym: u32) {
    match sym {
        0..=143 => bits.code(0x30 + sym, 8),
        144..=255 => bits.code(0x190 + sym - 144, 9),
        256..=279 => bits.code(sym - 256, 7),
        _ => bits.code(0xc0 + sym - 280, 8),
    }
}

const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

fn matched(bits: &mut Bits, len: usize, dist: usize) {
    let l = LEN_BASE
        .iter()
        .rposition(|&b| usize::from(b) <= len)
        .expect("len ≥ 3");
    literal(bits, 257 + l as u32);
    bits.put(
        (len - usize::from(LEN_BASE[l])) as u32,
        u32::from(LEN_EXTRA[l]),
    );
    let d = DIST_BASE
        .iter()
        .rposition(|&b| usize::from(b) <= dist)
        .expect("dist ≥ 1");
    bits.code(d as u32, 5);
    bits.put(
        (dist - usize::from(DIST_BASE[d])) as u32,
        u32::from(DIST_EXTRA[d]),
    );
}

const WINDOW: usize = 32 * 1024;
const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 258;
const HASH_BITS: u32 = 15;
/// How many earlier positions a match search tries.
const CHAIN: usize = 64;

/// `data` as literals and back-references: hash chains over 3-byte
/// prefixes, greedy, the longest match among the nearest [`CHAIN`].
fn lz77(data: &[u8], bits: &mut Bits) {
    let hash = |i: usize| -> usize {
        let v = u32::from(data[i]) << 16 | u32::from(data[i + 1]) << 8 | u32::from(data[i + 2]);
        (v.wrapping_mul(0x9e37_79b1) >> (32 - HASH_BITS)) as usize
    };
    const NONE: usize = usize::MAX;
    let mut head = vec![NONE; 1 << HASH_BITS];
    let mut prev = vec![NONE; WINDOW];
    let insert = |i: usize, head: &mut Vec<usize>, prev: &mut Vec<usize>| {
        if i + MIN_MATCH <= data.len() {
            let h = hash(i);
            prev[i % WINDOW] = head[h];
            head[h] = i;
        }
    };
    let mut i = 0;
    while i < data.len() {
        let (mut best_len, mut best_dist) = (0, 0);
        if i + MIN_MATCH <= data.len() {
            let mut cand = head[hash(i)];
            let max = MAX_MATCH.min(data.len() - i);
            for _ in 0..CHAIN {
                if cand == NONE || i - cand > WINDOW - 1 || cand >= i {
                    break;
                }
                let len = data[cand..]
                    .iter()
                    .zip(&data[i..i + max])
                    .take_while(|(a, b)| a == b)
                    .count();
                if len > best_len {
                    best_len = len;
                    best_dist = i - cand;
                    if len == max {
                        break;
                    }
                }
                cand = prev[cand % WINDOW];
            }
        }
        if best_len >= MIN_MATCH {
            matched(bits, best_len, best_dist);
            for k in i..i + best_len {
                insert(k, &mut head, &mut prev);
            }
            i += best_len;
        } else {
            literal(bits, u32::from(data[i]));
            insert(i, &mut head, &mut prev);
            i += 1;
        }
    }
}
