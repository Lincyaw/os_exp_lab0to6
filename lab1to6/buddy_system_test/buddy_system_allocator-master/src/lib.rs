#![feature(const_fn)]
#![feature(alloc, allocator_api)]
#![no_std]
#![allow(stable_features)]

#[cfg(test)]
#[macro_use]
extern crate std;

#[cfg(feature = "use_spin")]
extern crate spin;

extern crate alloc;

#[rustversion::before(2020-02-02)]
use alloc::alloc::Alloc;
#[rustversion::since(2020-02-02)]
use alloc::alloc::AllocRef;
use alloc::alloc::{AllocErr, Layout};
#[rustversion::since(2020-04-02)]
use alloc::alloc::{AllocInit, MemoryBlock};
use core::alloc::GlobalAlloc;
use core::cmp::{max, min};
use core::fmt;
use core::mem::size_of;
#[cfg(feature = "use_spin")]
use core::ops::Deref;
use core::ptr::NonNull;
#[cfg(feature = "use_spin")]
use spin::Mutex;

mod frame;
pub mod linked_list;
#[cfg(test)]
mod test;

pub use frame::*;

/// A heap that uses buddy system
///
/// # Usage
///
/// Create a heap and add a memory region to it:
/// ```
/// use buddy_system_allocator::*;
/// # use core::mem::size_of;
/// let mut heap = Heap::empty();
/// # let space: [usize; 100] = [0; 100];
/// # let begin: usize = space.as_ptr() as usize;
/// # let end: usize = begin + 100 * size_of::<usize>();
/// # let size: usize = 100 * size_of::<usize>();
/// unsafe {
///     heap.init(begin, size);
///     // or
///     heap.add_to_heap(begin, end);
/// }
/// ```
pub struct Heap {
    // buddy system with max order of 32
    free_list: [linked_list::LinkedList; 32],  //斯坦福大学实现的链表, 这里有32个链表

    // statistics
    user: usize,
    allocated: usize,
    total: usize,
}

impl Heap {
    /// Create an empty heap
    pub const fn new() -> Self {
        Heap {
            free_list: [linked_list::LinkedList::new(); 32],
            user: 0,
            allocated: 0,
            total: 0,
        }
    }

    /// Create an empty heap
    pub const fn empty() -> Self {
        Self::new()
    }

    /// Add a range of memory [start, end) to the heap
    pub unsafe fn add_to_heap(&mut self, mut start: usize, mut end: usize) {
        // avoid unaligned access on some platforms
        start = (start + size_of::<usize>() - 1) & (!size_of::<usize>() + 1);
        end = end & (!size_of::<usize>() + 1);
        assert!(start <= end);

        let mut total = 0;
        let mut current_start = start;

        
        //假设start=4
        while current_start + size_of::<usize>() <= end {   //如果开始的位置到结束的位置之间还有大于usize大小的位置
            let lowbit = current_start & (!current_start + 1);  //令lowbit等于从右到左第一个不为0的位置, 只可能是1, 10, 100, 1000...
            //lowbit用于判断该地址最大能够存放多大的内存(基于伙伴系统, 地址应该是2^n次)
            let size = min(lowbit, prev_power_of_two(end - current_start));
            //prev_power_of_two(end - current_start) 是需要存放的内存, 理论上这次能存放的最大的内存(必须是2^n)
            println!("start: {},  end: {}", current_start, end);
            println!("end - current_start: {}",end - current_start);
            println!("lowbit: {}, prev_power_of_two(end - current_start): {}\n", lowbit, prev_power_of_two(end - current_start));
            total += size;

            self.free_list[size.trailing_zeros() as usize].push(current_start as *mut usize);
            current_start += size;
        }
        //循环结束之后, 就存放完了所有需要的分配给堆区的内存
        println!("{:?}",total);
        self.total += total;
    }

    /// Add a range of memory [start, end) to the heap
    pub unsafe fn init(&mut self, start: usize, size: usize) {
        self.add_to_heap(start, start + size);
    }

