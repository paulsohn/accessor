#![no_std]

use accessor::array::{BoundSetGeneric, BoundSetGenericMut, BoundSetGenericOf};

#[repr(C)]
#[derive(Clone, Copy, BoundSetGenericOf)]
struct Foo {
    x: u32,
    y: u32,
}

use accessor::mapper::Mapper;
use core::num::NonZeroUsize;

struct M;
impl Mapper for M {
    unsafe fn map(&mut self, phys_start: usize, bytes: usize) -> NonZeroUsize {
        todo!()
    }
    fn unmap(&mut self, phys_start: usize, bytes: usize) {
        todo!()
    }
}

fn main() {
    let mut a = unsafe { accessor::array::ReadWrite::<Foo, M>::new(0x1000, 10, M) };

    // read `x` field of 0th element of the array.
    let x = a.set_at(0).x.read_volatile();

    // write 5 as the `y` field of 2nd element of the array.
    a.set_at_mut(2).y.write_volatile(5);

    a.at_mut(1).update_volatile(|k| {
        k.y = 12;
    });
}

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    loop {}
}
