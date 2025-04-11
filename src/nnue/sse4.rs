#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

pub type Vector = __m128i;

pub const VECTOR_WIDTH: usize = std::mem::size_of::<Vector>() / std::mem::size_of::<i16>();

pub fn zero() -> Vector {
    unsafe { _mm_setzero_si128() }
}

pub fn splat(value: i16) -> Vector {
    unsafe { _mm_set1_epi16(value) }
}

pub fn min(a: Vector, b: Vector) -> Vector {
    unsafe { _mm_min_epi16(a, b) }
}

pub fn max(a: Vector, b: Vector) -> Vector {
    unsafe { _mm_max_epi16(a, b) }
}

pub fn mullo(a: Vector, b: Vector) -> Vector {
    unsafe { _mm_mullo_epi16(a, b) }
}

pub fn add(a: Vector, b: Vector) -> Vector {
    unsafe { _mm_add_epi16(a, b) }
}

pub fn sub(a: Vector, b: Vector) -> Vector {
    unsafe { _mm_sub_epi16(a, b) }
}

pub fn add_i32(a: Vector, b: Vector) -> Vector {
    unsafe { _mm_add_epi32(a, b) }
}

pub fn dot(a: Vector, b: Vector) -> Vector {
    unsafe { _mm_madd_epi16(a, b) }
}

pub fn horizontal_sum(a: Vector) -> i32 {
    unsafe {
        let a = _mm_hadd_epi32(a, a);
        let b = _mm_hadd_epi32(a, a);
        _mm_cvtsi128_si32(b)
    }
}