    /// Alloc a range of memory from the heap satifying `layout` requirements
    pub fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
        let size = max(
            layout.size().next_power_of_two(),
            max(layout.align(), size_of::<usize>()),
        );
        println!("\n申请");
        println!("size: {}",size);
        let class = size.trailing_zeros() as usize;//size有几个0
        println!("class: {}", class);
        println!("free_list_len: {}", self.free_list.len());
        for i in class..self.free_list.len() {  //freelist的长度是这块堆区可分割的最大的n, 其中2^n==堆区大小
            // 因为需要可以分配的大小必须大于待分配的大小, 所以只要在堆区中找大于待分配的内存的大小的块即可
            // Find the first non-empty size class
            if !self.free_list[i].is_empty() {     //如果存在2^i大小的块
                println!("存在大小为2^{}的块",i);
                // Split buffers
                for j in (class + 1..i + 1).rev() {   //直到找到和size匹配的块
                    if let Some(block) = self.free_list[j].pop() {
                        println!("freelist里大小为{}的块被成功取出来了, 首地址是{}",j, block as usize);
                        unsafe {
                            self.free_list[j - 1]
                                .push((block as usize + (1 << (j - 1))) as *mut usize);
                            println!("freelist[{}]里存进去了{}",j-1, block as usize + (1 << (j - 1)));
                            self.free_list[j - 1].push(block);
                            println!("freelist[{}]也里存进去了{}",j-1,block as usize);
                        }
                    } else {
                        return Err(AllocErr {});
                    }
                }
                //上面分割结束之后, 就拿出一块对应大小的用来给到需求
                let result = NonNull::new(
                    self.free_list[class]
                        .pop()
                        .expect("current block should have free space now") as *mut u8,
                );
                if let Some(result) = result {//如果取出来没问题的话, 就把对应的数值改变一下
                    self.user += layout.size();
                    self.allocated += size;
                    return Ok(result);
                } else {
                    return Err(AllocErr {});
                }
            }else{println!("不存在大小为2^{}的块",i);}
            
        }
        Err(AllocErr {})
    }

    /// Dealloc a range of memory from the heap
    pub fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        let size = max(
            layout.size().next_power_of_two(),
            max(layout.align(), size_of::<usize>()),
        );
        let class = size.trailing_zeros() as usize;
        println!("\n释放");
        println!("size: {}",size);
        println!("class: {}", class);
        unsafe {
            // Put back into free list 把这块内存重新放到堆区
            self.free_list[class].push(ptr.as_ptr() as *mut usize);

            // Merge free buddy lists
            //查看是否有能够合并的块
            let mut current_ptr = ptr.as_ptr() as usize;
            let mut current_class = class;
            while current_class < self.free_list.len() {
                let buddy = current_ptr ^ (1 << current_class); //改变 1class个0 那个位的取值, 原来是1就变成0, 原来是0就变成1
                println!("current_ptr: {}", current_ptr);
                println!("buddy: {}", buddy);
                let mut flag = false;
                for block in self.free_list[current_class].iter_mut() {
                    println!("开始在freelist[{}]中找和block邻近的块",current_class);
                    if block.value() as usize == buddy {
                        println!("block.value() as usize {} 是 buddy", block.value() as usize);
                        block.pop();
                        flag = true;
                        break;
                    }else{
                        println!("block.value() as usize {} 不是 buddy", block.value() as usize);
                    }
                }

                // Free buddy found
                if flag {
                    println!("成功合并了current_ptr和buddy, 合并后的地址为{}", current_ptr);
                    self.free_list[current_class].pop();
                    current_ptr = min(current_ptr, buddy);
                    current_class += 1;
                    self.free_list[current_class].push(current_ptr as *mut usize);
                } else {
                    break;
                }
            }
        }

        self.user -= layout.size();
        self.allocated -= size;
    }

    /// Return the number of bytes that user requests
    pub fn stats_alloc_user(&self) -> usize {
        self.user
    }

    /// Return the number of bytes that are actually allocated
    pub fn stats_alloc_actual(&self) -> usize {
        self.allocated
    }

    /// Return the total number of bytes in the heap
    pub fn stats_total_bytes(&self) -> usize {
        self.total
    }
}

impl fmt::Debug for Heap {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Heap")
            .field("user", &self.user)
            .field("allocated", &self.allocated)
            .field("total", &self.total)
            .finish()
    }
}

#[rustversion::before(2020-02-02)]
unsafe impl Alloc for Heap {
    unsafe fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
        self.alloc(layout)
    }

    unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        self.dealloc(ptr, layout)
    }
}

