use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use yew::html::IntoPropValue;

#[derive(Default)]
pub struct ArcPtrEq<T: ?Sized>(Arc<T>);

impl<T> From<T> for ArcPtrEq<T> {
    fn from(x: T) -> Self {
        Self(Arc::new(x))
    }
}

impl<T: ?Sized> From<Arc<T>> for ArcPtrEq<T> {
    fn from(x: Arc<T>) -> Self {
        Self(x)
    }
}

impl<T: ?Sized> IntoPropValue<ArcPtrEq<T>> for Arc<T> {
    fn into_prop_value(self) -> ArcPtrEq<T> {
        ArcPtrEq(self)
    }
}

impl<T: ?Sized> Clone for ArcPtrEq<T> {
    fn clone(&self) -> Self {
        ArcPtrEq(self.0.clone())
    }
}

impl<T: ?Sized> PartialEq for ArcPtrEq<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: ?Sized> Deref for ArcPtrEq<T> {
    type Target = Arc<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ?Sized> DerefMut for ArcPtrEq<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
