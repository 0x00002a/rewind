use crate::atom::Atom;

#[derive(Default)]
pub struct Stack {
    atoms: Vec<Box<dyn Atom>>,
}
impl Stack {
    pub fn push(&mut self, atom: impl Atom + 'static) -> &mut Self {
        self.atoms.push(Box::new(atom));
        self
    }
    pub fn pop(&mut self) -> Option<Box<dyn Atom>> {
        self.atoms.pop()
    }
    pub fn new() -> Self {
        Self::default()
    }
}
impl Drop for Stack {
    fn drop(&mut self) {
        self.cancel()
    }
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
    fn undo(&mut self) {
        for atom in &mut self.atoms {
            atom.undo();
        }
    }

    fn cancel(&mut self) {
        for atom in &mut self.atoms {
            atom.cancel();
        }
    }
}
