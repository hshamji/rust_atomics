use std::net::Ipv6MulticastScope::RealmLocal;
use std::ptr::NonNull;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::sync::atomic::{fence, AtomicUsize};

struct ArcData<T> {
    ref_count: AtomicUsize,
    data: T,
}

pub struct Arc<T> {
    ptr: NonNull<ArcData<T>>,
}

unsafe impl<T: Send + Sync> Send for Arc<T> {}
unsafe impl<T: Send + Sync> Sync for Arc<T> {}

impl<T> Arc<T> {
    pub fn new(data: T) -> Arc<T> {
        Arc {
            ptr: NonNull::from(Box::leak(Box::new(ArcData {
                ref_count: AtomicUsize::new(1),
                data,
            }))),
        }
    }

    fn data(&self) -> &ArcData<T> {
        unsafe { self.ptr.as_ref() }
    }

    pub fn get_mut(arc: &mut Self) -> Option<&mut T> {
        if arc.data().ref_count.load(Relaxed) == 1 {
            fence(Acquire);
            unsafe { Some(&mut arc.ptr.as_mut().data) }
        } else {
            None
        }
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        self.data().ref_count.fetch_add(1, Relaxed);
        Arc { ptr: self.ptr }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Release) == 1 {
            fence(Acquire);
            unsafe {
                drop(Box::from_raw(self.ptr.as_ptr()));
            }
        }
    }
}

#[test]
fn test() {
    static NUM_DROPS: AtomicUsize = AtomicUsize::new(0);

    struct DetectDrop;

    impl Drop for DetectDrop {
        fn drop(&mut self) {
            NUM_DROPS.fetch_add(1, Relaxed);
        }
    }

    let x = Arc::new(("hello", DetectDrop));
    let y = x.clone();

    let t = std::thread::spawn(move || {
        // let x2 =
        assert_eq!(x.data().data.0, "hello");
    });

    assert_eq!(y.data().data.0, "hello");

    t.join().unwrap();

    assert_eq!(NUM_DROPS.load(Relaxed), 0);

    drop(y);

    assert_eq!(NUM_DROPS.load(Relaxed), 1);
}

#[derive(Debug)]
struct Test {
    a: i32,
}

impl Test {
    fn consume(self) -> Self {
        Test { a: self.a }
    }

    fn get_mutref(&mut self) {
        self.a = 321;
    }
}

impl Copy for Test {}

impl Clone for Test {
    fn clone(&self) -> Self {
        Test { a: self.a + 1 }
    }
}

fn main() {
    let b = Test { a: 123 };
    let a = b.consume();
    println!("Output {a:?}");

    let a2 = a.clone();
    println!("Cloned a: {a2:?}");
}
