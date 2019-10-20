use std::borrow::Borrow;

pub(crate) trait RefToString {
    fn ref_to_string(&self) -> String;
}

impl<T: Borrow<str>> RefToString for T {
    #[inline]
    fn ref_to_string(&self) -> String {
        self.borrow().into()
    }
}
