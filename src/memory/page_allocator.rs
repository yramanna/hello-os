//! Physical page allocator with 4KB and 2MB page support

use super::multiboot2::MemoryMapTag;
use super::mutex::Mutex;

const PAGE_SIZE_4KB: usize = 4096;
const PAGE_SIZE_2MB: usize = 2 * 1024 * 1024;
const PAGES_PER_2MB: usize = 512;

/// Page size enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageSize {
    Size4KB,
    Size2MB,
}

/// Page state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageState {
    Unavailable,
    Free4KB,
    Free2MB,
    Allocated,
}

/// Metadata for a single page
#[derive(Debug, Clone, Copy)]
struct PageMetadata {
    state: PageState,
    next: Option<usize>,
    prev: Option<usize>,
    counter: u16,  // For superpages: number of free 4KB pages
}

impl PageMetadata {
    const fn new() -> Self {
        Self {
            state: PageState::Unavailable,
            next: None,
            prev: None,
            counter: 0,
        }
    }
}

struct PageArrayWrapper {
    ptr: *mut PageMetadata,
    len: usize,
}

unsafe impl Send for PageArrayWrapper {}
unsafe impl Sync for PageArrayWrapper {}

impl PageArrayWrapper {
    const fn new() -> Self {
        Self {
            ptr: core::ptr::null_mut(),
            len: 0,
        }
    }

