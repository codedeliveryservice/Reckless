//! Decoders: LZ4 point reads (WDL) and LZMA block decoding (the distance
//! tables).
//!
//! A WDL block is dictionary-primed LZ4 held as a strided checkpoint index
//! rather than decompressed bytes: a read enters at the nearest checkpoint,
//! then chases match references back to a literal or into the dictionary.
//! Nothing is materialized.

const LZ4_MIN_MATCH: i64 = 4;
const LZ4_POINT_STRIDE_LOG2: usize = 8;
const LZ4_POINT_STRIDE: usize = 1 << LZ4_POINT_STRIDE_LOG2;

/// Where a sequence starts: its token within the block, and where its
/// literals land in the output.
#[derive(Copy, Clone, Default)]
pub struct Lz4Checkpoint {
    in_off: u32,
    out_off: u32,
}

/// The checkpoint index of a block that decompresses to `out_size` bytes: one
/// per `LZ4_POINT_STRIDE` output bytes, by a parse-only pass that moves no data.
pub fn lz4_build_point_index(src: &[u8], out_size: usize) -> Vec<Lz4Checkpoint> {
    let num_cp = out_size.div_ceil(LZ4_POINT_STRIDE);
    let mut cp = vec![Lz4Checkpoint::default(); num_cp];

    let mut ip = 0;
    let mut op = 0u32;
    let mut next_cp = 0;

    while ip < src.len() {
        let here = Lz4Checkpoint { in_off: ip as u32, out_off: op };

        let token = src[ip] as usize;
        ip += 1;
        let mut lit_len = token >> 4;
        if lit_len == 15 {
            loop {
                let s = src[ip] as usize;
                ip += 1;
                lit_len += s;
                if s != 255 {
                    break;
                }
            }
        }
        ip += lit_len;
        op += lit_len as u32;

        // A final sequence is literals only.
        if ip < src.len() {
            ip += 2; // match offset
            let mut match_len = token & 15;
            if match_len == 15 {
                loop {
                    let s = src[ip] as usize;
                    ip += 1;
                    match_len += s;
                    if s != 255 {
                        break;
                    }
                }
            }
            op += (match_len + LZ4_MIN_MATCH as usize) as u32;
        }

        while next_cp < num_cp && (next_cp << LZ4_POINT_STRIDE_LOG2) < op as usize {
            cp[next_cp] = here;
            next_cp += 1;
        }
    }
    cp
}

/// Output byte `pos` of the block `src`, primed with `dict`, through its index.
pub fn lz4_point_read(src: &[u8], dict: &[u8], index: &[Lz4Checkpoint], pos: usize) -> u8 {
    let mut target = pos as i64;

    loop {
        // Chased back past the block, into the dict.
        if target < 0 {
            return dict[(dict.len() as i64 + target) as usize];
        }

        let start = index[(target >> LZ4_POINT_STRIDE_LOG2) as usize];
        let mut ip = start.in_off as usize;
        let mut op = start.out_off as i64;

        loop {
            let token = src[ip] as i64;
            ip += 1;
            let mut lit_len = token >> 4;
            if lit_len == 15 {
                loop {
                    let s = src[ip] as i64;
                    ip += 1;
                    lit_len += s;
                    if s != 255 {
                        break;
                    }
                }
            }
            let literals = ip;
            ip += lit_len as usize;

            if target < op + lit_len {
                return src[literals + (target - op) as usize];
            }
            op += lit_len;

            let match_off = src[ip] as i64 | ((src[ip + 1] as i64) << 8);
            ip += 2;
            let mut match_len = token & 15;
            if match_len == 15 {
                loop {
                    let s = src[ip] as i64;
                    ip += 1;
                    match_len += s;
                    if s != 255 {
                        break;
                    }
                }
            }
            match_len += LZ4_MIN_MATCH;

            if target < op + match_len {
                // Fold every period the match repeats in one step, so a long
                // run at a short offset costs one hop, not one per period.
                let into = (target - op) as u32;
                let period = match_off as u32;
                target -= (into / period + 1) as i64 * match_off;
                break;
            }
            op += match_len;
        }
    }
}

/// Decodes `src`, a raw LZMA stream followed by its 5 property bytes, into
/// exactly `dst.len()` bytes, ignoring whatever the stream holds past them;
/// false if the stream is corrupt.
pub fn lzma_decompress_into(dst: &mut [u8], src: &[u8]) -> bool {
    use std::io::Read;

    const LZMA_PROPS_SIZE: usize = 5;
    let Some(split) = src.len().checked_sub(LZMA_PROPS_SIZE) else { return false };
    let (stream, props) = src.split_at(split);
    let dict_size = u32::from_le_bytes(props[1..5].try_into().unwrap());
    lzma_rust2::LzmaReader::new_with_props(stream, dst.len() as u64, props[0], dict_size, None)
        .is_ok_and(|mut reader| reader.read_exact(dst).is_ok())
}
