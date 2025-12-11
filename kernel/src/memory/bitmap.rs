// Aetherion OS - Bitmap Allocator
// Efficient bit-level tracking for frame allocation

/// Bitmap structure for tracking allocated/free frames
/// Each bit represents one frame (4KB)
pub struct Bitmap {
    data: &'static mut [u8],
    size: usize,  // Number of bits (frames) tracked
}

impl Bitmap {
    /// Create a new bitmap from a mutable byte slice
    /// 
    /// # Arguments
    /// * `data` - Mutable reference to backing storage
    /// * `size` - Number of bits (frames) to track
    /// 
    /// # Example
    /// ```
    /// let mut buffer = [0u8; 128];  // Track 1024 frames (4MB)
    /// let bitmap = Bitmap::new(&mut buffer, 1024);
    /// ```
    pub fn new(data: &'static mut [u8], size: usize) -> Self {
        // Clear all bits initially (all frames free)
        for byte in data.iter_mut() {
            *byte = 0;
        }
        
        Self { data, size }
    }
    
    /// Mark a frame as allocated (set bit to 1)
    /// 
    /// # Arguments
    /// * `index` - Frame index to mark as allocated
    pub fn set(&mut self, index: usize) {
        if index < self.size {
            let byte_idx = index / 8;
            let bit_idx = index % 8;
            self.data[byte_idx] |= 1 << bit_idx;
        }
    }
    
    /// Mark a frame as free (clear bit to 0)
    /// 
    /// # Arguments
    /// * `index` - Frame index to mark as free
    pub fn clear(&mut self, index: usize) {
        if index < self.size {
            let byte_idx = index / 8;
            let bit_idx = index % 8;
            self.data[byte_idx] &= !(1 << bit_idx);
        }
    }
    
    /// Check if a frame is allocated
    /// 
    /// # Arguments
    /// * `index` - Frame index to check
    /// 
    /// # Returns
    /// `true` if frame is allocated, `false` if free or out of bounds
    pub fn is_set(&self, index: usize) -> bool {
        if index >= self.size {
            return false;
        }
        
        let byte_idx = index / 8;
        let bit_idx = index % 8;
        (self.data[byte_idx] & (1 << bit_idx)) != 0
    }
    
    /// Find the first free frame (unset bit)
    /// 
    /// # Returns
    /// `Some(index)` of first free frame, or `None` if all allocated
    /// 
    /// # Performance
    /// O(n) worst case, but optimized to skip full bytes (0xFF)
    pub fn find_first_clear(&self) -> Option<usize> {
        for (byte_idx, &byte) in self.data.iter().enumerate() {
            // Skip fully allocated bytes (all bits set)
            if byte != 0xFF {
                // Check each bit in this byte
                for bit_idx in 0..8 {
                    let index = byte_idx * 8 + bit_idx;
                    
                    // Check bounds
                    if index >= self.size {
                        return None;
                    }
                    
                    // Found a free frame!
                    if !self.is_set(index) {
                        return Some(index);
                    }
                }
            }
        }
        
        None  // No free frames
    }
    
    /// Find N consecutive free frames
    /// 
    /// # Arguments
    /// * `count` - Number of consecutive frames needed
    /// 
    /// # Returns
    /// `Some(start_index)` of first frame in sequence, or `None` if not found
    pub fn find_consecutive_clear(&self, count: usize) -> Option<usize> {
        if count == 0 {
            return None;
        }
        
        let mut consecutive = 0;
        let mut start_idx = 0;
        
        for idx in 0..self.size {
            if !self.is_set(idx) {
                if consecutive == 0 {
                    start_idx = idx;
                }
                consecutive += 1;
                
                if consecutive == count {
                    return Some(start_idx);
                }
            } else {
                consecutive = 0;
            }
        }
        
        None
    }
    
    /// Count total free frames
    /// 
    /// # Returns
    /// Number of free (unset) frames
    pub fn count_free(&self) -> usize {
        let mut count = 0;
        
        for idx in 0..self.size {
            if !self.is_set(idx) {
                count += 1;
            }
        }
        
        count
    }
    
    /// Count total allocated frames
    /// 
    /// # Returns
    /// Number of allocated (set) frames
    pub fn count_allocated(&self) -> usize {
        self.size - self.count_free()
    }
    
    /// Get total number of frames tracked
    pub fn size(&self) -> usize {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bitmap_set_clear() {
        static mut BUFFER: [u8; 16] = [0; 16];
        let buf = unsafe { &mut BUFFER };
        
        let mut bitmap = Bitmap::new(buf, 128);
        
        // Initially all clear
        assert!(!bitmap.is_set(0));
        assert!(!bitmap.is_set(50));
        
        // Set some bits
        bitmap.set(0);
        bitmap.set(50);
        bitmap.set(127);
        
        assert!(bitmap.is_set(0));
        assert!(bitmap.is_set(50));
        assert!(bitmap.is_set(127));
        assert!(!bitmap.is_set(1));
        
        // Clear bits
        bitmap.clear(50);
        assert!(!bitmap.is_set(50));
    }
    
    #[test]
    fn test_find_first_clear() {
        static mut BUFFER: [u8; 16] = [0; 16];
        let buf = unsafe { &mut BUFFER };
        
        let mut bitmap = Bitmap::new(buf, 128);
        
        // First clear should be 0
        assert_eq!(bitmap.find_first_clear(), Some(0));
        
        // Allocate first 10 frames
        for i in 0..10 {
            bitmap.set(i);
        }
        
        // First clear should be 10
        assert_eq!(bitmap.find_first_clear(), Some(10));
    }
    
    #[test]
    fn test_find_consecutive_clear() {
        static mut BUFFER: [u8; 16] = [0; 16];
        let buf = unsafe { &mut BUFFER };
        
        let mut bitmap = Bitmap::new(buf, 128);
        
        // Allocate frames 0-9 and 15-19
        for i in 0..10 {
            bitmap.set(i);
        }
        for i in 15..20 {
            bitmap.set(i);
        }
        
        // Should find 4 consecutive at index 10
        assert_eq!(bitmap.find_consecutive_clear(4), Some(10));
        
        // Should find 5 consecutive at index 10
        assert_eq!(bitmap.find_consecutive_clear(5), Some(10));
        
        // Should NOT find 6 consecutive (only 5 available: 10-14)
        assert_eq!(bitmap.find_consecutive_clear(6), Some(20));
    }
    
    #[test]
    fn test_count_free() {
        static mut BUFFER: [u8; 16] = [0; 16];
        let buf = unsafe { &mut BUFFER };
        
        let mut bitmap = Bitmap::new(buf, 128);
        
        assert_eq!(bitmap.count_free(), 128);
        assert_eq!(bitmap.count_allocated(), 0);
        
        // Allocate 10 frames
        for i in 0..10 {
            bitmap.set(i);
        }
        
        assert_eq!(bitmap.count_free(), 118);
        assert_eq!(bitmap.count_allocated(), 10);
    }
}
