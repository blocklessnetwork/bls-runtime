#![allow(dead_code)]
use crate::{Error, Indicator, MultiAddr};

const ADDR_MAP: [bool; 256] = byte_map![
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //  \0                            \n
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //  commands
    0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    //  \w !  "  #  $  %  &  '  (  )  *  +  ,  -  .  /
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1,
    //  0  1  2  3  4  5  6  7  8  9  :  ;  <  =  >  ?
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    //  @  A  B  C  D  E  F  G  H  I  J  K  L  M  N  O
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    //  P  Q  R  S  T  U  V  W  X  Y  Z  [  \  ]  ^  _
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    //  `  a  b  c  d  e  f  g  h  i  j  k  l  m  n  o
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0,
    //  p  q  r  s  t  u  v  w  x  y  z  {  |  }  ~  del
    //   ====== Extended ASCII (aka. obs-text) ======
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

#[inline]
fn is_addr_token(b: u8) -> bool {
    ADDR_MAP[b as usize]
}
enum Status {
    None,
    IndicatorBegin(usize),
}

impl Status {
    #[inline]
    fn is_none(&self) -> bool {
        matches!(*self, Self::None)
    }
}

fn parse(address: &[u8]) -> Result<MultiAddr, Error> {
    let mut addr = address.iter();
    let mut token: Status = Status::None;
    let mut paths = Vec::new();
    let addr_b = address.as_ptr();
    loop {
        let (c, c_off) = match addr.next() {
            Some(c) => unsafe {
                let c_ptr = c as *const u8;
                (c, c_ptr.offset_from(addr_b) as usize)
            },
            None => break,
        };
        if !is_addr_token(*c) {
            return Err(Error::InvalidToken);
        }
        token = match token {
            Status::None if *c == b'/' => continue,
            Status::None => Status::IndicatorBegin(c_off),
            Status::IndicatorBegin(beign) if *c == b'/' => {
                paths.push(Indicator::new(address, beign, c_off));
                Status::None
            }
            Status::IndicatorBegin(_) => {
                continue;
            }
        };
    }
    if let Status::IndicatorBegin(beign) = token {
        let idc = Indicator::new(address, beign, address.len());
        paths.push(idc);
    }
    Ok(MultiAddr {
        paths,
    })
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parse_drivers() {
        let s = "////driver/test".as_bytes();
        let res = parse(s).unwrap();
        assert!(res.paths[0].value() == b"driver");
        assert!(res.paths[1].value() == b"test");
    }

    #[test]
    fn parse_drivers2() {
        let s = "////".as_bytes();
        let res = parse(s).unwrap();
        assert!(res.paths.len() == 0);
    }

    #[test]
    fn parse_drivers3() {
        let s = "////2".as_bytes();
        let res = parse(s).unwrap();
        assert!(res.paths[0].value() == b"2");
    }
}
