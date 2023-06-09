use std::{
    cell::RefCell,
    fmt::Debug,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    rc::Rc,
};

/// Carries a value with an undo action
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
    fn undo_mut(&mut self) -> Option<R> {
        if let Some(mut undo) = self.undo.take() {
            Some(unsafe { ManuallyDrop::take(&mut undo)(ManuallyDrop::take(&mut self.val)) })
        } else {
            None
        }
    }
}

impl<T, R, Undo: FnOnce(T) -> R> Atom for Simple<T, R, Undo> {
    type Undo = R;
    type Decay = T;
    /// Returns the result of the undo function
    ///
    /// ```
    /// # use rewind::Atom;
    /// let v = rewind::simple(4, |v| v + 2);
    /// assert_eq!(v.undo(), 6);
    /// ```
    fn undo(mut self) -> Self::Undo {
        self.undo_mut().unwrap()
    }
    /// Returns the original value
    ///
    /// ```
    /// # use rewind::Atom;
    /// let v = rewind::simple(4, |v| v + 2);
    /// assert_eq!(v.decay(), 4);
    /// ```
    fn decay(mut self) -> Self::Decay {
        self.undo.take().map(|u| ManuallyDrop::into_inner(u));
        unsafe { ManuallyDrop::take(&mut self.val) }
    }
}

impl<T, R, Undo: FnOnce(T) -> R> Drop for Simple<T, R, Undo> {
    fn drop(&mut self) {
        self.undo_mut();
    }
}
impl<T: Debug, R, Undo: FnOnce(T) -> R> Debug for Simple<T, R, Undo> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Simple").field("value", &self.val).finish()
    }
}

/// Caries an undo operation + an owned mutable value
///
/// See [`own`](rewind::own) for examples
pub struct Owning<T, Undo: FnOnce(T) -> T> {
    val: Option<ManuallyDrop<Simple<T, T, Undo>>>,
    stored: ManuallyDrop<T>,
}
impl<T, Undo: FnOnce(T) -> T> Owning<T, Undo> {
    pub(crate) fn new(val: T, undo: Undo) -> Self
    where
        T: Clone,
    {
        Self {
            val: Some(ManuallyDrop::new(Simple::new(val.clone(), undo))),
            stored: ManuallyDrop::new(val),
        }
    }
    pub fn get(&self) -> &T {
        &self.stored
    }
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.stored
    }
    fn undo_mut(&mut self) -> Option<T> {
        self.val
            .take()
            .map(|mut val| unsafe { ManuallyDrop::take(&mut val) }.undo())
    }
}

impl<T, Undo: FnOnce(T) -> T> Drop for Owning<T, Undo> {
    fn drop(&mut self) {
        self.undo_mut();
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
impl<T: Debug, Undo: FnOnce(T) -> T> Debug for Owning<T, Undo> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Owning")
            .field("inner", &self.val)
            .field("mutable", &self.stored)
            .finish()
    }
}

impl<T, Undo: FnOnce(T) -> T> Atom for Owning<T, Undo> {
    type Undo = T;
    type Decay = T;
    fn undo(mut self) -> Self::Undo {
        self.undo_mut().unwrap()
    }

    /// Returns the modified part
    ///
    /// ```
    /// # use rewind::Atom;
    /// let mut items = rewind::own_id(vec!["hello", "world"]);
    /// items.push("wow");
    /// let items = items.decay();
    /// assert_eq!(items.get(2), Some(&"wow"));
    /// ```
    fn decay(mut self) -> Self::Decay {
        unsafe { ManuallyDrop::take(&mut self.val.take().unwrap()) }.decay();

        unsafe { ManuallyDrop::take(&mut self.stored) }
    }
}

///
///
/// See [`encase`](rewind::encase) for usage details
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Default)]
#[repr(transparent)]
pub struct Encased<S>(Rc<RefCell<S>>);

