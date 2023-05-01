use std::any::Any;

use crate::atom::Atom;

#[repr(transparent)]
pub struct StackAtom<A>(A);
impl<C: Any, U: Any, A: Atom<Cancel = C, Undo = U>> Atom for StackAtom<A> {
    type Undo = Box<dyn Any>;
    type Cancel = Box<dyn Any>;

    fn undo(self) -> Self::Undo {
        Box::new(self.0.undo())
    }

    fn cancel(self) -> Self::Cancel {
        Box::new(self.0.cancel())
    }
}
impl<A> Drop for StackAtom<A> {
    fn drop(&mut self) {
        drop(&mut self.0)
    }
}

type StackEl = Box<dyn Atom<Cancel = Box<dyn Any>, Undo = Box<dyn Any>>>;
#[derive(Default)]
pub struct Stack {
    atoms: Vec<StackEl>,
}
impl Stack {
    pub fn push(&mut self, atom: impl Atom + 'static) -> &mut Self {
        self.atoms.push(Box::new(StackAtom(atom)));
        self
    }
    pub fn pop(&mut self) -> Option<StackEl> {
        self.atoms.pop()
    }
    pub fn new() -> Self {
        Self::default()
    }
}
impl Drop for Stack {
    fn drop(&mut self) {}
}

pub trait StackedAtom: Atom + Sized + 'static {
    fn chain<O: StackedAtom + 'static>(self, other: O) -> Stack {
        let mut s = Stack::new();
        s.push(self);
        s.push(other);
        s
    }
}
impl<A: Atom + 'static + Sized> StackedAtom for A {}

impl Atom for Stack {
    type Undo = Vec<Box<dyn Any>>;
    type Cancel = Vec<Box<dyn Any>>;
    fn undo(self) -> Self::Undo {
        self.atoms.into_iter().map(|a| a.undo()).collect()
    }

    fn cancel(self) -> Self::Cancel {
        self.atoms.into_iter().map(|a| a.cancel()).collect()
    }
}
