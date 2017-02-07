use std::ptr;
use std::mem;

#[derive(Debug)]
#[repr(C, packed)]
struct Example<T, S> {
    field: u8,
    field2: u8,
    field3: T,
    field4: S
}

fn main() {

    let buf : Vec<u8> = vec![0x01,
                             0x02,
                             0x01, 0x02,
                             0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

    let example = match buf[1]{
        2 => {
           let e : Example<u16,u64>; // = Example::new(0,0);
           println!("Size: {:?}", mem::size_of::<Example<u16,u64>>());
           println!("Size: {:?}", mem::size_of_val(&buf));
           unsafe{
                e = ptr::read(buf.as_ptr() as *const Example<u16, u64>)
           }
           Some(e)

        },

        _ => {
            None
        }
    };

    println!("Example: {:?}", example);
}
