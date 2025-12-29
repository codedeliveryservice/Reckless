#[cfg(feature = "numa")]
mod api {
    use libc::{c_int, c_void, size_t};

    #[link(name = "numa")]
    extern "C" {
        pub fn numa_available() -> c_int;
        pub fn numa_alloc_onnode(size: size_t, node: c_int) -> *mut c_void;
        pub fn numa_free(mem: *mut c_void, size: size_t);
        pub fn numa_max_node() -> c_int;
        pub fn numa_node_of_cpu(cpu: c_int) -> c_int;
    }
}

pub unsafe trait NumaValue: Sync {}

#[allow(dead_code)]
pub struct NumaReplicator<T: NumaValue> {
    nodes: Vec<*mut T>,
    size: Option<usize>,
    owned: Option<Box<T>>,
}

unsafe impl<T: NumaValue> Send for NumaReplicator<T> {}
unsafe impl<T: NumaValue> Sync for NumaReplicator<T> {}

#[allow(dead_code)]
impl<T: NumaValue> NumaReplicator<T> {
    fn try_replicate_from_ptr(ptr: *const T) -> Option<(Vec<*mut T>, usize)> {
        #[cfg(feature = "numa")]
        {
            if unsafe { api::numa_available() } < 0 {
                return None;
            }

            let size = std::mem::size_of::<T>();
            let nodes = unsafe { api::numa_max_node() } as usize + 1;
            let nodes = (0..nodes)
                .map(|node| {
                    let p = unsafe { api::numa_alloc_onnode(size, node as libc::c_int) } as *mut T;
                    if p.is_null() {
                        panic!("Failed to allocate NUMA memory on node {node}");
                    }

                    // SAFETY: T: NumaValue guarantees a byte-copy is valid for read-only sharing.
                    unsafe { std::ptr::copy_nonoverlapping(ptr, p, 1) };
                    p
                })
                .collect::<Vec<_>>();

            Some((nodes, size))
        }

        #[cfg(not(feature = "numa"))]
        {
            let _ = ptr;
            None
        }
    }

    pub fn new(source: &'static T) -> Self {
        let ptr = source as *const T;
        if let Some((nodes, size)) = Self::try_replicate_from_ptr(ptr) {
            Self { nodes, size: Some(size), owned: None }
        } else {
            Self::fallback(source)
        }
    }

    pub fn new_from_owned(source: T) -> Self {
        let ptr = &source as *const T;
        if let Some((nodes, size)) = Self::try_replicate_from_ptr(ptr) {
            std::mem::drop(source);
            Self { nodes, size: Some(size), owned: None }
        } else {
            let boxed = Box::new(source);
            let ptr = boxed.as_ref() as *const T as *mut T;
            Self { nodes: vec![ptr], size: None, owned: Some(boxed) }
        }
    }

    pub fn get_local_copy(&self) -> &T {
        unsafe { &*(self.nodes[self.current_node()]) }
    }

    fn current_node(&self) -> usize {
        #[cfg(feature = "numa")]
        unsafe {
            api::numa_node_of_cpu(libc::sched_getcpu()) as usize
        }

        #[cfg(not(feature = "numa"))]
        0
    }

    fn fallback(source: &'static T) -> Self {
        let ptr = source as *const T as *mut T;
        Self { nodes: vec![ptr], size: None, owned: None }
    }
}

#[cfg(feature = "numa")]
impl<T: NumaValue> Drop for NumaReplicator<T> {
    fn drop(&mut self) {
        if let Some(size) = self.size {
            for &ptr in &self.nodes {
                unsafe { api::numa_free(ptr.cast::<libc::c_void>(), size) };
            }
        }
    }
}
