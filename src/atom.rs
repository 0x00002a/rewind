use std::{
    borrow::{Borrow, BorrowMut},
    cell::{Cell, RefCell},
    marker::PhantomData,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    rc::Rc,
};

/// Carries a value with an undo action. If dropped without a call to `Atom::cancel` the undo
/// action is called. See `rewind::atom` for usage examples
pub struct Simple<T, R, Undo: FnOnce(T) -> R> {
    val: ManuallyDrop<T>,
    undo: Option<ManuallyDrop<Undo>>,
}

impl<T, R, Undo: FnOnce(T) -> R> Simple<T, R, Undo> {
    pub(crate) fn new(val: T, undo: Undo) -> Self {
        Self {
            val: ManuallyDrop::new(val),
            undo: Some(ManuallyDrop::new(undo)),
        }
    }
}

impl<T, R, Undo: FnOnce(T) -> R> Atom for Simple<T, R, Undo> {
    type Undo = R;
    type Cancel = T;
    fn undo(mut self) -> Self::Undo {
        ManuallyDrop::into_inner(self.undo.take().unwrap())(ManuallyDrop::into_inner(self.val))
    }
    fn cancel(mut self) -> Self::Cancel {
        ManuallyDrop::into_inner(self.val)
    }
}

impl<T, R, Undo: FnOnce(T) -> R> Drop for Simple<T, R, Undo> {
    fn drop(&mut self) {
        self.undo();
    }
}

pub struct Owning<T, Undo: FnOnce(T) -> T> {
    val: Simple<T, T, Undo>,
    stored: ManuallyDrop<T>,
}
impl<T, Undo: FnOnce(T) -> T> Owning<T, Undo> {
    pub(crate) fn new(val: T, undo: Undo) -> Self
    where
        T: Clone,
    {
        Self {
            val: Simple::new(val.clone(), undo),
            stored: ManuallyDrop::new(val),
        }
    }
    pub fn get(&self) -> &T {
        &self.stored
    }
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.stored
    }
    /// Consume the atom and return the modified values
    ///
    /// This is in contrast to [`Atom::cancel`] which returns the _unmodified_ value
    /// as otherwise it would have to clone
    ///
    /// Example usage:
    /// ```
    /// use rewind::Atom;
    /// let mut items = rewind::own(vec!["hello", "world"], rewind::id);
    /// items.push("wow");
    /// let items = items.into_inner();
    /// assert_eq!(items.get(2), Some(&"wow"));
    /// ```
    pub fn into_inner(mut self) -> T {
        self.cancel();
        let stored = unsafe { ManuallyDrop::take(&mut self.stored) };
        stored
    }
}

impl<T, Undo: FnOnce(T) -> T> Drop for Owning<T, Undo> {
    fn drop(&mut self) {
        self.undo();
    }
}

impl<T, Undo: FnOnce(T) -> T> Deref for Owning<T, Undo> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.stored
    }
}

impl<T, Undo: FnOnce(T) -> T> DerefMut for Owning<T, Undo> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.stored
    }
}

impl<T, Undo: FnOnce(T) -> T> Atom for Owning<T, Undo> {
    type Undo = T;
    type Cancel = T;
    fn undo(mut self) -> Self::Undo {
        self.val.undo()
    }

    fn cancel(mut self) -> Self::Cancel {
        self.val.cancel();
        let stored = unsafe { ManuallyDrop::take(&mut self.stored) };
        stored
    }
}

struct SideEffectInner<T, Undo> {
    value: T,
    undo: Undo,
}
pub struct Encased<S>(Rc<RefCell<S>>);

pub struct SideEffect<T, S, Undo: FnOnce(&mut S, T)> {
    undo: Option<ManuallyDrop<Undo>>,
    value: ManuallyDrop<T>,
    parent: Encased<S>,
}
impl<S> Encased<S> {
    pub fn peel_mut<R, U: FnOnce(&mut S, R)>(
        &mut self,
        act: impl FnOnce(&mut S) -> R,
        undo: U,
    ) -> SideEffect<R, S, U> {
        let stored = act(&mut (*self.0).borrow_mut());
        SideEffect::with_parent(stored, undo, Encased(self.0.clone()))
    }
    pub(crate) fn new(s: S) -> Self {
        Self(Rc::new(RefCell::new(s)))
    }
}
impl<S> Deref for Encased<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(*self.0).as_ptr() }
    }
}
impl<S> DerefMut for Encased<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(*self.0).as_ptr() }
    }
}

impl<T, S, Undo: FnOnce(&mut S, T)> SideEffect<T, S, Undo> {
    fn with_parent(value: T, undo: Undo, parent: Encased<S>) -> Self {
        Self {
            undo: Some(ManuallyDrop::new(undo)),
            value: ManuallyDrop::new(value),
            parent,
        }
    }
    pub(crate) fn new(value: T, undo: Undo, parent: S) -> Self {
        Self::with_parent(value, undo, Encased::new(parent))
    }
    pub fn peel_mut<R, U: FnOnce(&mut S, R)>(
        &mut self,
        act: impl FnOnce(&mut S) -> R,
        undo: U,
    ) -> SideEffect<R, S, U> {
        self.parent.peel_mut(act, undo)
    }
}
impl<T, S, Undo: FnOnce(&mut S, T)> Deref for SideEffect<T, S, Undo> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
impl<T, S, Undo: FnOnce(&mut S, T)> DerefMut for SideEffect<T, S, Undo> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}
impl<T, S, Undo: FnOnce(&mut S, T)> Drop for SideEffect<T, S, Undo> {
    fn drop(&mut self) {
        if let Some(undo) = &mut self.undo {
            let value = unsafe { ManuallyDrop::take(&mut self.value) };
            let undo = unsafe { ManuallyDrop::take(undo) };
            undo(&mut self.parent, value);
        }
    }
}

/// An undo action that can be cancelled
///
/// Implementors should implement [`Drop`] as `self.cancel();`, although unfortunately since [`Drop`] cannot
/// be implemented on a generic this cannot be enforced
#[allow(drop_bounds)]
pub trait Atom: Drop {
    type Undo;
    type Cancel;
    /// Undo the operation
    fn undo(self) -> Self::Undo;
    /// Cancel the operation
    ///
    /// After this call, calls to [`Atom::undo`] are not required to actually do anything
    ///
    /// Example usage:
    /// ```
    /// use rewind::Atom;
    /// let mut items = rewind::own(vec!["hello", "world"], rewind::id);
    /// items.push("wow");
    /// let items = items.cancel().unwrap();
    /// assert_eq!(items.len(), 2);
    /// ```
    /// Note how the length of the items is 2 at the end, this is because for [`rewind::atom::Owning`] this function
    /// must return the unmodified value (as otherwise it would have to clone)
    ///
    fn cancel(self) -> Self::Cancel;
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn atom_runs_on_drop_if_uncancelled() {
        let mut scoped = 12;
        {
            let atom = Simple::new(&mut scoped, |v| {
                *v = 0;
            });
            drop(atom);
        }
        assert_eq!(scoped, 0);
    }

    #[test]
    fn cancelling_atom_stops_it_running_on_drop() {
        let mut scoped = 12;
        {
            let mut atom = Simple::new(&mut scoped, |s| {
                *s = 13;
            });
            atom.cancel();
        }
        assert_eq!(scoped, 12);
    }
}
