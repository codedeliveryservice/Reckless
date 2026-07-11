//! Converts the f32 tail of the network into the engine's integer format,
//! so the engine can embed the result verbatim. Scales must match src/nnue.
//!
//! All scales are powers of two, so the rounding is done on the raw f32
//! bits. This keeps the build script free of float instructions, which
//! matters when RUSTFLAGS carries a target-cpu the build host lacks.

use std::path::Path;

const L1_SIZE: usize = 768;
const L2_SIZE: usize = 16;
const L3_SIZE: usize = 32;
const INPUT_BUCKETS: usize = 10;
const OUTPUT_BUCKETS: usize = 8;
const THREAT_ROWS: usize = 66864;

const TAIL_SHIFT: i64 = 14;
const TAIL_ACT_SHIFT: i64 = 12;
const TAIL_ACT_QUANT: i64 = 1 << TAIL_ACT_SHIFT;
const L1_SHIFT: i64 = 30;

const HEAD_SIZE: usize =
    THREAT_ROWS * L1_SIZE + INPUT_BUCKETS * 768 * L1_SIZE * 2 + L1_SIZE * 2 + OUTPUT_BUCKETS * L2_SIZE * L1_SIZE;

const INPUT_SIZE: usize = HEAD_SIZE
    + OUTPUT_BUCKETS * (L2_SIZE + L2_SIZE * L3_SIZE + L3_SIZE + L3_SIZE) * 4
    + (OUTPUT_BUCKETS * 4).next_multiple_of(64);

// round(value * 2^shift) with ties away from zero, straight from the f32 bits.
fn quantize(bits: u32, shift: i64) -> i64 {
    let exponent = (bits >> 23) as i64 & 0xFF;
    let mantissa = (bits & 0x7F_FFFF) as i64;

    assert!(exponent != 0xFF, "non-finite tail parameter");
    let (mantissa, exponent) = if exponent == 0 { (mantissa, 1) } else { (mantissa | 0x80_0000, exponent) };

    // value = sign * mantissa * 2^(exponent - 150)
    let power = exponent - 150 + shift;
    assert!(mantissa == 0 || power < 39, "tail parameter out of range");

    let magnitude = if power >= 0 {
        mantissa << power
    } else if power >= -24 {
        (mantissa + (1 << (-power - 1))) >> -power
    } else {
        0
    };

    if bits >> 31 != 0 { -magnitude } else { magnitude }
}

pub fn quantize_model(input: &Path, output: &Path) {
    let data = std::fs::read(input).unwrap_or_else(|e| panic!("failed to read {}: {e}", input.display()));
    assert!(data.len() == INPUT_SIZE, "unexpected network size {} (want {INPUT_SIZE})", data.len());

    let mut floats = data[HEAD_SIZE..].chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap()));
    let mut next = move || floats.next().unwrap();

    let l1_biases: Vec<Vec<u32>> = (0..OUTPUT_BUCKETS).map(|_| (0..L2_SIZE).map(|_| next()).collect()).collect();
    let l2_weights: Vec<Vec<Vec<u32>>> =
        (0..OUTPUT_BUCKETS).map(|_| (0..L2_SIZE).map(|_| (0..L3_SIZE).map(|_| next()).collect()).collect()).collect();
    let l2_biases: Vec<Vec<u32>> = (0..OUTPUT_BUCKETS).map(|_| (0..L3_SIZE).map(|_| next()).collect()).collect();
    let l3_weights: Vec<Vec<u32>> = (0..OUTPUT_BUCKETS).map(|_| (0..L3_SIZE).map(|_| next()).collect()).collect();
    let l3_biases: Vec<u32> = (0..OUTPUT_BUCKETS).map(|_| next()).collect();

    let weight = |w: &u32| i16::try_from(quantize(*w, TAIL_SHIFT)).expect("tail weight does not fit i16 at TAIL_SHIFT");
    let bias = |b: &u32| i32::try_from(quantize(*b, TAIL_SHIFT + TAIL_ACT_SHIFT)).expect("tail bias does not fit i32");

    let quantized_l1_biases: Vec<Vec<i64>> = l1_biases
        .iter()
        .map(|bucket| bucket.iter().map(|b| quantize(*b, L1_SHIFT + TAIL_ACT_SHIFT)).collect())
        .collect();

    let mut quantized_l2 = vec![[[0i16; L3_SIZE * 2]; L2_SIZE / 2]; OUTPUT_BUCKETS];
    for bucket in 0..OUTPUT_BUCKETS {
        for i in 0..L2_SIZE {
            for j in 0..L3_SIZE {
                quantized_l2[bucket][i / 2][2 * j + i % 2] = weight(&l2_weights[bucket][i][j]);
            }
        }
    }

    let quantized_l2_biases: Vec<Vec<i32>> = l2_biases.iter().map(|bucket| bucket.iter().map(bias).collect()).collect();
    let quantized_l3: Vec<Vec<i16>> = l3_weights.iter().map(|bucket| bucket.iter().map(weight).collect()).collect();
    let quantized_l3_biases: Vec<i32> = l3_biases.iter().map(bias).collect();

    // The i32 accumulators in the engine must fit the worst case of every activation
    // saturated (plus the rounding half). Reject nets that break it.
    let limit = i32::MAX as i64 - (1 << (TAIL_SHIFT - 1));

    for bucket in 0..OUTPUT_BUCKETS {
        for j in 0..L3_SIZE {
            let mut bound = (quantized_l2_biases[bucket][j] as i64).abs();
            for i in 0..L2_SIZE {
                bound += quantized_l2[bucket][i / 2][2 * j + i % 2].unsigned_abs() as i64 * TAIL_ACT_QUANT;
            }
            assert!(bound <= limit, "l2 accumulator can overflow; lower TAIL_SHIFT");
        }

        let mut bound = (quantized_l3_biases[bucket] as i64).abs();
        for w in &quantized_l3[bucket] {
            bound += w.unsigned_abs() as i64 * TAIL_ACT_QUANT;
        }
        assert!(bound <= limit, "l3 accumulator can overflow; lower TAIL_SHIFT");
    }

    let mut image = Vec::with_capacity(INPUT_SIZE);
    image.extend_from_slice(&data[..HEAD_SIZE]);

    for bucket in &quantized_l1_biases {
        for b in bucket {
            image.extend_from_slice(&b.to_le_bytes());
        }
    }
    for bucket in &quantized_l2 {
        for row in bucket {
            for w in row {
                image.extend_from_slice(&w.to_le_bytes());
            }
        }
    }
    for bucket in &quantized_l2_biases {
        for b in bucket {
            image.extend_from_slice(&b.to_le_bytes());
        }
    }
    for bucket in &quantized_l3 {
        for w in bucket {
            image.extend_from_slice(&w.to_le_bytes());
        }
    }
    for b in &quantized_l3_biases {
        image.extend_from_slice(&b.to_le_bytes());
    }
    image.resize(image.len().next_multiple_of(64), 0);

    std::fs::write(output, image).unwrap_or_else(|e| panic!("failed to write {}: {e}", output.display()));
}
