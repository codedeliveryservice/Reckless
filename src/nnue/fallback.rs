pub type Vector = i16;

pub const VECTOR_WIDTH: usize = 1;

pub const fn zero() -> i32 {
    0
}

pub const fn splat(value: Vector) -> i32 {
    value as i32
}

pub fn min(a: Vector, b: i32) -> Vector {
    a.min(b as Vector)
}

pub fn max(a: Vector, b: i32) -> Vector {
    a.max(b as Vector)
}

pub const fn mullo(a: Vector, b: Vector) -> i32 {
    (a as i32) * (b as i32)
}

pub const fn add(a: Vector, b: Vector) -> Vector {
    a + b
}

pub const fn sub(a: Vector, b: Vector) -> Vector {
    a - b
}

pub const fn add_i32(a: i32, b: i32) -> i32 {
    a + b
}

pub const fn dot(a: i32, b: Vector) -> i32 {
    a * (b as i32)
}

pub const fn horizontal_sum(a: i32) -> i32 {
    a
}
