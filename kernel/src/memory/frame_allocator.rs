// Aetherion OS - Physical Frame Allocator
// Manages physical memory frames using bitmap tracking

use super::*;
use super::bitmap::Bitmap;

/// Physical Frame Allocator
/// 
/// Manages allocation and deallocation of physical memory frames (4KB pages).
/// Uses a bitmap for O(n) allocation and O(1) deallocation.
/// 
/// # Memory Layout
/// ```
/// Physical Memory:
/// ┌─────────────────┬──────────────────┬─────────────────────┐
/// │ Kernel & Boot   │ Frame Allocator  │ Available Frames    │
/// │ (0x0 - 0x10000) │ Bitmap           │ (managed by bitmap) │
/// └─────────────────┴──────────────────┴─────────────────────┘
/// ```
pub struct FrameAllocator {
    bitmap: Bitmap,
    start_address: PhysicalAddress,
    total_frames: usize,
    allocated_frames: usize,
}

impl FrameAllocator {
    /// Create a new frame allocator
    /// 
    /// # Arguments
    /// * `start_addr` - Starting physical address of managed region
    /// * `memory_size` - Total size of memory region in bytes
    /// * `bitmap_buffer` - Static mutable buffer for bitmap storage
    /// 
    /// # Example
    /// ```
    /// static mut BITMAP: [u8; 2048] = [0; 2048];
    /// 
    /// let allocator = FrameAllocator::new(
    ///     PhysicalAddress::new(0x100000),  // Start at 1MB
    ///     16 * 1024 * 1024,                // 16MB total
    ///     unsafe { &mut BITMAP },
    /// );
    /// ```
    pub fn new(
        start_addr: PhysicalAddress,
        memory_size: usize,
        bitmap_buffer: &'static mut [u8],
    ) -> Self {
        let total_frames = memory_size / FRAME_SIZE;
        let bitmap = Bitmap::new(bitmap_buffer, total_frames);
        
        Self {
            bitmap,
            start_address: start_addr,
            total_frames,
            allocated_frames: 0,
        }
    }
    
    /// Allocate a single physical frame
    /// 
    /// # Returns
    /// * `Some(PhysicalAddress)` - Physical address of allocated frame
    /// * `None` - No frames available (out of memory)
    /// 
    /// # Performance
    /// O(n) worst case (scans bitmap for free frame)
    /// 
    /// # Example
    /// ```
    /// if let Some(frame) = allocator.allocate_frame() {
    ///     println!("Allocated frame at 0x{:x}", frame.as_usize());
    /// } else {
    ///     println!("Out of memory!");
    /// }
    /// ```
    pub fn allocate_frame(&mut self) -> Option<PhysicalAddress> {
        if let Some(frame_idx) = self.bitmap.find_first_clear() {
            self.bitmap.set(frame_idx);
            self.allocated_frames += 1;
            
            let addr = self.start_address.as_usize() + (frame_idx * FRAME_SIZE);
            Some(PhysicalAddress::new(addr))
        } else {
            None  // Out of memory
        }
    }
    
    /// Allocate multiple consecutive frames
    /// 
    /// Useful for allocating large contiguous memory regions.
    /// 
    /// # Arguments
    /// * `count` - Number of consecutive frames to allocate
    /// 
    /// # Returns
    /// * `Some(PhysicalAddress)` - Start address of allocated region
    /// * `None` - Not enough consecutive frames available
    pub fn allocate_frames(&mut self, count: usize) -> Option<PhysicalAddress> {
        if count == 0 {
            return None;
        }
        
        if let Some(start_idx) = self.bitmap.find_consecutive_clear(count) {
            // Mark all frames as allocated
            for idx in start_idx..(start_idx + count) {
                self.bitmap.set(idx);
            }
            
            self.allocated_frames += count;
            
            let addr = self.start_address.as_usize() + (start_idx * FRAME_SIZE);
            Some(PhysicalAddress::new(addr))
        } else {
            None
        }
    }
    
    /// Deallocate a physical frame
    /// 
    /// # Arguments
    /// * `addr` - Physical address of frame to deallocate
    ///           (will be aligned down to frame boundary)
    /// 
    /// # Performance
    /// O(1) - Direct bitmap update
    /// 
    /// # Safety
    /// Caller must ensure frame is not in use before deallocating
    pub fn deallocate_frame(&mut self, addr: PhysicalAddress) {
        let aligned = addr.align_down(FRAME_SIZE);
        let frame_idx = (aligned.as_usize() - self.start_address.as_usize()) / FRAME_SIZE;
        
        if frame_idx < self.total_frames && self.bitmap.is_set(frame_idx) {
            self.bitmap.clear(frame_idx);
            self.allocated_frames -= 1;
        }
    }
    
    /// Deallocate multiple consecutive frames
    /// 
    /// # Arguments
    /// * `addr` - Start address of region to deallocate
    /// * `count` - Number of frames to deallocate
    pub fn deallocate_frames(&mut self, addr: PhysicalAddress, count: usize) {
        let aligned = addr.align_down(FRAME_SIZE);
        let start_idx = (aligned.as_usize() - self.start_address.as_usize()) / FRAME_SIZE;
        
        for idx in start_idx..(start_idx + count) {
            if idx < self.total_frames && self.bitmap.is_set(idx) {
                self.bitmap.clear(idx);
                self.allocated_frames -= 1;
            }
        }
    }
    
