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
    pub fn new(source: &'static T) -> Self {
        #[cfg(feature = "numa")]
        {
            if unsafe { api::numa_available() } < 0 {
                return Self::fallback(source);
            }

            let size = std::mem::size_of::<T>();
            let nodes = unsafe { api::numa_max_node() } as usize + 1;
            let nodes = (0..nodes)
                .map(|node| {
                    let ptr = unsafe { api::numa_alloc_onnode(size, node as libc::c_int) } as *mut T;
                    if ptr.is_null() {
                        panic!("Failed to allocate NUMA memory on node {node}");
                    }

                    // SAFETY: T: NumaValue guarantees a byte-copy is valid for read-only sharing.
                    unsafe { std::ptr::copy_nonoverlapping(source, ptr, 1) };
                    ptr
                })
                .collect::<Vec<_>>();

            Self { nodes, size: Some(size), owned: None }
        }

        #[cfg(not(feature = "numa"))]
        Self::fallback(source)
    }

    pub fn new_from_owned(source: T) -> Self {
        #[cfg(feature = "numa")]
        {
            if unsafe { api::numa_available() } < 0 {
                let boxed = Box::new(source);
                let ptr = boxed.as_ref() as *const T as *mut T;
                return Self { nodes: vec![ptr], size: None, owned: Some(boxed) };
            }

            let size = std::mem::size_of::<T>();
            let nodes = unsafe { api::numa_max_node() } as usize + 1;
            let nodes = (0..nodes)
                .map(|node| {
                    let ptr = unsafe { api::numa_alloc_onnode(size, node as libc::c_int) } as *mut T;
                    if ptr.is_null() {
                        panic!("Failed to allocate NUMA memory on node {node}");
                    }

                    // SAFETY: T: NumaValue guarantees a byte-copy is valid for read-only sharing.
                    unsafe { std::ptr::copy_nonoverlapping(&source, ptr, 1) };
                    ptr
                })
                .collect::<Vec<_>>();

            std::mem::drop(source);
            return Self { nodes, size: Some(size), owned: None };
        }

        #[cfg(not(feature = "numa"))]
        {
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
