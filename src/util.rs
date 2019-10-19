pub(crate) trait RefToString {
    fn ref_to_string(&self) -> String;
}

impl<T: AsRef<str>> RefToString for T {
    #[inline]
    fn ref_to_string(&self) -> String {
        AsRef::<str>::as_ref(self).into()
    }
}