    fn as_slice(&self) -> &mut [PageMetadata] {
        unsafe { core::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

/// The physical page allocator
pub struct PageAllocator {
    page_array: Mutex<PageArrayWrapper>,
    free_4kb_list: Mutex<Option<usize>>,
    free_2mb_list: Mutex<Option<usize>>,
    kernel_end: Mutex<usize>,
}

impl PageAllocator {
    pub const fn new() -> Self {
        Self {
            page_array: Mutex::new(PageArrayWrapper::new()),
            free_4kb_list: Mutex::new(None),
            free_2mb_list: Mutex::new(None),
            kernel_end: Mutex::new(0),
        }
    }

    pub unsafe fn init(&self, max_physical_addr: u64, mmap: &MemoryMapTag) {
        // Cap at 4GB
        let max_addr = max_physical_addr.min(4 * 1024 * 1024 * 1024);
        let total_pages = (max_addr as usize + PAGE_SIZE_4KB - 1) / PAGE_SIZE_4KB;
        
        // Get kernel end
        extern "C" { static __end: u8; }
        let kernel_end = (&__end as *const u8 as usize + PAGE_SIZE_4KB - 1) & !(PAGE_SIZE_4KB - 1);
        
        // Allocate page_array after kernel
        let metadata_size = total_pages * core::mem::size_of::<PageMetadata>();
        let page_array_ptr = kernel_end as *mut PageMetadata;
        let page_array_slice = core::slice::from_raw_parts_mut(page_array_ptr, total_pages);
        
        // Initialize all as unavailable
        for i in 0..total_pages {
            page_array_slice[i] = PageMetadata::new();
        }
        
        {
            let mut wrapper = self.page_array.lock();
            wrapper.ptr = page_array_ptr;
            wrapper.len = total_pages;
        }
        
        let final_kernel_end = (kernel_end + metadata_size + PAGE_SIZE_4KB - 1) & !(PAGE_SIZE_4KB - 1);
        *self.kernel_end.lock() = final_kernel_end;
        
        // Mark available regions from memory map
        for entry in mmap.memory_areas() {
            if entry.typ == 1 {
                self.mark_available(entry.base_addr as usize, entry.length as usize);
            }
        }
        
        // Build free lists
        self.build_lists();
    }

    fn mark_available(&self, base: usize, length: usize) {
        let pages = self.page_array.lock().as_slice();
        let start_pfn = base / PAGE_SIZE_4KB;
        let end_pfn = (base + length) / PAGE_SIZE_4KB;
        let kernel_pfn = *self.kernel_end.lock() / PAGE_SIZE_4KB;
        
        let mut pfn = start_pfn.max(kernel_pfn);
        while pfn < end_pfn && pfn < pages.len() {
            let addr = pfn * PAGE_SIZE_4KB;
            
            // Try to make 2MB page
            if addr % PAGE_SIZE_2MB == 0 && pfn + PAGES_PER_2MB <= end_pfn && pfn + PAGES_PER_2MB <= pages.len() {
                pages[pfn].state = PageState::Free2MB;
                pages[pfn].counter = PAGES_PER_2MB as u16;
                for i in 1..PAGES_PER_2MB {
                    pages[pfn + i].state = PageState::Unavailable; // Part of 2MB page
                }
                pfn += PAGES_PER_2MB;
            } else {
                pages[pfn].state = PageState::Free4KB;
                pfn += 1;
            }
        }
    }

    fn build_lists(&self) {
        let pages = self.page_array.lock().as_slice();
        let mut head_4kb = None;
        let mut head_2mb = None;
        
        for pfn in 0..pages.len() {
            match pages[pfn].state {
                PageState::Free4KB => {
                    pages[pfn].next = head_4kb;
                    pages[pfn].prev = None;
                    if let Some(old) = head_4kb {
                        pages[old].prev = Some(pfn);
                    }
                    head_4kb = Some(pfn);
                }
                PageState::Free2MB => {
                    pages[pfn].next = head_2mb;
                    pages[pfn].prev = None;
                    if let Some(old) = head_2mb {
                        pages[old].prev = Some(pfn);
                    }
                    head_2mb = Some(pfn);
                }
                _ => {}
            }
        }
        
        *self.free_4kb_list.lock() = head_4kb;
        *self.free_2mb_list.lock() = head_2mb;
    }

    pub fn allocate_page(&self, size: PageSize) -> Option<usize> {
        match size {
            PageSize::Size4KB => self.alloc_4kb(),
            PageSize::Size2MB => self.alloc_2mb(),
        }
    }

    fn alloc_4kb(&self) -> Option<usize> {
        let mut head = self.free_4kb_list.lock();
        
        if let Some(pfn) = *head {
            let pages = self.page_array.lock().as_slice();
            
            // Remove from list
            *head = pages[pfn].next;
            if let Some(next) = pages[pfn].next {
                pages[next].prev = None;
            }
            
            pages[pfn].state = PageState::Allocated;
            pages[pfn].next = None;
            pages[pfn].prev = None;
            
            drop(head);
            
            // Update superpage counter
            let sp_head = (pfn / PAGES_PER_2MB) * PAGES_PER_2MB;
            let pages = self.page_array.lock().as_slice();
            if pages[sp_head].counter > 0 {
                pages[sp_head].counter -= 1;
            }
            
            return Some(pfn * PAGE_SIZE_4KB);
        }
        
        // Try splitting 2MB page
        drop(head);
        self.split_2mb()?;
        self.alloc_4kb()
    }

    fn alloc_2mb(&self) -> Option<usize> {
        let mut head = self.free_2mb_list.lock();
        let pfn = (*head)?;
        
        let pages = self.page_array.lock().as_slice();
        
        // Remove from list
        *head = pages[pfn].next;
        if let Some(next) = pages[pfn].next {
            pages[next].prev = None;
        }
        
        pages[pfn].state = PageState::Allocated;
        pages[pfn].next = None;
        pages[pfn].prev = None;
        
        Some(pfn * PAGE_SIZE_4KB)
    }

    fn split_2mb(&self) -> Option<()> {
        let mut head = self.free_2mb_list.lock();
        let pfn = (*head)?;
        
        let pages = self.page_array.lock().as_slice();
        
        // Remove from 2MB list
        *head = pages[pfn].next;
        if let Some(next) = pages[pfn].next {
            pages[next].prev = None;
        }
        
        drop(head);
        
        // Convert to 4KB pages
        let pages = self.page_array.lock().as_slice();
        let mut head_4kb = self.free_4kb_list.lock();
        for i in 0..PAGES_PER_2MB {
            let p = pfn + i;
            pages[p].state = PageState::Free4KB;
            pages[p].next = *head_4kb;
            pages[p].prev = None;
            
            if let Some(old) = *head_4kb {
                pages[old].prev = Some(p);
            }
            *head_4kb = Some(p);
        }
        
        pages[pfn].counter = PAGES_PER_2MB as u16;
        
        Some(())
    }

    pub fn free_page(&self, addr: usize, size: PageSize) {
        let pfn = addr / PAGE_SIZE_4KB;
        match size {
            PageSize::Size4KB => self.free_4kb(pfn),
            PageSize::Size2MB => self.free_2mb(pfn),
        }
    }

    fn free_4kb(&self, pfn: usize) {
        let pages = self.page_array.lock().as_slice();
        
        // Update superpage counter
        let sp_head = (pfn / PAGES_PER_2MB) * PAGES_PER_2MB;
        if pages[sp_head].state == PageState::Free4KB || pages[sp_head].state == PageState::Allocated {
            pages[sp_head].counter += 1;
        }
        let can_merge = pages[sp_head].counter == PAGES_PER_2MB as u16;
        
        // Add to 4KB list
        pages[pfn].state = PageState::Free4KB;
        let mut head = self.free_4kb_list.lock();
        pages[pfn].next = *head;
        pages[pfn].prev = None;
        
        if let Some(old) = *head {
            pages[old].prev = Some(pfn);
        }
        *head = Some(pfn);
        drop(head);
        
        // Try to merge
        if can_merge {
            self.try_merge(pfn);
        }
    }

    fn free_2mb(&self, pfn: usize) {
        let pages = self.page_array.lock().as_slice();
        
        pages[pfn].state = PageState::Free2MB;
        pages[pfn].counter = PAGES_PER_2MB as u16;
        
        let mut head = self.free_2mb_list.lock();
        pages[pfn].next = *head;
        pages[pfn].prev = None;
        
        if let Some(old) = *head {
            pages[old].prev = Some(pfn);
        }
        *head = Some(pfn);
    }

    fn try_merge(&self, pfn: usize) {
        let sp_head = (pfn / PAGES_PER_2MB) * PAGES_PER_2MB;
        let pages = self.page_array.lock().as_slice();
        
        // Check all pages are free
        for i in 0..PAGES_PER_2MB {
            if pages[sp_head + i].state != PageState::Free4KB {
                return;
            }
        }
        
        // Remove all from 4KB list
        for i in 0..PAGES_PER_2MB {
            let p = sp_head + i;
            let prev = pages[p].prev;
            let next = pages[p].next;
            
            if let Some(prev_p) = prev {
                pages[prev_p].next = next;
            } else {
                *self.free_4kb_list.lock() = next;
            }
            
            if let Some(next_p) = next {
                pages[next_p].prev = prev;
            }
            
            pages[p].next = None;
            pages[p].prev = None;
        }
        
        // Add as 2MB page
        pages[sp_head].state = PageState::Free2MB;
        pages[sp_head].counter = PAGES_PER_2MB as u16;
        
        let mut head = self.free_2mb_list.lock();
        pages[sp_head].next = *head;
        pages[sp_head].prev = None;
        
        if let Some(old) = *head {
            pages[old].prev = Some(sp_head);
        }
        *head = Some(sp_head);
    }
}