    /// Get number of free frames
    pub fn free_frames(&self) -> usize {
        self.total_frames - self.allocated_frames
    }
    
    /// Get number of allocated frames
    pub fn allocated_frames(&self) -> usize {
        self.allocated_frames
    }
    
    /// Get total number of frames managed
    pub fn total_frames(&self) -> usize {
        self.total_frames
    }
    
    /// Get total memory size in bytes
    pub fn total_memory(&self) -> usize {
        self.total_frames * FRAME_SIZE
    }
    
    /// Get free memory size in bytes
    pub fn free_memory(&self) -> usize {
        self.free_frames() * FRAME_SIZE
    }
    
    /// Get allocated memory size in bytes
    pub fn allocated_memory(&self) -> usize {
        self.allocated_frames * FRAME_SIZE
    }
    
    /// Get memory usage percentage (0-100)
    pub fn usage_percent(&self) -> usize {
        if self.total_frames == 0 {
            return 0;
        }
        
        (self.allocated_frames * 100) / self.total_frames
    }
    
    /// Check if a frame is allocated
    /// 
    /// # Arguments
    /// * `addr` - Physical address to check
    pub fn is_allocated(&self, addr: PhysicalAddress) -> bool {
        let aligned = addr.align_down(FRAME_SIZE);
        let frame_idx = (aligned.as_usize() - self.start_address.as_usize()) / FRAME_SIZE;
        
        frame_idx < self.total_frames && self.bitmap.is_set(frame_idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_allocate_single_frame() {
        static mut BITMAP_BUF: [u8; 128] = [0; 128];
        let buf = unsafe { &mut BITMAP_BUF };
        
        let mut allocator = FrameAllocator::new(
            PhysicalAddress::new(0x100000),
            1024 * FRAME_SIZE,
            buf,
        );
        
        // Allocate first frame
        let frame1 = allocator.allocate_frame();
        assert!(frame1.is_some());
        assert_eq!(frame1.unwrap().as_usize(), 0x100000);
        assert_eq!(allocator.allocated_frames(), 1);
        
        // Allocate second frame
        let frame2 = allocator.allocate_frame();
        assert!(frame2.is_some());
        assert_eq!(frame2.unwrap().as_usize(), 0x100000 + FRAME_SIZE);
        assert_eq!(allocator.allocated_frames(), 2);
    }
    
    #[test]
    fn test_deallocate_frame() {
        static mut BITMAP_BUF: [u8; 128] = [0; 128];
        let buf = unsafe { &mut BITMAP_BUF };
        
        let mut allocator = FrameAllocator::new(
            PhysicalAddress::new(0x100000),
            1024 * FRAME_SIZE,
            buf,
        );
        
        let frame = allocator.allocate_frame().unwrap();
        assert_eq!(allocator.allocated_frames(), 1);
        
        allocator.deallocate_frame(frame);
        assert_eq!(allocator.allocated_frames(), 0);
        
        // Should be able to allocate same frame again
        let frame2 = allocator.allocate_frame().unwrap();
        assert_eq!(frame.as_usize(), frame2.as_usize());
    }
    
    #[test]
    fn test_allocate_multiple_frames() {
        static mut BITMAP_BUF: [u8; 128] = [0; 128];
        let buf = unsafe { &mut BITMAP_BUF };
        
        let mut allocator = FrameAllocator::new(
            PhysicalAddress::new(0x100000),
            1024 * FRAME_SIZE,
            buf,
        );
        
        // Allocate 10 consecutive frames
        let frames = allocator.allocate_frames(10);
        assert!(frames.is_some());
        assert_eq!(allocator.allocated_frames(), 10);
        
        // Deallocate them
        allocator.deallocate_frames(frames.unwrap(), 10);
        assert_eq!(allocator.allocated_frames(), 0);
    }
    
    #[test]
    fn test_memory_stats() {
        static mut BITMAP_BUF: [u8; 128] = [0; 128];
        let buf = unsafe { &mut BITMAP_BUF };
        
        let mut allocator = FrameAllocator::new(
            PhysicalAddress::new(0x100000),
            1024 * FRAME_SIZE,
            buf,
        );
        
        assert_eq!(allocator.total_frames(), 1024);
        assert_eq!(allocator.total_memory(), 1024 * FRAME_SIZE);
        assert_eq!(allocator.free_frames(), 1024);
        assert_eq!(allocator.usage_percent(), 0);
        
        // Allocate 50% of frames
        for _ in 0..512 {
            allocator.allocate_frame();
        }
        
        assert_eq!(allocator.usage_percent(), 50);
        assert_eq!(allocator.free_frames(), 512);
    }
    
    #[test]
    fn test_out_of_memory() {
        static mut BITMAP_BUF: [u8; 2] = [0; 2];  // Only 16 frames
        let buf = unsafe { &mut BITMAP_BUF };
        
        let mut allocator = FrameAllocator::new(
            PhysicalAddress::new(0x100000),
            16 * FRAME_SIZE,
            buf,
        );
        
        // Allocate all frames
        for _ in 0..16 {
            assert!(allocator.allocate_frame().is_some());
        }
        
        // 17th allocation should fail
        assert!(allocator.allocate_frame().is_none());
    }
}