#[rustversion::since(2020-02-02)]
unsafe impl AllocRef for Heap {
    #[rustversion::before(2020-03-03)]
    unsafe fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
        self.alloc(layout)
    }

    #[rustversion::all(since(2020-03-03), before(2020-03-10))]
    unsafe fn alloc(&mut self, layout: Layout) -> Result<(NonNull<u8>, usize), AllocErr> {
        self.alloc(layout).map(|p| (p, layout.size()))
    }

    #[rustversion::all(since(2020-03-10), before(2020-04-02))]
    fn alloc(&mut self, layout: Layout) -> Result<(NonNull<u8>, usize), AllocErr> {
        self.alloc(layout).map(|p| (p, layout.size()))
    }

    #[rustversion::since(2020-04-02)]
    fn alloc(&mut self, layout: Layout, init: AllocInit) -> Result<MemoryBlock, AllocErr> {
        self.alloc(layout).map(|p| {
            let block = MemoryBlock {
                ptr: p,
                size: layout.size(),
            };
            unsafe {
                init.init(block);
            }
            block
        })
    }

    unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        self.dealloc(ptr, layout)
    }
}

/// A locked version of `Heap`
///
/// # Usage
///
/// Create a locked heap and add a memory region to it:
/// ```
/// use buddy_system_allocator::*;
/// # use core::mem::size_of;
/// let mut heap = LockedHeap::new();
/// # let space: [usize; 100] = [0; 100];
/// # let begin: usize = space.as_ptr() as usize;
/// # let end: usize = begin + 100 * size_of::<usize>();
/// # let size: usize = 100 * size_of::<usize>();
/// unsafe {
///     heap.lock().init(begin, size);
///     // or
///     heap.lock().add_to_heap(begin, end);
/// }
/// ```
#[cfg(feature = "use_spin")]
pub struct LockedHeap(Mutex<Heap>);

#[cfg(feature = "use_spin")]
impl LockedHeap {
    /// Creates an empty heap
    pub const fn new() -> LockedHeap {
        LockedHeap(Mutex::new(Heap::new()))
    }

    /// Creates an empty heap
    pub const fn empty() -> LockedHeap {
        LockedHeap(Mutex::new(Heap::new()))
    }
}

#[cfg(feature = "use_spin")]
impl Deref for LockedHeap {
    type Target = Mutex<Heap>;

    fn deref(&self) -> &Mutex<Heap> {
        &self.0
    }
}

#[cfg(feature = "use_spin")]
unsafe impl GlobalAlloc for LockedHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0
            .lock()
            .alloc(layout)
            .ok()
            .map_or(0 as *mut u8, |allocation| allocation.as_ptr())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0.lock().dealloc(NonNull::new_unchecked(ptr), layout)
    }
}

/// A locked version of `Heap` with rescue before oom
///
/// # Usage
///
/// Create a locked heap:
/// ```
/// use buddy_system_allocator::*;
/// let heap = LockedHeapWithRescue::new(|heap: &mut Heap| {});
/// ```
///
/// Before oom, the allocator will try to call rescue function and try for one more time.
#[cfg(feature = "use_spin")]
pub struct LockedHeapWithRescue {
    inner: Mutex<Heap>,
    rescue: fn(&mut Heap),
}

#[cfg(feature = "use_spin")]
impl LockedHeapWithRescue {
    /// Creates an empty heap
    pub const fn new(rescue: fn(&mut Heap)) -> LockedHeapWithRescue {
        LockedHeapWithRescue {
            inner: Mutex::new(Heap::new()),
            rescue,
        }
    }
}

#[cfg(feature = "use_spin")]
impl Deref for LockedHeapWithRescue {
    type Target = Mutex<Heap>;

    fn deref(&self) -> &Mutex<Heap> {
        &self.inner
    }
}

#[cfg(feature = "use_spin")]
unsafe impl GlobalAlloc for LockedHeapWithRescue {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut inner = self.inner.lock();
        match inner.alloc(layout) {
            Ok(allocation) => allocation.as_ptr(),
            Err(_) => {
                (self.rescue)(&mut inner);
                inner
                    .alloc(layout)
                    .ok()
                    .map_or(0 as *mut u8, |allocation| allocation.as_ptr())
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.inner
            .lock()
            .dealloc(NonNull::new_unchecked(ptr), layout)
    }
}

pub(crate) fn prev_power_of_two(num: usize) -> usize {
    1 << (8 * (size_of::<usize>()) - num.leading_zeros() as usize - 1)
}
