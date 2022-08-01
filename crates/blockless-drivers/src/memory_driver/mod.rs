use std::{collections::HashMap, sync::Once};
use crate::{read_ext::ReadRemain, S3ErrorKind};

pub struct VecResult {
  content: Vec<u8>,
  read_point: usize,
}

pub enum S3Ctx {
  VecResult(VecResult),
  None,
}

pub fn get_ctx() -> Option<&'static mut HashMap<u32, S3Ctx>> {
  static mut CTX: Option<HashMap<u32, S3Ctx>> = None;
  static CTX_ONCE: Once = Once::new();
  CTX_ONCE.call_once(|| {
      unsafe {
          CTX = Some(HashMap::new());
      };
  });
  unsafe { CTX.as_mut() }
}

impl ReadRemain for VecResult {
  fn as_bytes_ref(&self) -> Option<&[u8]> {
      Some(&self.content)
  }

  fn read_point(&self) -> usize {
      self.read_point
  }

  fn set_read_point(&mut self, point: usize) {
      self.read_point = point;
  }
}


pub async fn read(handle: u32, buf: &mut [u8]) -> Result<u32, S3ErrorKind> {
    let ctx = get_ctx().unwrap();

    let string = "foo";
    
    let mut ret = VecResult {
        content: string.as_bytes().to_vec(),
        read_point: string.as_bytes().to_vec().len(),
    };

    if buf.len() == 0 {
        return Err(S3ErrorKind::InvalidParameter);
    }

    Ok(ret.copy_remain(buf) as _)
    
    // match ctx.get_mut(&handle) {
    //     Some(S3Ctx::VecResult(resp)) =>,
    //     _ => return Err(S3ErrorKind::InvalidHandle),
    // }
}