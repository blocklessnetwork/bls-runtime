use std::mem;
use wiggle::{GuestType, GuestPtr, GuestTypeTransparent};

#[derive(Debug)]
pub struct ArrayTuple(u32, u32);

impl<'a> GuestType<'a> for ArrayTuple {
    fn guest_size() -> u32 {
        mem::size_of::<Self>() as u32 
    }

    fn guest_align() -> usize {
        mem::align_of::<Self>() 
    }

    fn read(ptr: &GuestPtr<'a, Self>) -> Result<Self, wiggle::GuestError> {
        let offset = ptr.cast::<u32>().read()?;
        let len = ptr.cast::<u32>().add(1)?.read()?;
        Ok(ArrayTuple(offset, len))
    }

    fn write(ptr: &GuestPtr<'_, Self>, val: Self) -> Result<(), wiggle::GuestError> {
        let (offs, len) = (val.0, val.1);
        let len_ptr = ptr.cast::<u32>().add(1)?;
        ptr.cast::<u32>().write(offs)?;
        len_ptr.write(len)
    }
}

unsafe impl<'a> GuestTypeTransparent<'a> for ArrayTuple {
    fn validate(_ptr: *mut Self) -> Result<(), wiggle::GuestError> {
        Ok(())
    }
}