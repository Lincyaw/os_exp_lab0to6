
use buddy_system_allocator::Heap;
fn main() {
    println!("Hello, world!");
}


#[test]
fn test1(){
    let mut a = Heap::new();
    unsafe {
        a.add_to_heap(2,4);
        println!("{:?}",a);
    }


}