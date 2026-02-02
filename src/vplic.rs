use crate::consts::*;
use axaddrspace::{GuestPhysAddr, GuestPhysAddrRange, HostPhysAddr};
use bitmaps::Bitmap;
use core::option::Option;
use spin::Mutex;
use log::{debug, warn};

pub struct VPlicGlobal {
    /// The address of the VPlicGlobal in the guest physical address space.
    pub addr: GuestPhysAddr,
    /// The size of the VPlicGlobal in bytes.
    pub size: usize,
    /// The address range of this device (cached for address_ranges())
    pub addr_range: GuestPhysAddrRange,
    /// Num of contexts.
    pub contexts_num: usize,
    /// IRQs assigned to this VPlicGlobal.
    pub assigned_irqs: Mutex<Bitmap<{ PLIC_NUM_SOURCES }>>,
    /// Pending IRQs for this VPlicGlobal.
    pub pending_irqs: Mutex<Bitmap<{ PLIC_NUM_SOURCES }>>,
    /// Active IRQs for this VPlicGlobal.
    pub active_irqs: Mutex<Bitmap<{ PLIC_NUM_SOURCES }>>,
    /// The host physical address of the PLIC.
    pub host_plic_addr: HostPhysAddr,
}

impl VPlicGlobal {
    pub fn new(addr: GuestPhysAddr, size: Option<usize>, contexts_num: usize) -> Self {
        let addr_end = addr.as_usize()
            + contexts_num * PLIC_CONTEXT_STRIDE
            + PLIC_CONTEXT_CTRL_OFFSET
            + PLIC_CONTEXT_CLAIM_COMPLETE_OFFSET;
        let size = size.expect("Size must be specified for VPlicGlobal");
        assert!(
            addr.as_usize() + size > addr_end,
            "End address 0x{:x} exceeds region [0x{:x}, 0x{:x})  ",
            addr_end,
            addr.as_usize(),
            addr.as_usize() + size,
        );
        Self {
            addr,
            size,
            addr_range: GuestPhysAddrRange::from_start_size(addr, size),
            assigned_irqs: Mutex::new(Bitmap::new()),
            pending_irqs: Mutex::new(Bitmap::new()),
            active_irqs: Mutex::new(Bitmap::new()),
            contexts_num,
            host_plic_addr: HostPhysAddr::from_usize(addr.as_usize()), // Currently we assume host_plic_addr = guest_vplic_addr
        }
    }

    // pub fn assign_irq(&self, irq: u32, cpu_phys_id: usize, target_cpu_affinity: (u8, u8, u8, u8)) {
    //     warn!(
    //         "Assigning IRQ {} to vGICD at addr {:#x} for CPU phys id {} is not supported yet",
    //         irq, self.addr, cpu_phys_id
    //     );
    // }

    /// Set an IRQ as pending in the vPLIC.
    ///
    /// This marks the interrupt as pending in the vPLIC's pending bitmap.
    /// The vCPU will see this interrupt when it checks the vPLIC state.
    ///
    /// # Arguments
    ///
    /// * `irq` - The interrupt number to mark as pending (1-based, 0 is reserved)
    ///
    /// # Returns
    ///
    /// `true` if the IRQ was successfully marked as pending, `false` if the IRQ number is invalid.
    pub fn set_irq_pending(&self, irq: usize) -> bool {
        if irq == 0 || irq >= PLIC_NUM_SOURCES {
            warn!("Invalid IRQ number: {}", irq);
            return false;
        }

        let mut pending = self.pending_irqs.lock();
        if !pending.get(irq) {
            pending.set(irq, true);
            debug!("vPLIC: Set IRQ {} as pending", irq);
            true
        } else {
            // Already pending
            false
        }
    }

    /// Clear a pending IRQ in the vPLIC.
    ///
    /// # Arguments
    ///
    /// * `irq` - The interrupt number to clear
    pub fn clear_irq_pending(&self, irq: usize) -> bool {
        if irq == 0 || irq >= PLIC_NUM_SOURCES {
            return false;
        }

        let mut pending = self.pending_irqs.lock();
        if pending.get(irq) {
            pending.set(irq, false);
            debug!("vPLIC: Cleared IRQ {} pending status", irq);
            true
        } else {
            false
        }
    }

    /// Check if there are any pending interrupts.
    pub fn has_pending_irqs(&self) -> bool {
        let pending = self.pending_irqs.lock();
        // Check if any bit is set in the bitmap
        for i in 1..PLIC_NUM_SOURCES {
            if pending.get(i) {
                return true;
            }
        }
        false
    }
}
