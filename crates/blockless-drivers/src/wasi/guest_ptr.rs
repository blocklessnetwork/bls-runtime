use std::mem;
use wiggle::{GuestMemory, GuestPtr, GuestType};

#[derive(Debug)]
pub struct ArrayTuple(u32, u32);

/// the tuple type.
impl GuestType for ArrayTuple {
    fn guest_size() -> u32 {
        mem::size_of::<Self>() as u32
    }

    fn guest_align() -> usize {
        mem::align_of::<Self>()
    }

    /// read tuple from memoery
    fn read(memory: &GuestMemory<'_>, ptr: GuestPtr<Self>) -> Result<Self, wiggle::GuestError> {
        let offset = memory.read(ptr.cast::<u32>())?;
        let len = memory.read(ptr.cast::<u32>().add(1)?)?;
        Ok(ArrayTuple(offset, len))
    }

    /// write tuple to memoery
    fn write(
        memory: &mut GuestMemory<'_>,
        ptr: GuestPtr<Self>,
        val: Self,
    ) -> Result<(), wiggle::GuestError> {
        let (offs, len) = (val.0, val.1);
        let len_ptr = ptr.cast::<u32>().add(1)?;
        memory.write(ptr.cast::<u32>(), offs)?;
        memory.write(len_ptr, len)?;
        Ok(())
    }
}
