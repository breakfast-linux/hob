pub use hob_derive::ObjectTraversal;
use std::collections::HashMap;
use std::hash::Hash;

pub trait ObjectTraversal {
    fn traverse<T: ObjectWalker>(&mut self, walker: &mut T);
}

pub trait ObjectWalker {
    fn enter_string(&mut self, value: &mut String);
}

impl<T: ObjectTraversal> ObjectTraversal for Vec<T> {
    fn traverse<W: ObjectWalker>(&mut self, walker: &mut W) {
        for item in self {
            item.traverse(walker);
        }
    }
}

impl<T: ObjectTraversal> ObjectTraversal for Option<T> {
    fn traverse<W: ObjectWalker>(&mut self, walker: &mut W) {
        if let Some(v) = self {
            v.traverse(walker);
        }
    }
}

impl ObjectTraversal for String {
    fn traverse<W: ObjectWalker>(&mut self, walker: &mut W) {
        walker.enter_string(self);
    }
}

impl ObjectTraversal for usize {
    fn traverse<T: ObjectWalker>(&mut self, _: &mut T) {}
}

impl ObjectTraversal for bool {
    fn traverse<T: ObjectWalker>(&mut self, _: &mut T) {}
}

impl<K: ObjectTraversal + Eq + Hash, V: ObjectTraversal> ObjectTraversal for HashMap<K, V> {
    fn traverse<T: ObjectWalker>(&mut self, walker: &mut T) {
        let mut new_map = HashMap::new();

        for (mut k, mut v) in self.drain() {
            k.traverse(walker);
            v.traverse(walker);
            new_map.insert(k, v);
        }

        *self = new_map;
    }
}
