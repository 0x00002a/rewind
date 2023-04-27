pub struct Atom<T, R, Act: FnOnce(&mut T) -> R, Undo: FnOnce(&mut T)> {
    val: T,
    act: Option<Act>,
    undo: Option<Undo>,
}

impl<T, R, Act: FnOnce(&mut T) -> R, Undo: FnOnce(&mut T)> Atom<T, R, Act, Undo> {
    pub fn new(val: T, act: Act, undo: Undo) -> Self {
        Self {
            val: val,
            act: Some(act),
            undo: Some(undo),
        }
    }
    pub fn eval(mut self) -> R {
        self.act.take().unwrap()(&mut self.val)
    }
    pub fn cancel(mut self) {
        self.undo.take();
    }
}
impl<T, R, Act: FnOnce(&mut T) -> R, Undo: FnOnce(&mut T)> Drop for Atom<T, R, Act, Undo> {
    fn drop(&mut self) {
        if let Some(undo) = self.undo.take() {
            undo(&mut self.val);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atom_runs_on_drop_if_uncancelled() {
        let mut scoped = 12;
        {
            let atom = Atom::new(
                &mut scoped,
                |v| {
                    **v += 2;
                },
                |v| {
                    **v = 0;
                },
            );
            drop(atom);
        }
        assert_eq!(scoped, 0);
    }

    #[test]
    fn cancelling_atom_stops_it_running_on_drop() {
        let mut scoped = 12;
        {
            let atom = Atom::new(
                &mut scoped,
                |s| {
                    **s = 13;
                },
                |s| {
                    **s = 0;
                },
            );
            atom.cancel();
        }
        assert_eq!(scoped, 12);
    }
}
