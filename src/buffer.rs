use std::cell::RefCell;
use std::rc::Rc;
use std::io;

use std::io::{Read,Write};

#[derive(Clone)]
pub struct Buffer {
    buf: Rc<RefCell<Vec<u8>>>
}

impl Buffer {
    pub fn new() -> Buffer {
        Buffer { buf:Rc::new(RefCell::new(vec![0; 64 * 1024])) }
    }
    pub fn read(&mut self, reader: &mut Read) -> Result<usize,io::Error> {
        let mut buffer = self.buf.borrow_mut();
        reader.read(&mut buffer)
    }
    pub fn write(&mut self, writer: &mut Write, n:usize) -> Result<usize,io::Error> {
        let buffer = self.buf.borrow();
        writer.write(&buffer[..n])
    }
}



