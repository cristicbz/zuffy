use lazy::Lazy;
use std::str::{MaybeOwned, IntoMaybeOwned, Slice, Owned};

mod lazy;

#[deriving(Eq, PartialEq, Show)]
enum ErrorCode {
    DeadlineExceeded,
    NetworkError,
    InternalServerError,
}

struct Error {
    code: ErrorCode,
    desc: Lazy<MaybeOwned<'static>>,
}
impl Error {
    pub fn new(code: ErrorCode) -> Error {
        Error::with_desc(code, "")
    }

    pub fn with_desc<T: IntoMaybeOwned<'static>>(code: ErrorCode, desc: T)
            -> Error {
        Error {
            code: code,
            desc: Lazy::from_value(desc.into_maybe_owned())
        }
    }

    pub fn with_lazy_desc<T: IntoMaybeOwned<'static>>(
            code: ErrorCode, desc: proc():Send -> T) -> Error {
        Error {
            code: code,
            desc: Lazy::from_fn(proc() desc().into_maybe_owned()),
        }
    }

    pub fn code(&self) -> ErrorCode { self.code }
    pub fn desc(&self) -> &str {
        match self.desc.get() {
            &Slice(s) => s,
            &Owned(ref s) => s.as_slice(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Error, InternalServerError, DeadlineExceeded};

    #[test]
    fn test_no_desc() {
        let err = Error::new(InternalServerError);
        assert_eq!(err.code(), InternalServerError);
        assert_eq!(err.desc(), "");
    }

    #[test]
    fn test_slice_desc() {
        let err = Error::with_desc(DeadlineExceeded, "failwhale");
        assert_eq!(err.code(), DeadlineExceeded);
        assert_eq!(err.desc(), "failwhale");
    }

    #[test]
    fn test_string_desc() {
        let err = Error::with_desc(InternalServerError, "failboat".to_string());
        assert_eq!(err.code(), InternalServerError);
        assert_eq!(err.desc(), "failboat");
    }

    #[test]
    fn test_lazy_slice_desc() {
        let err = Error::with_lazy_desc(InternalServerError, proc() "failboat");
        assert_eq!(err.code(), InternalServerError);
        assert_eq!(err.desc(), "failboat");
    }

    #[test]
    fn test_lazy_string_desc() {
        let err = Error::with_lazy_desc(
            InternalServerError, proc() "failboat".to_string());
        assert_eq!(err.code(), InternalServerError);
        assert_eq!(err.desc(), "failboat");
    }
}
