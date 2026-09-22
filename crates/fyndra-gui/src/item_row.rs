use fyndra_core::FileEntry;
use glib::subclass::prelude::*;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct FileObject {
        pub entry: RefCell<Option<FileEntry>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileObject {
        const NAME: &'static str = "FyndraFileObject";
        type Type = super::FileObject;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for FileObject {}
}

glib::wrapper! {
    pub struct FileObject(ObjectSubclass<imp::FileObject>);
}

impl FileObject {
    pub fn new(entry: FileEntry) -> Self {
        let obj: Self = glib::Object::builder().build();
        *obj.imp().entry.borrow_mut() = Some(entry);
        obj
    }

    pub fn entry(&self) -> Option<FileEntry> {
        self.imp().entry.borrow().clone()
    }
}
