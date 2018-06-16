use std::marker::PhantomData;
use std::ptr::NonNull;

/// A doubly-linked list that provides unsafe access to inner nodes.
pub struct UnsafeLinkedList<T> {
    head: Option<NonNull<Node<T>>>,
    marker: PhantomData<Box<Node<T>>>,
}

/// A non-cloneable handle to a node in an `UnsafeLinkedList`.
pub struct NodeHandle<T> {
    ptr: NonNull<Node<T>>,
}

/// An iterator over the elements in an `UnsafeLinkedList`.
pub struct Iter<'a, T: 'a> {
    node: Option<NonNull<Node<T>>>,
    marker: PhantomData<&'a Node<T>>,
}

struct Node<T> {
    next: Option<NonNull<Node<T>>>,
    prev: Option<NonNull<Node<T>>>,
    elm: T,
}

impl<T> UnsafeLinkedList<T> {
    pub fn new() -> Self {
        UnsafeLinkedList {
            head: None,
            marker: PhantomData,
        }
    }

    pub fn push_front(&mut self, elm: T) -> NodeHandle<T> {
        self.push_front_node(Box::new(Node {
            prev: None,
            next: None,
            elm,
        }))
    }

    /// Remove a node from the list and returns the element.
    ///
    /// # Safety
    ///
    /// The node must belong to the list.
    pub unsafe fn remove(&mut self, node: NodeHandle<T>) -> T {
        self.remove_node(node).elm
    }

    /// Bumps up a node to the front of the list.
    ///
    /// # Safety
    ///
    /// The node must belong to the list.
    pub unsafe fn bump(&mut self, node: &NodeHandle<T>) {
        let node = self.remove_node(NodeHandle::clone(node));
        self.push_front_node(node);
    }

    /// Returns a reference to the element of the node.
    ///
    /// # Safety
    ///
    /// The node must belong to the list.
    pub unsafe fn get(&self, node: &NodeHandle<T>) -> &T {
        &(*node.ptr.as_ptr()).elm
    }

    pub unsafe fn get_mut(&mut self, node: &NodeHandle<T>) -> &mut T {
        &mut (*node.ptr.as_ptr()).elm
    }

    pub fn iter(&self) -> Iter<T> {
        Iter {
            node: self.head,
            marker: PhantomData,
        }
    }
}

/// Private interface.
impl<T> UnsafeLinkedList<T> {
    fn push_front_node(&mut self, mut node: Box<Node<T>>) -> NodeHandle<T> {
        node.next = self.head;
        node.prev = None;

        // TODO: replace with `Box::into_raw_non_null` once it's stable.
        let ptr = NonNull::from(&*node);
        Box::into_raw(node);

        if let Some(mut head) = self.head {
            unsafe {
                head.as_mut().prev = Some(ptr);
            }
        }

        self.head = Some(ptr);

        NodeHandle { ptr }
    }

    unsafe fn remove_node(&mut self, node: NodeHandle<T>) -> Box<Node<T>> {
        let node = Box::from_raw(node.ptr.as_ptr());
        let ptr = NonNull::from(&*node);

        if let Some(mut next) = node.next {
            debug_assert_eq!(Some(ptr), next.as_ref().prev);
            next.as_mut().prev = node.prev;
        }

        if let Some(mut prev) = node.prev {
            debug_assert_eq!(Some(ptr), prev.as_ref().next);
            prev.as_mut().next = node.next;
        } else {
            debug_assert_eq!(Some(ptr), self.head);
            self.head = node.next;
        }

        node
    }
}

impl<T> Drop for UnsafeLinkedList<T> {
    fn drop(&mut self) {
        while let Some(head) = self.head {
            unsafe {
                self.head = Box::from_raw(head.as_ptr()).next;
            }
        }
    }
}

unsafe impl<T: Send> Send for UnsafeLinkedList<T> {}
unsafe impl<T: Sync> Sync for UnsafeLinkedList<T> {}

impl<T> NodeHandle<T> {
    fn clone(&self) -> Self {
        NodeHandle { ptr: self.ptr }
    }
}

unsafe impl<T: Send> Send for NodeHandle<T> {}
unsafe impl<T: Sync> Sync for NodeHandle<T> {}

impl<'a, T: 'a> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        self.node.map(|n| unsafe {
            let n = &*n.as_ptr();
            self.node = n.next;
            &n.elm
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drop() {
        use std::cell::Cell;
        use std::rc::Rc;

        #[derive(Clone, Default)]
        struct DropCount(Rc<Cell<usize>>);
        impl Drop for DropCount {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }

        let count = DropCount::default();

        let node = {
            let mut list = UnsafeLinkedList::new();

            list.push_front(count.clone());
            let middle = list.push_front(count.clone());
            list.push_front(count.clone());

            unsafe {
                list.remove_node(middle)
            }
        };

        assert_eq!(count.0.get(), 2);
        drop(node);
        assert_eq!(count.0.get(), 3);
    }
}
