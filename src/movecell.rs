use std::cell::UnsafeCell;
use std::mem;
use std::kinds::marker;

pub struct MoveCell<T> {
    value: UnsafeCell<Option<T>>,
    _nosync: marker::NoSync,
}

impl<T> MoveCell<T> {
    pub fn new() -> MoveCell<T> {
        MoveCell {
            value: UnsafeCell::new(None),
            _nosync: marker::NoSync,
        }
    }

    pub fn from_value(value: T) -> MoveCell<T> {
        MoveCell {
            value: UnsafeCell::new(Some(value)),
            _nosync: marker::NoSync,
        }
    }

    pub fn put(&self, value: T) -> Option<T> {
        unsafe { mem::replace(&mut *self.value.get(), Some(value)) }
    }

    pub fn get_ref(&self) -> Option<&T> {
        unsafe { (*self.value.get()).as_ref() }
    }

    pub fn take(&self) -> Option<T> {
        unsafe { (*self.value.get()).take() }
    }

    pub fn empty(&self) -> bool {
        unsafe { (*self.value.get()).is_none() }
    }
}

#[cfg(test)]
mod test {
    use super::MoveCell;

    #[test]
    fn test_copy() {
        let x = MoveCell::from_value(10u);

        assert_eq!(x.take().unwrap(), 10u);
        assert!(x.take().is_none());
        assert!(x.empty());

        assert!(x.put(15u).is_none());
        assert_eq!(x.take().unwrap(), 15u);
        assert!(x.take().is_none());
        assert!(x.empty());
    }

    #[test]
    fn test_nocopy() {
        let x = MoveCell::from_value("a".to_string());

        assert_eq!(x.take().unwrap().as_slice(), "a");
        assert!(x.take().is_none());
        assert!(x.empty());

        assert!(x.put("b".to_string()).is_none());
        assert_eq!(x.take().unwrap().as_slice(), "b");
        assert!(x.take().is_none());
        assert!(x.empty());
    }

    #[test]
    fn test_empty_new() {
        let x = MoveCell::<String>::new();
        assert!(x.empty());
        assert!(x.put("a".to_string()).is_none());
        assert_eq!(x.take().unwrap().as_slice(), "a");
        assert!(x.take().is_none());
        assert!(x.empty());
    }
}
