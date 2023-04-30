use std::{
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
};

/// Carries a value with an undo action. If dropped without a call to `Atom::cancel` the undo
/// action is called. See `rewind::atom` for usage examples
pub struct ValAtom<T, R, Undo: FnOnce(T) -> R> {
    val: Option<T>,
    undo: Option<Undo>,
}

impl<T, R, Undo: FnOnce(T) -> R> ValAtom<T, R, Undo> {
    pub(crate) fn new(val: T, undo: Undo) -> Self {
        Self {
            val: Some(val),
            undo: Some(undo),
        }
    }
}

impl<T, R, Undo: FnOnce(T) -> R> Atom for ValAtom<T, R, Undo> {
    type Undo = Option<R>;
    type Cancel = Option<T>;
    fn undo(&mut self) -> Self::Undo {
        if let Some(undo) = self.undo.take() {
            Some(undo(self.val.take().unwrap()))
        } else {
            None
        }
    }
    fn cancel(&mut self) -> Self::Cancel {
        self.undo.take();
        self.val.take()
    }
}

impl<T, R, Undo: FnOnce(T) -> R> Drop for ValAtom<T, R, Undo> {
    fn drop(&mut self) {
        self.undo();
    }
}

pub struct StoreAtom<T, Undo: FnOnce(T) -> T> {
    val: ValAtom<T, T, Undo>,
    stored: ManuallyDrop<T>,
}
impl<T, Undo: FnOnce(T) -> T> StoreAtom<T, Undo> {
    pub(crate) fn new(val: T, undo: Undo) -> Self
    where
        T: Clone,
    {
        Self {
            val: ValAtom::new(val.clone(), undo),
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

impl<T, Undo: FnOnce(T) -> T> Drop for StoreAtom<T, Undo> {
    fn drop(&mut self) {
        self.undo();
    }
}

impl<T, Undo: FnOnce(T) -> T> Deref for StoreAtom<T, Undo> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.stored
    }
}

impl<T, Undo: FnOnce(T) -> T> DerefMut for StoreAtom<T, Undo> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.stored
    }
}

impl<T, Undo: FnOnce(T) -> T> Atom for StoreAtom<T, Undo> {
    type Undo = ();
    type Cancel = Option<T>;
    fn undo(&mut self) -> Self::Undo {
        if let Some(v) = self.val.undo() {
            self.stored = ManuallyDrop::new(v);
        }
    }

    fn cancel(&mut self) -> Self::Cancel {
        self.val.cancel()
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
    fn undo(&mut self) -> Self::Undo;
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
    /// Note how the length of the items is 2 at the end, this is because for [`StoreAtom`] this function
    /// must return the unmodified value (as otherwise it would have to clone)
    ///
    fn cancel(&mut self) -> Self::Cancel;
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn atom_runs_on_drop_if_uncancelled() {
        let mut scoped = 12;
        {
            let atom = ValAtom::new(&mut scoped, |v| {
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
            let mut atom = ValAtom::new(&mut scoped, |s| {
                *s = 13;
            });
            atom.cancel();
        }
        assert_eq!(scoped, 12);
    }
}
