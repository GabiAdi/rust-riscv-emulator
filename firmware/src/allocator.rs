use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr;
use spin::Mutex;

const MIN_BLOCK_SIZE: usize = core::mem::size_of::<FreeBlock>();

struct FreeBlock {
    size: usize,
    next: Option<*mut FreeBlock>,
}

struct AllocatorState {
    heap_start: usize,
    heap_end: usize,
    head: FreeBlock,
}

pub struct LinkedListAllocator {
    inner: Mutex<AllocatorState>,
}

impl FreeBlock {
    unsafe fn new_at(addr: usize, size: usize, next: Option<*mut FreeBlock>) -> *mut FreeBlock {
        unsafe {
            let block_ptr = addr as *mut FreeBlock;
            block_ptr.write(FreeBlock { size, next });
            block_ptr
        }
    }
    fn start_addr(&self) -> usize {
        self as *const _ as usize
    }
    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
    fn is_fit(&self, layout: Layout) -> bool {
        let aligned_start = align_up(self.start_addr(), layout.align());
        match aligned_start.checked_add(layout.size()) {
            Some(required_end) => required_end <= self.end_addr(),
            None => false,
        }
    }
}

unsafe impl Sync for LinkedListAllocator {}

impl AllocatorState {
    pub fn heap_size(&self) -> usize {
        self.heap_end - self.heap_start
    }
}

impl LinkedListAllocator {
    pub const fn new_uninit() -> Self {
        LinkedListAllocator {
            inner: Mutex::new(AllocatorState {
                heap_start: 0,
                heap_end: 0,
                head: FreeBlock {
                    size: 0,
                    next: None,
                },
            }),
        }
    }
    pub unsafe fn init(&self, heap_start: usize, heap_end: usize) {
        unsafe {
            let state = &mut *self.inner.lock();
            state.heap_start = heap_start;
            state.heap_end = heap_end;
            state.head.size = heap_end - heap_start;
            let block_ptr = FreeBlock::new_at(state.heap_start, state.heap_size(), None);
            state.head.next = Some(block_ptr);
        }
    }
    pub unsafe fn free_space(&self) -> u64 {
        unsafe {
            let state = &mut *self.inner.lock();

            let mut prev = &mut state.head as *mut FreeBlock;
            let mut current = state.head.next;

            let mut free_space: u64 = 0;

            while let Some(block) = current {
                free_space += (*block).size as u64;
                prev = block;
                current = (*block).next;
            }

            free_space
        }
    }
}

unsafe impl GlobalAlloc for LinkedListAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            let state = &mut *self.inner.lock();

            let mut prev = &mut state.head as *mut FreeBlock;
            let mut current = state.head.next;

            while let Some(block) = current {
                if (*block).is_fit(layout) {
                    break;
                }
                prev = block;
                current = (*block).next;
            }

            let current_free = match current {
                Some(b) => b,
                None => return ptr::null_mut(),
            };

            let alloc_start = align_up((*current_free).start_addr(), layout.align());
            let alloc_end = match alloc_start.checked_add(layout.size()) {
                Some(end) => end,
                None => return ptr::null_mut(),
            };

            if alloc_end > (*current_free).end_addr() {
                return ptr::null_mut();
            }

            let next_free: Option<*mut FreeBlock>;
            match (*current_free).next {
                Some(free_block) => {
                    let gap_size = (*free_block).start_addr() - alloc_end;
                    if gap_size >= MIN_BLOCK_SIZE {
                        next_free = Some(FreeBlock::new_at(alloc_end, gap_size, Some(free_block)));
                    } else {
                        next_free = Some(free_block);
                    }
                }
                None => {
                    let gap_size = state.heap_end - alloc_end;
                    if gap_size >= MIN_BLOCK_SIZE {
                        next_free = Some(FreeBlock::new_at(alloc_end, gap_size, None));
                    } else {
                        next_free = None;
                    }
                }
            }

            if alloc_start != (*current_free).start_addr() {
                (*current_free).size = alloc_start - (*current_free).start_addr();
                (*current_free).next = next_free;
            } else {
                if prev == &mut state.head as *mut FreeBlock {
                    state.head.next = next_free;
                } else {
                    (*prev).next = next_free;
                }
            }

            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            let state = &mut *self.inner.lock();
            let start_addr = ptr as usize;

            let mut prev_block = &mut state.head as *mut FreeBlock;
            let mut next_block = state.head.next;
            while let Some(block) = next_block {
                if (*block).start_addr() > start_addr {
                    break;
                }
                prev_block = block;
                next_block = (*block).next;
            }

            let current_block = FreeBlock::new_at(start_addr, layout.size(), next_block);
            (*prev_block).next = Some(current_block);

            prev_block = &mut state.head as *mut FreeBlock;
            next_block = state.head.next;
            while let Some(block) = next_block {
                if (*prev_block).end_addr() == (*block).start_addr() {
                    (*prev_block).next = (*block).next;
                    (*prev_block).size += (*block).size;
                    next_block = (*prev_block).next;
                    continue;
                }
                prev_block = block;
                next_block = (*block).next;
            }
        }
    }
}

fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}
