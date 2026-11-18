//! The decoded-block cache: a pool shared by all tables, and each thread's last
//! blocks per table type.

use std::{
    any::Any,
    cell::RefCell,
    collections::{HashMap, VecDeque},
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    thread::LocalKey,
};

pub const TL_BLOCK_SHARDS: usize = 64;

/// A number no earlier call returned.
pub fn next_epoch() -> u64 {
    static CTR: AtomicU64 = AtomicU64::new(0);
    CTR.fetch_add(1, Ordering::Relaxed) + 1
}

type AnyBlock = Arc<dyn Any + Send + Sync>;

/// (owner, block id)
type Key = (u64, usize);

struct Entry {
    blk: AnyBlock,
    bytes: usize,
    used: AtomicBool,
}

#[derive(Default)]
struct ShardInner {
    ring: VecDeque<Key>, // front = the clock hand
    index: HashMap<Key, Entry>,
}

#[derive(Default)]
#[repr(align(64))]
struct Shard {
    inner: RwLock<ShardInner>,
}

/// Decoded blocks of every table, within a byte budget, evicted by clock.
pub struct BlockPool {
    shards: Box<[Shard]>,
    max_bytes: AtomicUsize,
    cur_bytes: AtomicUsize,
}

impl BlockPool {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            shards: (0..TL_BLOCK_SHARDS).map(|_| Shard::default()).collect(),
            max_bytes: AtomicUsize::new(max_bytes),
            cur_bytes: AtomicUsize::new(0),
        }
    }

    fn shard(&self, shard_key: u64) -> &Shard {
        &self.shards[shard_key as usize & (TL_BLOCK_SHARDS - 1)]
    }

    /// The pooled block, if any.
    fn find(&self, key: Key, shard_key: u64) -> Option<AnyBlock> {
        let inner = self.shard(shard_key).inner.read().unwrap();
        let e = inner.index.get(&key)?;
        e.used.store(true, Ordering::Relaxed);
        Some(e.blk.clone())
    }

    /// Pools `blk` unless the shard is busy or holds the block already; returns
    /// the pooled copy where there is one.
    fn try_insert_or_get(&self, key: Key, shard_key: u64, blk: AnyBlock, bytes: usize) -> AnyBlock {
        let Ok(mut inner) = self.shard(shard_key).inner.try_write() else {
            return blk;
        };
        if let Some(e) = inner.index.get(&key) {
            e.used.store(true, Ordering::Relaxed);
            return e.blk.clone();
        }
        inner.ring.push_back(key);
        inner.index.insert(key, Entry { blk: blk.clone(), bytes, used: AtomicBool::new(false) });
        self.cur_bytes.fetch_add(bytes, Ordering::Relaxed);
        self.evict_locked(&mut inner);
        blk
    }

    fn drop_owner(&self, owner: u64) {
        for shard in &self.shards {
            let mut guard = shard.inner.write().unwrap();
            let inner = &mut *guard;
            let index = &mut inner.index;
            let cur = &self.cur_bytes;
            inner.ring.retain(|key| {
                if key.0 != owner {
                    return true;
                }
                if let Some(e) = index.remove(key) {
                    cur.fetch_sub(e.bytes, Ordering::Relaxed);
                }
                false
            });
        }
    }

    pub fn set_max_bytes(&self, n: usize) {
        self.max_bytes.store(n, Ordering::Relaxed);
        for shard in &self.shards {
            self.evict_locked(&mut shard.inner.write().unwrap());
        }
    }

    fn evict_locked(&self, inner: &mut ShardInner) {
        while self.cur_bytes.load(Ordering::Relaxed) > self.max_bytes.load(Ordering::Relaxed) && inner.ring.len() > 1 {
            let key = *inner.ring.front().unwrap();
            if inner.index[&key].used.swap(false, Ordering::Relaxed) {
                inner.ring.rotate_left(1);
                continue;
            }
            let e = inner.index.remove(&key).unwrap();
            self.cur_bytes.fetch_sub(e.bytes, Ordering::Relaxed);
            inner.ring.pop_front();
        }
    }
}

/// One table colour's handle on the pool: its blocks are dropped with it.
pub struct BlockCache {
    pub epoch: u64,
    pool: Arc<BlockPool>,
}

impl BlockCache {
    pub fn new(pool: Arc<BlockPool>) -> Self {
        Self { epoch: next_epoch(), pool }
    }
}

impl Drop for BlockCache {
    fn drop(&mut self) {
        self.pool.drop_owner(self.epoch);
    }
}

/// A thread's last blocks of one table type, round robin.
pub struct TlFifo<T> {
    epoch: Vec<u64>,
    block_id: Vec<usize>,
    blocks: Vec<Option<Arc<T>>>,
    next: usize,
}

impl<T> TlFifo<T> {
    /// `slots` must be a power of two.
    pub fn new(slots: usize) -> Self {
        debug_assert!(slots.is_power_of_two());
        Self {
            epoch: vec![0; slots],
            block_id: vec![0; slots],
            blocks: (0..slots).map(|_| None).collect(),
            next: 0,
        }
    }

    /// `shards` FIFOs of `slots` each.
    pub fn sharded(shards: usize, slots: usize) -> Vec<Self> {
        debug_assert!(shards.is_power_of_two());
        (0..shards).map(|_| Self::new(slots)).collect()
    }
}

/// The block's contents: the thread's FIFO, else (pooled) the shared pool,
/// else `build`, which returns the block and its size in bytes.
pub fn fetch_block<T: Send + Sync + 'static>(
    tl: &'static LocalKey<RefCell<Vec<TlFifo<T>>>>, cache: &BlockCache, block_id: usize, pooled: bool,
    build: impl FnOnce() -> (T, usize),
) -> Arc<T> {
    let key = cache.epoch ^ block_id as u64;
    let hit = tl.with_borrow(|fifos| {
        let f = &fifos[key as usize & (fifos.len() - 1)];
        let slots = f.epoch.len();
        (1..=slots).map(|n| f.next.wrapping_sub(n) & (slots - 1)).find_map(|i| {
            (f.epoch[i] == cache.epoch && f.block_id[i] == block_id).then(|| f.blocks[i].clone().unwrap())
        })
    });
    if let Some(blk) = hit {
        return blk;
    }

    let blk = if pooled {
        let pool_key = (cache.epoch, block_id);
        if let Some(found) = cache.pool.find(pool_key, key) {
            found.downcast::<T>().unwrap()
        } else {
            let (built, bytes) = build();
            let built: AnyBlock = Arc::new(built);
            cache.pool.try_insert_or_get(pool_key, key, built, bytes).downcast::<T>().unwrap()
        }
    } else {
        Arc::new(build().0)
    };

    tl.with_borrow_mut(|fifos| {
        let shard = key as usize & (fifos.len() - 1);
        let f = &mut fifos[shard];
        let slots = f.epoch.len();
        let s = f.next & (slots - 1);
        f.next = f.next.wrapping_add(1);
        f.epoch[s] = cache.epoch;
        f.block_id[s] = block_id;
        f.blocks[s] = Some(blk.clone());
    });
    blk
}
