pub trait ReadRemain {
    fn remain(&self) -> usize {
        self.as_bytes_ref()
            .map_or(0, |b| b.len() - self.read_point())
    }

    fn as_bytes_ref(&self) -> Option<&[u8]>;

    fn read_point(&self) -> usize;

    fn set_read_point(&mut self, point: usize);

    fn copy_remain(&mut self, buf: &mut [u8]) -> usize {
        let remain = self.remain();
        if remain == 0 {
            return 0;
        }
        let size = if remain <= buf.len() {
            remain
        } else {
            buf.len()
        };
        let read_p = self.read_point();
        let size = self.as_bytes_ref().map_or(0, |body| {
            buf[..size].copy_from_slice(&body[read_p..(read_p + size)]);
            size
        });
        if size > 0 {
            self.set_read_point(read_p + size);
        }
        size
    }
}
