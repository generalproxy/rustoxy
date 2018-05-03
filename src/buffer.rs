use std::cell::RefCell;
use std::rc::Rc;
use std::io;

use std::io::{Read,Write};

pub trait Buffer {
    fn read(&mut self, reader: &mut Read) -> Result<usize,io::Error>;
    fn write(&mut self, writer: &mut Write, n:usize) -> Result<usize,io::Error>;
}

#[derive(Clone)]
pub struct RcBuffer {
    buf: Rc<RefCell<Vec<u8>>>
}

impl RcBuffer {
    pub fn new() -> RcBuffer {
        RcBuffer { buf:Rc::new(RefCell::new(vec![0; 64 * 1024])) }
    }
}
impl Buffer for RcBuffer {
    fn read(&mut self, reader: &mut Read) -> Result<usize,io::Error> {
        let mut buffer = self.buf.borrow_mut();
        reader.read(&mut buffer)
    }
    fn write(&mut self, writer: &mut Write, n:usize) -> Result<usize,io::Error> {
        let buffer = self.buf.borrow();
        writer.write(&buffer[..n])
    }
}



