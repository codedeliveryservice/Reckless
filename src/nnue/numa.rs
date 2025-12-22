use std::cell::Cell;
use std::mem;
use std::ptr;

use super::Parameters;

#[cfg(feature = "numa")]
mod sys {
    use libc::{c_int, c_void, size_t};

    #[link(name = "numa")]
    extern "C" {
        pub fn numa_alloc_onnode(size: size_t, node: c_int) -> *mut c_void;
        pub fn numa_free(mem: *mut c_void, size: size_t);
        pub fn numa_max_node() -> c_int;
        pub fn numa_node_of_cpu(cpu: c_int) -> c_int;
    }
}

#[derive(Copy, Clone)]
struct LocalCache {
    owner: usize,
    ptr: *const (),
}

const EMPTY_CACHE: LocalCache = LocalCache { owner: 0, ptr: ptr::null() };

thread_local! {
    static LOCAL_CACHE: Cell<LocalCache> = const { Cell::new(EMPTY_CACHE) };
}

pub unsafe trait NumaValue: Sync {}

unsafe impl NumaValue for Parameters {}

#[allow(dead_code)]
pub struct NumaReplicator<T: NumaValue> {
    nodes: Vec<*mut T>,
    allocation_size: usize,
    owns_allocations: bool,
}

unsafe impl<T: NumaValue> Send for NumaReplicator<T> {}
unsafe impl<T: NumaValue> Sync for NumaReplicator<T> {}

impl<T: NumaValue> NumaReplicator<T> {
    pub fn new(source: &'static T) -> Self {
        let allocation_size = mem::size_of::<T>();
        if allocation_size == 0 {
            return Self::fallback(source, 1);
        }

        #[cfg(feature = "numa")]
        {
            let node_count = node_count();
            let mut nodes: Vec<*mut T> = Vec::with_capacity(node_count);

            for node in 0..node_count {
                let ptr = unsafe { sys::numa_alloc_onnode(allocation_size, node as libc::c_int) } as *mut T;
                if ptr.is_null() {
                    for &allocated in &nodes {
                        unsafe { sys::numa_free(allocated.cast::<libc::c_void>(), allocation_size) };
                    }
                    return Self::fallback(source, node_count);
                }

                // SAFETY: T: NumaValue guarantees a byte-copy is valid for read-only sharing.
                unsafe { ptr::copy_nonoverlapping(source, ptr, 1) };
                nodes.push(ptr);
            }

            Self { nodes, allocation_size, owns_allocations: true }
        }

        #[cfg(not(feature = "numa"))]
        {
            let _ = allocation_size;
            let node_count = node_count();
            Self::fallback(source, node_count)
        }
    }

    pub fn get_local_weights(&self) -> &T {
        let self_id = self as *const Self as usize;
        let ptr = LOCAL_CACHE.with(|cache| {
            let cached = cache.get();
            if cached.owner == self_id && !cached.ptr.is_null() {
                return cached.ptr;
            }

            let node = self.current_node();
            let ptr = self.nodes.get(node).copied().unwrap_or_else(|| self.nodes[0]) as *const T;
            cache.set(LocalCache { owner: self_id, ptr: ptr.cast() });
            ptr.cast()
        });

        unsafe { &*(ptr as *const T) }
    }

    fn current_node(&self) -> usize {
        if self.nodes.len() <= 1 {
            return 0;
        }

        #[cfg(feature = "numa")]
        {
            let cpu = unsafe { libc::sched_getcpu() };
            if cpu < 0 {
                return 0;
            }

            let node = unsafe { sys::numa_node_of_cpu(cpu) };
            if node < 0 {
                return 0;
            }

            let node = node as usize;
            if node >= self.nodes.len() {
                0
            } else {
                node
            }
        }

        #[cfg(not(feature = "numa"))]
        {
            0
        }
    }

    fn fallback(source: &'static T, node_count: usize) -> Self {
        let count = node_count.max(1);
        let ptr = source as *const T as *mut T;
        let mut nodes = Vec::with_capacity(count);
        nodes.resize(count, ptr);

        Self { nodes, allocation_size: 0, owns_allocations: false }
    }
}

#[cfg(feature = "numa")]
impl<T: NumaValue> Drop for NumaReplicator<T> {
    fn drop(&mut self) {
        if !self.owns_allocations {
            return;
        }

        for &ptr in &self.nodes {
            unsafe { sys::numa_free(ptr.cast::<libc::c_void>(), self.allocation_size) };
        }
    }
}

pub type NumaNodes = NumaReplicator<Parameters>;

#[cfg(feature = "numa")]
fn node_count() -> usize {
    let max_node = unsafe { sys::numa_max_node() };
    if max_node < 0 {
        1
    } else {
        max_node as usize + 1
    }
}

#[cfg(not(feature = "numa"))]
fn node_count() -> usize {
    1
}