impl<S> Clone for Encased<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// An operation that has side effects on a shared state
///
/// The shared state `S` is managed via [`Encased<S>`]
pub struct SideEffect<T, R, S, Undo: FnOnce(&mut S, T) -> R> {
    undo: Option<ManuallyDrop<Undo>>,
    value: ManuallyDrop<T>,
    parent: Encased<S>,
}
impl<S> Encased<S> {
    pub fn peel_mut<R, Ru, U: FnOnce(&mut S, R) -> Ru>(
        &mut self,
        act: impl FnOnce(&mut S) -> R,
        undo: U,
    ) -> SideEffect<R, Ru, S, U> {
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

impl<T, R, S, Undo: FnOnce(&mut S, T) -> R> SideEffect<T, R, S, Undo> {
    fn with_parent(value: T, undo: Undo, parent: Encased<S>) -> Self {
        Self {
            undo: Some(ManuallyDrop::new(undo)),
            value: ManuallyDrop::new(value),
            parent,
        }
    }
    pub fn peel_mut<Rv, Ru, U: FnOnce(&mut S, Rv) -> Ru>(
        &mut self,
        act: impl FnOnce(&mut S) -> Rv,
        undo: U,
    ) -> SideEffect<Rv, Ru, S, U> {
        self.parent.peel_mut(act, undo)
    }
}
impl<T, R, S, Undo: FnOnce(&mut S, T) -> R> Deref for SideEffect<T, R, S, Undo> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
impl<T, R, S, Undo: FnOnce(&mut S, T) -> R> DerefMut for SideEffect<T, R, S, Undo> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}
impl<T, R, S, Undo: FnOnce(&mut S, T) -> R> Drop for SideEffect<T, R, S, Undo> {
    fn drop(&mut self) {
        if let Some(undo) = &mut self.undo {
            let value = unsafe { ManuallyDrop::take(&mut self.value) };
            let undo = unsafe { ManuallyDrop::take(undo) };
            undo(&mut self.parent, value);
        }
    }
}
impl<T, S, R, Undo: FnOnce(&mut S, T) -> R> Atom for SideEffect<T, R, S, Undo> {
    type Undo = R;
    type Decay = T;

    fn undo(mut self) -> Self::Undo {
        let value = unsafe { ManuallyDrop::take(&mut self.value) };
        ManuallyDrop::into_inner(self.undo.take().unwrap())(&mut self.parent, value)
    }

    fn decay(mut self) -> Self::Decay {
        self.undo.take();
        unsafe { ManuallyDrop::take(&mut self.value) }
    }
}
impl<T: Debug, S: Debug, R, Undo: FnOnce(&mut S, T) -> R> Debug for SideEffect<T, R, S, Undo> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SideEffect")
            .field("value", &self.value)
            .field("parent", &self.parent)
            .finish()
    }
}

/// An undo action that can be cancelled
///
/// Implementors should implement [`Drop`] as `self.undo();`, although unfortunately since [`Drop`] cannot
/// be implemented on a generic this cannot be enforced
#[allow(drop_bounds)]
pub trait Atom: Drop {
    type Undo;
    type Decay;
    /// Undo the operation
    ///
    /// ```
    /// # use rewind::Atom;
    /// let mut items = rewind::own_id(vec!["hello", "world"]);
    /// items.push("wow");
    /// let items = items.undo();
    /// assert_eq!(items.len(), 2);
    /// ```
    fn undo(self) -> Self::Undo;
    /// Forget about how to undo
    ///
    /// ```
    /// # use rewind::Atom;
    /// let mut items = rewind::own_id(vec!["hello", "world"]);
    /// items.push("wow");
    /// let items = items.decay();
    /// assert_eq!(items.len(), 3);
    /// ```
    fn decay(self) -> Self::Decay;
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn simple_undo_does_not_cause_ub_on_drop() {
        Simple::new(vec!["test"], |c| c).undo();
    }
    #[test]
    fn owned_undo_does_not_cause_ub_on_drop() {
        Owning::new(vec!["test"], |c| c).undo();
    }

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
            let atom = Simple::new(&mut scoped, |s| {
                *s = 13;
            });
            atom.decay();
        }
        assert_eq!(scoped, 12);
    }
}
