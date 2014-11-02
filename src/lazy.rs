use std::cell::UnsafeCell;
use std::kinds::marker;

enum LazyState<T> {
    Deferred(proc():Send -> T),
    Evaluating,
    Cached(T),
}

pub struct Lazy<T> {
    state: UnsafeCell<LazyState<T>>,
    _nosync: marker::NoSync,
}

impl<T> Lazy<T> {
    pub fn from_value(value: T) -> Lazy<T> {
        Lazy {
            state: UnsafeCell::new(Cached(value)),
            _nosync: marker::NoSync,
        }
    }

    pub fn from_fn(generator: proc():Send -> T) -> Lazy<T> {
        Lazy {
            state: UnsafeCell::new(Deferred(generator)),
            _nosync: marker::NoSync,
        }
    }

    pub fn unwrap(self) -> T {
        match unsafe { self.state.unwrap() } {
            Cached(value) => value,
            Deferred(gen) => gen(),
            Evaluating => unreachable!(),
        }
    }

    pub fn get(&self) -> &T {
        match unsafe { &*self.state.get() } {
            &Cached(ref value) => value,
            &Evaluating => panic!("Recursive Lazy::get()."),
            &Deferred(_) => unsafe { self.evaluate() }
        }
    }

    unsafe fn evaluate(&self) -> &T {
        use std::mem::replace;
        match replace(&mut *self.state.get(), Evaluating) {
            Deferred(gen) => {
                *self.state.get() = Cached(gen());
                self.get()
            },
            Cached(_) | Evaluating => unreachable!(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::Lazy;

    #[test]
    fn test_copy_value_unwrap() {
        const VALUE : u32 = 10;
        let x = Lazy::from_value(VALUE);
        assert_eq!(x.unwrap(), VALUE);
    }

    #[test]
    fn test_copy_value_get() {
        const VALUE : u32 = 10;
        let x = Lazy::from_value(VALUE);
        let a = x.get();
        let b = x.get();
        assert_eq!(a, b);
        assert_eq!(a as *const _, b as *const _);
    }

    #[test]
    fn test_nocopy_value_unwrap() {
        let value = "foobar".to_string();
        let x = Lazy::from_value(value.clone());
        assert_eq!(x.unwrap(), value);
    }

    #[test]
    fn test_nocopy_value_get() {
        let value = "foobar".to_string();
        let x = Lazy::from_value(value.clone());
        let a = x.get();
        let b = x.get();
        assert_eq!(a, b);
        assert_eq!(a as *const _, b as *const _);
        assert_eq!(*a, value);
    }

    #[test]
    fn test_fn_unwrap() {
        let x = Lazy::from_fn(proc() format!("{} {}", "A", "B"));
        assert_eq!(x.unwrap().as_slice(), "A B");
    }

    #[test]
    fn test_fn_get() {
        let x = Lazy::from_fn(proc() format!("{} {}", "A", "B"));
        let a = x.get();
        let b = x.get();
        assert_eq!(a, b);
        assert_eq!(a as *const _, b as *const _);
        assert_eq!(a.as_slice(), "A B");
    }
}
