pub trait NeqAssign {
    fn neq_assign(&mut self, new: Self) -> bool;
}

impl<T: PartialEq> NeqAssign for T {
    fn neq_assign(&mut self, new: T) -> bool {
        if self != &new {
            *self = new;
            true
        } else {
            false
        }
    }
}
